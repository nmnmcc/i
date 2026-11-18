//! Compiler execution hook for outside-in expression and type expansion: the
//! hook of the body walker, the encoding of a call's arguments, and
//! `is_direct`.
use crate::{
    body_walk::{Hook, Local},
    call,
    prepare::{self, MacroPosition},
    registry::{Macro, Registry},
    run::{self, MacroProgram},
    session::{DefinitionResolver, Expanded, Failure, Session, Transaction},
};
use ilang_check::{Parsed, hygiene};
use ilang_package::{PackageGraph, SourceId};
use ilang_syntax::ast::{DItem, Expr, ExprKind, Ident, Item, Pattern, Stmt, Type};
use ilang_syntax::lisp::{
    Datum,
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
                "a typed macro cannot make items; use a syntax macro"
            } else {
                "a typed macro is allowed only as an expression call; it cannot give a type"
            };
            return Err(Error::InvalidItem(invocation_span, message.into()));
        }
        if mac.source == self.source {
            return Err(Error::SameModule(invocation_span));
        }
        let (definition, target) =
            mac.definition
                .variant(&args, item.as_ref(), call)
                .map_err(|error| Error::Run {
                    failure: Failure::Syntax(error),
                    macro_name: mac.definition.name.clone(),
                    call: call.clone(),
                    invocation_span,
                })?;
        let phase = match position {
            MacroPosition::Item => 0,
            MacroPosition::Expression => 1,
            MacroPosition::Type => 2,
        };
        let key = (
            mac.source.0,
            format!("{}:{target}", mac.definition.name),
            phase,
        );
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
                .find(|b| {
                    b.module == module.path && b.name == mac.definition.name && b.target == target
                })
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
                &definition,
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

/// A node that a macro can take as an argument.
pub trait Encode {
    /// The node in the source rows, with the spans of
    /// `ilang_syntax::rows::*_datum`.
    fn datum(&self) -> Datum;
    fn span(&self) -> Span;
    fn normalize(&self, syntax: &Syntax) -> Syntax;
}
impl Encode for Expr {
    fn normalize(&self, syntax: &Syntax) -> Syntax {
        ilang_syntax::lisp::source_syntax::normalize_expression(syntax)
    }
    fn datum(&self) -> Datum {
        ilang_syntax::rows::expr_datum(self)
    }
    fn span(&self) -> Span {
        self.span.0
    }
}
impl Encode for Type {
    fn normalize(&self, syntax: &Syntax) -> Syntax {
        ilang_syntax::lisp::source_syntax::normalize_type(syntax)
    }
    fn datum(&self) -> Datum {
        ilang_syntax::rows::type_datum(self)
    }
    fn span(&self) -> Span {
        self.span.0
    }
}
impl Encode for Pattern {
    fn normalize(&self, syntax: &Syntax) -> Syntax {
        ilang_syntax::lisp::source_syntax::normalize_pattern(syntax)
    }
    fn datum(&self) -> Datum {
        ilang_syntax::rows::pattern_datum(self)
    }
    fn span(&self) -> Span {
        self.span.0
    }
}
impl Encode for Item {
    fn normalize(&self, syntax: &Syntax) -> Syntax {
        ilang_syntax::lisp::source_syntax::normalize_item(syntax)
    }
    fn datum(&self) -> Datum {
        ilang_syntax::rows::item_datum(self)
    }
    fn span(&self) -> Span {
        self.span.0
    }
}

/// The syntax value of a written node: the argument of a macro call. The
/// builder is a plain recursion over the node; the callers are on a deep
/// stack (`body_walk::expressions`).
pub fn encode_node(node: &impl Encode, scope: &Scope) -> Syntax {
    Syntax::from_datum(&node.datum(), scope)
}

