//! One syntax-expansion session: fresh marks, run isolation and transactions.
use std::sync::Arc;

use ilang_host::syntax::{
    DefinitionAbort, DefinitionContext, DefinitionReply, ItemContext, ItemRequest,
};
use ilang_syntax::lisp::expansion::{MacroArgument, MacroDefinition};
use ilang_syntax::lisp::syntax::{Syntax, SyntaxError, flip_mark_all, validate_expansion};

use crate::prepare::MacroPosition;
use crate::run::{MacroProgram, RunOutcome};

/// One successful run's effects. They are visible only after expansion and
/// output validation both succeed; the driver commits them with item phase.
pub struct Transaction {
    pub notes: Vec<String>,
    pub requests: Vec<ItemRequest>,
    /// Filled by the item driver after the entire result validates.
    pub item_origin: Option<ItemOrigin>,
    /// Unique invocation mark, used to discard only a run invalidated by audit.
    pub mark: u64,
    pub location: Option<(ilang_package::SourceId, ilang_syntax::span::Span)>,
}
#[derive(Clone, Debug)]
pub struct ItemOrigin {
    pub source: u32,
    pub call: ilang_syntax::span::Span,
    pub path: Vec<u32>,
}

pub struct Expanded {
    pub syntax: Syntax,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug)]
pub enum Failure {
    Syntax(SyntaxError),
    Aborted(DefinitionAbort),
    Stopped(ilang_host::Stop),
    Host(anyhow::Error),
}

/// A definition callback returns a fresh, unmarked answer. The session marks
/// it in the same way as arguments before it crosses into the guest.
pub type DefinitionResolver =
    dyn Fn(&Syntax, &ilang_host::macro_budget::MacroBudget) -> DefinitionReply + Send + Sync;

#[derive(Default)]
pub struct Session {
    next_mark: u64,
    depth: u16,
}

impl Session {
    pub(crate) fn next_mark(&self) -> u64 {
        self.next_mark
    }
    /// Expand one call and recursively expand its result while this run still
    /// occupies an expansion-depth frame. A retried call obtains a new mark.
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &mut self,
        definition: &MacroDefinition,
        arguments: &[MacroArgument],
        item: Option<Syntax>,
        call: &Syntax,
        position: MacroPosition,
        program: &MacroProgram,
        export: &str,
        templates: Arc<[Syntax]>,
        resolver: Arc<DefinitionResolver>,
        expand_result: impl FnOnce(
            &mut Self,
            Syntax,
            &ilang_host::macro_budget::MacroBudget,
        ) -> Result<Expanded, Failure>,
    ) -> Result<Expanded, Failure> {
        if self.depth == 256 {
            return Err(Failure::Syntax(SyntaxError {
                message: "macro expansion exceeds depth 256".into(),
                span: call.span(),
            }));
        }
        let budget = ilang_host::macro_budget::MacroBudget::default();
        let arguments = definition
            .arguments(arguments, item, call)
            .map_err(Failure::Syntax)?;
        let mark = self.next_mark;
        self.next_mark = mark.checked_add(1).expect("macro mark space exhausted");
        for argument in &arguments {
            ilang_host::syntax::charge_tree(&budget, argument, true).map_err(Failure::Host)?;
        }
        let marked = flip_mark_all(&arguments, mark);
        let resolver_budget = budget.clone();
        let context = Arc::new(DefinitionContext::new(move |query| {
            match resolver(query, &resolver_budget) {
                DefinitionReply::Answer(answer) => {
                    if ilang_host::syntax::charge_tree(&resolver_budget, &answer, true).is_err() {
                        return DefinitionReply::CompileError {
                            message: "macro resource limit exceeded".into(),
                            at: query.clone(),
                        };
                    }
                    DefinitionReply::Answer(answer.flip_mark(mark))
                }
                other => other,
            }
        }));
        let items = (position == MacroPosition::Item)
            .then(|| Arc::new(ItemContext::new(marked.clone().into(), call.clone())));
        self.depth += 1;
        let result = (|| {
            let outcome = crate::run::invoke_with_items_budget(
                program,
                export,
                &marked,
                templates,
                context.clone(),
                items.clone(),
                budget.clone(),
            )
            .map_err(Failure::Host)?;
            let syntax = match outcome {
                RunOutcome::Returned(syntax) => syntax,
                RunOutcome::Aborted(abort) => return Err(Failure::Aborted(abort)),
                RunOutcome::Stopped(stop) => return Err(Failure::Stopped(stop)),
            };
            ilang_host::syntax::charge_tree(&budget, &syntax, true).map_err(Failure::Host)?;
            validate_expansion(&syntax, &marked).map_err(Failure::Syntax)?;
            let requests = items
                .as_ref()
                .map_or_else(Vec::new, |items| items.take_requests());
            for request in &requests {
                match request {
                    ItemRequest::Add {
                        collection,
                        target,
                        at,
                        ..
                    } => {
                        for tree in [collection, target, at] {
                            ilang_host::syntax::charge_tree(&budget, tree, true)
                                .map_err(Failure::Host)?;
                        }
                    }
                    ItemRequest::Expose { name } => {
                        ilang_host::syntax::charge_tree(&budget, name, true)
                            .map_err(Failure::Host)?;
                    }
                }
            }
            let (syntax, requests) =
                ilang_host::syntax::flip_output_and_requests(&syntax, &requests, mark);
            let notes = context.take_notes();
            let mut expanded = expand_result(self, syntax, &budget)?;
            expanded.transactions.insert(
                0,
                Transaction {
                    notes,
                    requests,
                    item_origin: None,
                    mark,
                    location: None,
                },
            );
            Ok(expanded)
        })();
        self.depth -= 1;
        result
    }
}
