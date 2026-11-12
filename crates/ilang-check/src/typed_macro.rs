//! Typed expansion runs at the inference group barrier, through this compiler hook.
use crate::hir::{DefId, Expr, ModuleId, Program};
use ilang_package::Diagnostic;
use ilang_syntax::{ast, lisp::scoped::ScopedSource, span::Span};
use std::sync::Arc;

pub trait Expander: std::fmt::Debug + Send + Sync {
    fn expand(
        &self,
        program: &Program,
        module: ModuleId,
        span: Span,
        definition: DefId,
        query: Arc<crate::types::TypedMacroQuery>,
        var_count: u32,
        ty_var_count: u32,
    ) -> Result<Output, Vec<Diagnostic>>;
}
#[derive(Clone, Debug)]
pub struct Output {
    pub expression: Expr,
    pub syntax: ast::Expr,
    pub scoped: ScopedSource,
    pub var_count: u32,
    pub ty_var_count: u32,
    /// Virtual output spans to checked input occurrences.
    pub holes: Vec<(Span, Span)>,
    pub trace: Vec<String>,
    pub notes: Vec<String>,
}

/// Install checked output without re-inferring the caller or its schemes.
pub fn apply(program: &mut Program, typed: &crate::types::Typed) {
    for module_index in 0..program.modules.len() {
        let module = ModuleId(module_index as u32);
        let mut originals = Vec::new();
        for b in &mut program.modules[module_index].bindings {
            crate::hir::visit_expr_mut(&mut b.binding.value, &mut |e| {
                let original=e.clone();let mut changed=false;
                while let Some(output)=typed.typed_expansions.get(&(module,e.span)) {
                    *e=output.expression.clone();changed=true;
                }
                if changed {originals.push(((module,e.span),original));}
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
}
