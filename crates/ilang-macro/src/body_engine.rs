//! Compiler execution hook for outside-in expression and type expansion.
use crate::{
    body_walk3::Local,
    prepare::{self, MacroPosition},
    registry::{Macro, Registry},
    run::{self, MacroProgram},
    session::{DefinitionResolver, Expanded, Failure, Session, Transaction},
};
use ilang_check::{Parsed, hygiene};
use ilang_package::{PackageGraph, SourceId};
use ilang_syntax::lisp::{
    expansion::MacroArgument,
    scoped::{MeteredError, ScopedError, ScopedSource},
    syntax::{Scope, Syntax},
};
use ilang_syntax::span::Span;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

#[derive(Debug)]
pub enum Error {
    Compile(Vec<ilang_package::Diagnostic>),
    Preparation(Vec<ilang_package::Diagnostic>),
    Run {
        failure: Failure,
        macro_name: String,
        call: Syntax,
        invocation_span: Span,
    },
    Conversion(ScopedError),
    Resource {
        error: Box<dyn std::error::Error + Send + Sync>,
        macro_name: String,
        call: Syntax,
        invocation_span: Span,
    },
    SameModule(Span),
    Depth(Span),
    InvalidItem(Span, String),
}
pub(crate) struct Compiled {
    program: MacroProgram,
    templates: Arc<[Syntax]>,
    export: String,
}

pub struct BodyEngine<'a> {
    pub graph: &'a PackageGraph,
    pub parsed: &'a [Parsed],
    pub registry: &'a Registry,
    pub source: SourceId,
    pub scoped: ScopedSource,
    failure_scoped: Option<ScopedSource>,
    pub(crate) phase_notes: Vec<crate::item_phase::PhaseNote>,
    pub resolver: Arc<DefinitionResolver>,
    pub transactions: Vec<Transaction>,
    /// Successful type replacements, with unique virtual occurrence spans.
    pub type_replacements: Vec<(Span, Span, Syntax)>,
    pub(crate) item_locals: Vec<Local>,
    pub(crate) item_overrides: HashMap<Span, (Syntax, Span)>,
    pub(crate) item_replacements: Vec<(Span, Span, ilang_host::macro_budget::MacroBudget)>,
    pub(crate) run_mark: Arc<std::sync::atomic::AtomicU64>,
    session: &'a mut Session,
    source_encoder: std::sync::OnceLock<ilang_syntax::lisp::source_tree::SourceTree>,
}
impl<'a> BodyEngine<'a> {
    pub fn new(
        graph: &'a PackageGraph,
        parsed: &'a [Parsed],
        registry: &'a Registry,
        source: SourceId,
        scoped: ScopedSource,
        resolver: Arc<DefinitionResolver>,
        session: &'a mut Session,
    ) -> Self {
        Self {
            graph,
            parsed,
            registry,
            source,
            scoped,
            failure_scoped: None,
            phase_notes: Vec::new(),
            resolver,
            transactions: Vec::new(),
            type_replacements: Vec::new(),
            item_locals: Vec::new(),
            item_overrides: HashMap::new(),
            item_replacements: Vec::new(),
            run_mark: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            session,
            source_encoder: std::sync::OnceLock::new(),
        }
    }
    /// Virtual origins produced by the most recent failed item phase.
    pub fn failed_notes(&self) -> &[crate::item_phase::PhaseNote] {
        &self.phase_notes
    }
    pub fn failure_scoped(&self) -> Option<&ScopedSource> {
        self.failure_scoped.as_ref()
    }
    pub(crate) fn save_failure_source(&mut self) {
        self.failure_scoped = Some(self.scoped.clone());
    }
    pub(crate) fn clear_failure_source(&mut self) {
        self.failure_scoped = None;
    }
    pub(crate) fn scope(&self) -> Scope {
        Scope {
            module: self.source.0,
            marks: BTreeSet::new(),
        }
    }

    fn locals(&self, locals: &[Local], depth: u32) -> Vec<hygiene::Binding<()>> {
        locals
            .iter()
            .filter(|local| local.fn_only.is_none_or(|level| depth > level))
            .map(|local| {
                let origin = self.scoped.origin(local.span).map(|origin| &origin.syntax);
                hygiene::Binding {
                    text: local.name.clone(),
                    scope: origin.map_or_else(|| self.scope(), |s| s.scope().clone()),
                    identity: origin.and_then(|s| s.scope_identity().cloned()),
                    target: (),
                }
            })
            .collect()
    }
    pub(crate) fn hygiene_locals(&self, locals: &[Local], depth: u32) -> Vec<hygiene::Binding<()>> {
        self.locals(locals, depth)
    }

