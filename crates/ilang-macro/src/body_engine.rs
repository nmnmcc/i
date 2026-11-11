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
    scoped::{ScopedError, ScopedSource},
    source_syntax::{source_expression_syntax, source_type_syntax},
    syntax::{Scope, Syntax},
};
use ilang_syntax::span::Span;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

#[derive(Debug)]
pub enum Error {
    Compile(run::CompileError),
    Preparation(Vec<ilang_package::Diagnostic>),
    Run(Failure),
    Conversion(ScopedError),
    Metered(ilang_syntax::lisp::scoped::MeteredError),
    SameModule(Span),
    Depth(Span),
    InvalidItem(Span, String),
}
struct Compiled {
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
    pub resolver: Arc<DefinitionResolver>,
    pub transactions: Vec<Transaction>,
    cache: HashMap<(u32, String, u8), Compiled>,
    pub(crate) item_locals: Vec<Local>,
    pub(crate) run_mark: Arc<std::sync::atomic::AtomicU64>,
    session: &'a mut Session,
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
            resolver,
            transactions: Vec::new(),
            cache: HashMap::new(),
            item_locals: Vec::new(),
            run_mark: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            session,
        }
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
    pub(crate) fn expr_syntax(&self, e: &Expr) -> Syntax {
        if let Some(origin) = self.scoped.origin(e.span) {
            return origin.syntax.clone();
        }
        let file = self.graph.sources.get(self.source);
        source_expression_syntax(e, &self.scope(), &file.text, file.display.ends_with(".is"))
    }
    pub(crate) fn ty_syntax(&self, ty: &TypeExpr) -> Syntax {
        if let Some(origin) = self.scoped.origin(ty.span) {
            return origin.syntax.clone();
        }
        let file = self.graph.sources.get(self.source);
        source_type_syntax(ty, &self.scope(), &file.text, file.display.ends_with(".is"))
    }
    pub(crate) fn execute(
        &mut self,
        mac: &Macro,
        args: Vec<MacroArgument>,
        call: &Syntax,
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
        if mac.source == self.source {
            return Err(Error::SameModule(call.span()));
        }
        let phase = match position {
            MacroPosition::Item => 0,
            MacroPosition::Expression => 1,
            MacroPosition::Type => 2,
        };
        let key = (mac.source.0, mac.definition.name.clone(), phase);
        if !self.cache.contains_key(&key) {
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
            let program =
                run::compile(&graph, &parsed, &[(&path, &name)]).map_err(Error::Compile)?;
            self.cache.insert(
                key.clone(),
                Compiled {
                    program,
                    templates: prepared.templates.into(),
                    export: format!("{path}.{name}"),
                },
            );
        }
        let compiled = &self.cache[&key];
        self.run_mark.store(
            self.session.next_mark(),
            std::sync::atomic::Ordering::Relaxed,
        );
        let mut result_budget = None;
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
                    result_budget = Some(budget.clone());
                    Ok(Expanded {
                        syntax,
                        transactions: Vec::new(),
                    })
                },
            )
            .map_err(Error::Run)?;
        Ok((
            result.syntax,
            result_budget.expect("successful run retains its budget"),
            result.transactions,
        ))
    }
}
impl Hook for BodyEngine<'_> {
    type Error = Error;
    fn is_macro(&mut self, expression: &Expr, locals: &[Local], depth: u32) -> Result<bool, Error> {
        Ok(call::expression(expression).is_some_and(|call| {
            self.resolve(call.module, call.name, locals, depth)
                .is_some()
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
        let (result, budget, transactions) =
            self.execute(&mac, args, &call, None, MacroPosition::Expression)?;
        let output = self
            .scoped
            .expression_metered(&result, &call, |work, words| {
                budget.charge(work, words).map_err(|e| Box::new(e) as _)
            })
            .map_err(Error::Metered)?;
        self.transactions.extend(transactions);
        Ok(output)
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
        let call = self.ty_syntax(ty);
        let (result, budget, transactions) =
            self.execute(&mac, args, &call, None, MacroPosition::Type)?;
        let output = self
            .scoped
            .ty_metered(&result, &call, |work, words| {
                budget.charge(work, words).map_err(|e| Box::new(e) as _)
            })
            .map_err(Error::Metered)?;
        self.transactions.extend(transactions);
        Ok(output)
    }
    fn depth_error(&mut self, span: Span) -> Error {
        Error::Depth(span)
    }
    fn is_direct(&mut self, expression: &Expr, locals: &[Local], depth: u32) -> bool {
        fn ungroup(e: &Expr) -> &Expr {
            match &e.kind {
                ExprKind::Block(b) if b.stmts.len() == 1 => match &b.stmts[0] {
                    Stmt::Expr(inner) => ungroup(inner),
                    _ => e,
                },
                _ => e,
            }
        }
        fn constructor(e: &Expr) -> bool {
            matches!(&e.kind, ExprKind::Name { name, .. } if name.name.chars().next().is_some_and(char::is_uppercase))
        }
        let e = ungroup(expression);
        match &e.kind {
            ExprKind::Fn(_)
            | ExprKind::Clauses(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name { .. } => true,
            ExprKind::Unary { expr, .. } => matches!(
                ungroup(expr).kind,
                ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::String(_)
            ),
            ExprKind::Field { target, .. } => match &target.kind {
                ExprKind::Name { module: None, name } => self.registry.is_module(
                    self.source,
                    &self.atom(name),
                    &self.locals(locals, depth),
                ),
                _ => false,
            },
            ExprKind::Tuple(items) => items.iter().all(|e| self.is_direct(e, locals, depth)),
            ExprKind::List { items, rest } => items
                .iter()
                .chain(rest.as_deref())
                .all(|e| self.is_direct(e, locals, depth)),
            ExprKind::Call { callee, args } => {
                constructor(callee) && args.iter().all(|a| self.is_direct(&a.value, locals, depth))
            }
            ExprKind::Binary {
                op: ast::BinOp::Pipe,
                lhs,
                rhs,
            } => {
                self.is_direct(lhs, locals, depth)
                    && match &rhs.kind {
                        ExprKind::Name { .. } => constructor(rhs),
                        ExprKind::Call { callee, args } => {
                            constructor(callee)
                                && args.iter().all(|a| self.is_direct(&a.value, locals, depth))
                        }
                        _ => false,
                    }
            }
            _ => false,
        }
    }
}
