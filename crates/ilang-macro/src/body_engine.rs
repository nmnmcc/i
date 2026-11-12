//! Compiler execution hook for outside-in expression and type expansion.
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
use ilang_syntax::ast::{self, Expr, ExprKind, Ident, Stmt, TypeExpr};
use ilang_syntax::lisp::{
    Datum, DatumKind,
    expansion::MacroArgument,
    scoped::{MeteredError, ScopedError, ScopedSource},
    source_syntax::{source_expression_syntax, source_type_syntax},
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
    source_encoder: std::sync::OnceLock<ilang_syntax::lisp::SourceEncoder>,
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
    pub(crate) fn atom(&self, id: &Ident) -> Syntax {
        if let Some(origin) = self.scoped.origin(id.span) {
            return origin.syntax.clone();
        }
        Syntax::from_datum(
            &Datum {
                kind: DatumKind::Name(id.name.clone()),
                span: id.span,
                docs: Vec::new(),
            },
            &self.scope(),
        )
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
            &self.locals(locals, depth),
        )
    }
    pub(crate) fn source_encoder(&self) -> &ilang_syntax::lisp::SourceEncoder {
        self.source_encoder.get_or_init(|| {
            ilang_syntax::lisp::SourceEncoder::new(
                &self.scope(),
                &self.graph.sources.get(self.source).text,
            )
        })
    }
    pub(crate) fn expr_syntax(&self, e: &Expr) -> Syntax {
        if let Some(origin) = self.scoped.origin(e.span) {
            return origin.syntax.clone();
        }
        let file = self.graph.sources.get(self.source);
        if file.display.ends_with(".is") {
            source_expression_syntax(e, &self.scope(), &file.text, true)
        } else {
            self.source_encoder().expression(e)
        }
    }
    pub(crate) fn ty_syntax(&self, ty: &TypeExpr) -> Syntax {
        if let Some(origin) = self.scoped.origin(ty.span) {
            return origin.syntax.clone();
        }
        let file = self.graph.sources.get(self.source);
        if file.display.ends_with(".is") {
            source_type_syntax(ty, &self.scope(), &file.text, true)
        } else {
            self.source_encoder().ty(ty)
        }
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
            return Err(Error::InvalidItem(invocation_span,"a typed macro is allowed only as an expression call; it cannot make items or types".into()));
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
            let definition_span = module.ast.items[mac.definition.item].span;
            let program = run::compile(&graph, &parsed, &[(&path, &name)]).map_err(|error| {
                let mut diagnostics = match error {
                    run::CompileError::Program(diagnostics) => diagnostics,
                    run::CompileError::Internal(message) => vec![ilang_package::Diagnostic::error(
                        ilang_check::code::MACRO_RESERVED,
                        Some(mac.source),
                        definition_span,
                        format!(
                            "compiler error while compiling macro `{}`: {message}",
                            mac.definition.name
                        ),
                    )],
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
        let mut args = Vec::new();
        if let Some(prefix) = written.prefix {
            args.push(MacroArgument {
                label: None,
                syntax: self.expr_syntax(prefix),
            });
        }
        args.extend(written.args.iter().map(|arg| MacroArgument {
            label: arg.label.as_ref().map(|label| label.name.clone()),
            syntax: self.expr_syntax(&arg.value),
        }));
        let call = self.expr_syntax(expression);
        let (result, budget, transactions) = self.execute(
            &mac,
            args,
            &call,
            expression.span,
            None,
            MacroPosition::Expression,
        )?;
        let invocation_span = expression.span;
        let start = u32::try_from(self.scoped.text().len() + 1).expect("source span overflow");
        let expression = self
            .scoped
            .expression_metered(&result, &call, |work, words| {
                budget.charge(work, words).map_err(|e| Box::new(e) as _)
            })
            .map_err(|error| metered_error(error, &mac.definition.name, &call, expression.span))?;
        let end = u32::try_from(self.scoped.text().len()).expect("source span overflow");
        self.scoped.inherit_call_chain(
            Span::new(start, end),
            invocation_span,
            &mac.definition.name,
        );
        self.transactions.extend(transactions);
        Ok(expression)
    }
    fn is_type_macro(
        &mut self,
        ty: &TypeExpr,
        locals: &[Local],
        depth: u32,
    ) -> Result<bool, Error> {
        Ok(call::ty(ty).is_some_and(|call| {
            self.resolve(call.module, call.name, locals, depth)
                .is_some()
        }))
    }
    fn expand_type(
        &mut self,
        ty: &TypeExpr,
        locals: &[Local],
        depth: u32,
    ) -> Result<TypeExpr, Error> {
        let written = call::ty(ty).expect("body walker identified a type call");
        let mac = self
            .resolve(written.module, written.name, locals, depth)
            .expect("body walker resolved a macro");
        let args = written
            .args
            .iter()
            .map(|arg| MacroArgument {
                label: None,
                syntax: self.ty_syntax(arg),
            })
            .collect();
        let old_span = ty.span;
        let call = self.ty_syntax(ty);
        let (result, budget, transactions) =
            self.execute(&mac, args, &call, ty.span, None, MacroPosition::Type)?;
        let start = u32::try_from(self.scoped.text().len() + 1).expect("source span overflow");
        let ty = self
            .scoped
            .ty_metered(&result, &call, |work, words| {
                budget.charge(work, words).map_err(|e| Box::new(e) as _)
            })
            .map_err(|error| metered_error(error, &mac.definition.name, &call, ty.span))?;
        let end = u32::try_from(self.scoped.text().len()).expect("source span overflow");
        self.scoped
            .inherit_call_chain(Span::new(start, end), old_span, &mac.definition.name);
        self.type_replacements.push((old_span, ty.span, result));
        self.transactions.extend(transactions);
        self.item_replacements.push((old_span, ty.span, budget));
        Ok(ty)
    }
    fn depth_error(&mut self, span: Span) -> Error {
        Error::Depth(span)
    }
    fn is_direct(&mut self, expression: &Expr, locals: &[Local], depth: u32) -> bool {
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
        let mut pending = vec![expression];
        while let Some(expression) = pending.pop() {
            match &ungroup(expression).kind {
                ExprKind::Fn(_)
                | ExprKind::Clauses(_)
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
                    ExprKind::Name { module: None, name }
                        if self.registry.is_module(
                            self.source,
                            &self.atom(name),
                            &self.locals(locals, depth),
                        ) => {}
                    _ => return false,
                },
                ExprKind::Tuple(items) => pending.extend(items),
                ExprKind::List { items, rest } => {
                    pending.extend(items);
                    pending.extend(rest.as_deref());
                }
                ExprKind::Call { callee, args } if constructor(callee) => {
                    pending.extend(args.iter().map(|arg| &arg.value))
                }
                ExprKind::Binary {
                    op: ast::BinOp::Pipe,
                    lhs,
                    rhs,
                } => {
                    pending.push(lhs);
                    match &rhs.kind {
                        ExprKind::Name { .. } if constructor(rhs) => {}
                        ExprKind::Call { callee, args } if constructor(callee) => {
                            pending.extend(args.iter().map(|arg| &arg.value))
                        }
                        _ => return false,
                    }
                }
                _ => return false,
            }
        }
        true
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