    pub(crate) fn source_encoder(&self) -> &ilang_syntax::lisp::source_tree::SourceTree {
        self.source_encoder.get_or_init(|| {
            let file = self.graph.sources.get(self.source);
            ilang_syntax::lisp::source_tree::SourceTree::read(
                &self.scope(),
                &file.text,
                file.display.ends_with(".is"),
            )
        })
    }

    pub(crate) fn execute(
        &mut self,
        mac: &Macro,
        args: Vec<MacroArgument>,
        call: &Syntax,
        invocation_span: Span,
        item: Option<Syntax>,
        position: MacroPosition,
    ) -> Result<
        (
            Syntax,
            ilang_host::macro_budget::MacroBudget,
            Vec<Transaction>,
        ),
        Error,
    > {
        if mac.definition.signature.is_some() && position != MacroPosition::Expression {
            let message = if position == MacroPosition::Item {
                "a typed macro cannot make items; use a syntax macro (§15.7.5)"
            } else {
                "a typed macro is allowed only as an expression call; it cannot give a type"
            };
            return Err(Error::InvalidItem(invocation_span, message.into()));
        }
        if mac.source == self.source {
            return Err(Error::SameModule(invocation_span));
        }
        let phase = match position {
            MacroPosition::Item => 0,
            MacroPosition::Expression => 1,
            MacroPosition::Type => 2,
        };
        let key = (mac.source.0, mac.definition.name.clone(), phase);
        if !self.session.compiled.contains_key(&key) {
            let mut graph = self.graph.clone();
            let mut parsed = prepare::import_closure(&graph, self.parsed, mac.source)
                .map_err(Error::Preparation)?;
            let prepared = prepare::prepare(&graph, &mut parsed).map_err(Error::Preparation)?;
            let module = self
                .parsed
                .iter()
                .find(|p| p.source == mac.source)
                .expect("macro module must exist");
            let binding = prepared
                .bindings
                .iter()
                .find(|b| b.module == module.path && b.name == mac.definition.name)
                .expect("registered macro must be prepared");
            let (path, name) = prepare::add_wrapper(&mut graph, &mut parsed, binding, position);
            let wrapper_source = parsed.last().expect("wrapper module was added").source;
            let definition_span = module.ast.items[mac.definition.item].span.0;
            let program =
                run::compile_owned(graph, &parsed, &[(&path, &name)]).map_err(|error| {
                    let mut diagnostics = match error {
                        run::CompileError::Program(diagnostics) => diagnostics,
                        run::CompileError::Internal(message) => {
                            vec![ilang_package::Diagnostic::error(
                                ilang_check::code::MACRO_RESERVED,
                                Some(mac.source),
                                definition_span,
                                format!(
                                    "compiler error while compiling macro `{}`: {message}",
                                    mac.definition.name
                                ),
                            )]
                        }
                    };
                    for diagnostic in &mut diagnostics {
                        if diagnostic.source == Some(wrapper_source) {
                            diagnostic.source = Some(mac.source);
                            diagnostic.span = definition_span;
                            diagnostic.message =
                                format!("macro `{}`: {}", mac.definition.name, diagnostic.message);
                        }
                    }
                    Error::Compile(diagnostics)
                })?;
            self.session.compiled.insert(
                key.clone(),
                Arc::new(Compiled {
                    program,
                    templates: prepared.templates.into(),
                    export: format!("{path}.{name}"),
                }),
            );
        }
        let compiled = self.session.compiled[&key].clone();
        self.run_mark.store(
            self.session.next_mark(),
            std::sync::atomic::Ordering::Relaxed,
        );
        let mut output_budget = None;
        let result = self
            .session
            .run(
                &mac.definition,
                &args,
                item,
                call,
                position,
                &compiled.program,
                &compiled.export,
                compiled.templates.clone(),
                self.resolver.clone(),
                |_, syntax, budget| {
                    output_budget = Some(budget.clone());
                    Ok(Expanded {
                        syntax,
                        transactions: Vec::new(),
                    })
                },
            )
            .map_err(|failure| Error::Run {
                failure,
                macro_name: mac.definition.name.clone(),
                call: call.clone(),
                invocation_span,
            })?;
        let mut transactions = result.transactions;
        for transaction in &mut transactions {
            transaction.location = Some((self.source, invocation_span));
        }
        Ok((
            result.syntax,
            output_budget.expect("successful run supplies its budget"),
            transactions,
        ))
    }
}


pub(crate) fn metered_error(
    error: MeteredError,
    macro_name: &str,
    call: &Syntax,
    invocation_span: Span,
) -> Error {
    match error {
        MeteredError::Syntax(error) => Error::Conversion(error),
        MeteredError::Resource(error) => Error::Resource {
            error,
            macro_name: macro_name.to_owned(),
            call: call.clone(),
            invocation_span,
        },
    }
}