/// Whether a value is made without running any code. `is_module` answers
/// whether a name is a module alias at this place.
///
/// The answers are those of `BodyEngine::is_direct` on the lowered tree:
/// every chain is a function (`Fn` or `Clauses`); data with a label or a
/// spread is a record and not direct; a call with a leading spread is a
/// record update; an interpolation is a chain of `<>`; a pipe into a chain
/// is a match or a pipe into a function.
pub fn is_direct(expression: &Expr, mut is_module: impl FnMut(&Ident) -> bool) -> bool {
    fn ungroup(mut expression: &Expr) -> &Expr {
        while let ExprKind::Block(block) = &expression.kind {
            let [Stmt::Expr(inner)] = block.stmts.as_slice() else {
                break;
            };
            expression = inner;
        }
        expression
    }
    fn constructor(expression: &Expr) -> bool {
        matches!(&expression.kind, ExprKind::Name { name, .. } if name.name.chars().next().is_some_and(char::is_uppercase))
    }
    fn values(args: &[DItem]) -> impl Iterator<Item = &Expr> {
        args.iter().map(|arg| call::argument(arg).1)
    }
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        match &ungroup(expression).kind {
            ExprKind::Chain(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name { .. } => {}
            ExprKind::Unary { expr, .. }
                if matches!(
                    ungroup(expr).kind,
                    ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_)
                ) => {}
            ExprKind::Field { target, .. } => match &target.kind {
                ExprKind::Name { module: None, name } if is_module(name) => {}
                _ => return false,
            },
            ExprKind::Data(items) if items.iter().all(|i| matches!(i, DItem::Pos(_))) => {
                pending.extend(values(items))
            }
            ExprKind::List { items, rest } => {
                pending.extend(items);
                pending.extend(rest.as_deref());
            }
            ExprKind::Call { callee, args } if constructor(callee) && !call::is_update(args) => {
                pending.extend(values(args))
            }
            ExprKind::Pipe { lhs, rhs } => {
                pending.push(lhs);
                match &rhs.kind {
                    ExprKind::Name { .. } if constructor(rhs) => {}
                    ExprKind::Call { callee, args }
                        if constructor(callee) && !call::is_update(args) =>
                    {
                        pending.extend(values(args))
                    }
                    _ => return false,
                }
            }
            _ => return false,
        }
    }
    true
}

/// The arguments of an expression macro call: the left side of a pipe, then
/// the written items.
pub fn expression_arguments(
    written: &call::ExprCall<'_>,
    mut encode: impl FnMut(&Expr) -> Syntax,
) -> Vec<MacroArgument> {
    let mut args = Vec::new();
    if let Some(prefix) = written.prefix {
        args.push(MacroArgument {
            label: None,
            syntax: encode(prefix),
        });
    }
    args.extend(item_arguments(written.args, encode));
    args
}

/// The written items of a call (an expression call, an item call, an
/// attribute) as macro arguments.
pub fn item_arguments(
    items: &[DItem],
    mut encode: impl FnMut(&Expr) -> Syntax,
) -> Vec<MacroArgument> {
    items
        .iter()
        .map(|item| {
            let (label, value) = call::argument(item);
            MacroArgument {
                label: label.map(|label| label.name.clone()),
                syntax: encode(value),
            }
        })
        .collect()
}

impl BodyEngine<'_> {
    pub(crate) fn atom(&self, id: &Ident) -> Syntax {
        if let Some(origin) = self.scoped.origin(id.span.0) {
            return origin.syntax.clone();
        }
        Syntax::from_datum(
            &Datum {
                kind: ilang_syntax::lisp::DatumKind::Name(id.name.clone()),
                span: id.span.0,
                docs: Vec::new(),
            },
            &self.scope(),
        )
    }
    pub(crate) fn resolve(
        &self,
        module: Option<&Ident>,
        name: &Ident,
        locals: &[Local],
        depth: u32,
    ) -> Option<Macro> {
        let module = module.map(|m| self.atom(m));
        self.registry.resolve(
            self.source,
            module.as_ref(),
            &self.atom(name),
            &self.hygiene_locals(locals, depth),
        )
    }
    /// The syntax of a node: what a macro made it from, or the written node
    /// encoded.
    pub(crate) fn node_syntax(&self, node: &impl Encode) -> Syntax {
        if let Some(origin) = self.scoped.origin(node.span()) {
            return origin.syntax.clone();
        }
        let file = self.graph.sources.get(self.source);
        let syntax = if file.display.ends_with(".is") {
            ilang_syntax::lisp::source_syntax::written_lisp_syntax(
                node.span(),
                &self.scope(),
                &file.text,
            )
        } else {
            self.source_encoder()
                .with_tokens(|| encode_node(node, &self.scope()))
        };
        node.normalize(&syntax)
    }
}

