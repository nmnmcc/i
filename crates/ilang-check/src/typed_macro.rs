//! Typed expansion runs at the inference group barrier, through this compiler hook.
use crate::hir::{DefId, Expr, ModuleId, Program};
use ilang_package::Diagnostic;
use ilang_syntax::{lisp::scoped::ScopedSource, span::Span, syn3::ast};
use std::sync::Arc;

pub struct Invocation<'a> {
    pub program: &'a Program,
    pub module: ModuleId,
    pub span: Span,
    pub definition: DefId,
    pub var_count: u32,
    pub ty_var_count: u32,
}
pub trait Expander: std::fmt::Debug + Send + Sync {
    fn expand(
        &self,
        call: Invocation<'_>,
        query: Arc<crate::types::TypedMacroQuery>,
    ) -> Result<Output, Vec<Diagnostic>>;
}
#[derive(Debug)]
pub struct Output {
    pub expression: Expr,
    pub syntax: ast::Expr,
    pub scoped: ScopedSource,
    pub var_count: u32,
    pub ty_var_count: u32,
    pub resolution: Arc<Program>,
    /// Virtual output spans to checked input occurrences.
    pub holes: Vec<(Span, Span)>,
    pub trace: Vec<String>,
    pub notes: Vec<String>,
}

impl Clone for Output {
    fn clone(&self) -> Self {
        ilang_syntax::syn3::deep::deep(|| Self {
            expression: self.expression.clone(),
            syntax: self.syntax.clone(),
            scoped: self.scoped.clone(),
            var_count: self.var_count,
            ty_var_count: self.ty_var_count,
            resolution: self.resolution.clone(),
            holes: self.holes.clone(),
            trace: self.trace.clone(),
            notes: self.notes.clone(),
        })
    }
}

/// Install checked output without re-inferring the caller or its schemes.
pub fn apply(program: &mut Program, typed: &crate::types::Typed) {
    for module_index in 0..program.modules.len() {
        let module = ModuleId(module_index as u32);
        let mut originals = Vec::new();
        for b in &mut program.modules[module_index].bindings {
            crate::hir::visit_expr_mut(&mut b.binding.value, &mut |e| {
                let original = e.clone();
                let mut changed = false;
                while let Some(output) = typed.typed_expansions.get(&(module, e.span)) {
                    *e = output.expression.clone();
                    changed = true;
                }
                if changed {
                    originals.push(((module, e.span), original));
                }
                true
            });
        }
        program.typed_originals.extend(originals);
        if let Some(output) = typed
            .typed_expansions
            .iter()
            .filter(|((m, _), _)| *m == module)
            .map(|(_, o)| o)
            .max_by_key(|o| o.scoped.text().len())
        {
            program.modules[module_index].scoped = Some(output.scoped.clone());
        }
    }
    program.var_count = program.var_count.max(
        typed
            .typed_expansions
            .values()
            .map(|o| o.var_count)
            .max()
            .unwrap_or(0),
    );
    if let Some(output) = typed
        .typed_expansions
        .values()
        .max_by_key(|o| o.resolution.ty_var_names.len())
        && output.resolution.ty_var_names.len() > program.ty_var_names.len()
    {
        program.ty_var_names = output.resolution.ty_var_names.clone();
    }
    program.ty_var_count = program.ty_var_count.max(
        typed
            .typed_expansions
            .values()
            .map(|o| o.ty_var_count)
            .max()
            .unwrap_or(0),
    );
    if !typed.typed_expansions.is_empty() {
        crate::order::order(program, &mut Vec::new());
    }
}
pub fn restore(program: &mut Program) {
    let originals = std::mem::take(&mut program.typed_originals);
    let changed = !originals.is_empty();
    for (i, module) in program.modules.iter_mut().enumerate() {
        for b in &mut module.bindings {
            crate::hir::visit_expr_mut(&mut b.binding.value, &mut |e| {
                if let Some(original) = originals.get(&(ModuleId(i as u32), e.span)) {
                    *e = original.clone();
                    return false;
                }
                true
            });
        }
    }
    if changed {
        let mut diagnostics = Vec::new();
        crate::order::order(program, &mut diagnostics);
        assert!(
            !diagnostics.iter().any(Diagnostic::is_error),
            "restored typed caller lost its original dependency order"
        );
    }
}

impl Output {
    pub fn syntax3(&self) -> ilang_syntax::syn3::ast::Expr {
        ilang_syntax::syn3::deep::deep(|| self.syntax.clone())
    }
    pub fn print_syntax(&self, lisp: bool) -> String {
        ilang_syntax::syn3::print_tree(&ilang_syntax::syn3::expression_module(self.syntax3()), lisp)
    }
}