impl Hook for BodyEngine<'_> {
    type Error = Error;
    fn is_macro(&mut self, expression: &Expr, locals: &[Local], depth: u32) -> Result<bool, Error> {
        Ok(call::expression(expression).is_some_and(|call| {
            self.resolve(call.module, call.name, locals, depth)
                .is_some_and(|mac| mac.definition.signature.is_none())
        }))
    }
    fn expand(&mut self, expression: &Expr, locals: &[Local], depth: u32) -> Result<Expr, Error> {
        let written = call::expression(expression).expect("body walker identified a call");
        let mac = self
            .resolve(written.module, written.name, locals, depth)
            .expect("body walker resolved a macro");
        let args = expression_arguments(&written, |e| self.node_syntax(e));
        let call = self.node_syntax(expression);
        let invocation_span = expression.span.0;
        let (result, budget, transactions) = self.execute(
            &mac,
            args,
            &call,
            invocation_span,
            None,
            MacroPosition::Expression,
        )?;
        let start = u32::try_from(self.scoped.text().len() + 1).expect("source span overflow");
        let expression = self
            .scoped
            .expression_metered(&result, &call, |work, words| {
                budget.charge(work, words).map_err(|e| Box::new(e) as _)
            })
            .map_err(|error| metered_error(error, &mac.definition.name, &call, invocation_span))?;
        let end = u32::try_from(self.scoped.text().len()).expect("source span overflow");
        self.scoped.inherit_call_chain(
            Span::new(start, end),
            invocation_span,
            &mac.definition.name,
        );
        self.transactions.extend(transactions);
        Ok(expression)
    }
    fn is_type_macro(&mut self, ty: &Type, locals: &[Local], depth: u32) -> Result<bool, Error> {
        Ok(call::ty(ty).is_some_and(|call| {
            self.resolve(call.module, call.name, locals, depth)
                .is_some()
        }))
    }
    fn expand_type(&mut self, ty: &Type, locals: &[Local], depth: u32) -> Result<Type, Error> {
        let written = call::ty(ty).expect("body walker identified a type call");
        let mac = self
            .resolve(written.module, written.name, locals, depth)
            .expect("body walker resolved a macro");
        let args = written
            .args
            .iter()
            .map(|arg| MacroArgument {
                label: None,
                syntax: self.node_syntax(arg),
            })
            .collect();
        let old_span = ty.span.0;
        let call = self.node_syntax(ty);
        let (result, budget, transactions) =
            self.execute(&mac, args, &call, old_span, None, MacroPosition::Type)?;
        let start = u32::try_from(self.scoped.text().len() + 1).expect("source span overflow");
        let ty = self
            .scoped
            .ty_metered(&result, &call, |work, words| {
                budget.charge(work, words).map_err(|e| Box::new(e) as _)
            })
            .map_err(|error| metered_error(error, &mac.definition.name, &call, old_span))?;
        let end = u32::try_from(self.scoped.text().len()).expect("source span overflow");
        self.scoped
            .inherit_call_chain(Span::new(start, end), old_span, &mac.definition.name);
        self.type_replacements.push((old_span, ty.span.0, result));
        self.transactions.extend(transactions);
        self.item_replacements.push((old_span, ty.span.0, budget));
        Ok(ty)
    }
    fn depth_error(&mut self, span: Span) -> Error {
        Error::Depth(span)
    }
    fn is_direct(&mut self, expression: &Expr, locals: &[Local], depth: u32) -> bool {
        is_direct(expression, |name| {
            self.registry.is_module(
                self.source,
                &self.atom(name),
                &self.hygiene_locals(locals, depth),
            )
        })
    }
}
