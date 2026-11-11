//! Spec §2.4.1: count references in every top-level binding.
//! This pass changes no HIR.

use crate::code;
use crate::hir::*;
use ilang_package::{Diagnostic, ModuleKind};
use std::collections::HashSet;

/// The entry policy of one package, independent of the package being built.
fn is_package_entry(p: &Program, d: DefId) -> bool {
    match p.def(d).kind {
        DefKind::Binding { binding, .. } => p.module(d.module).bindings[binding as usize].main,
        DefKind::External { .. } => false,
    }
}

pub(crate) fn check(p: &Program, test_shaped: &HashSet<DefId>) -> Vec<Diagnostic> {
    let mut references = References { p, pending: vec![] };
    for (mi, m) in p.modules.iter().enumerate() {
        let module = ModuleId(mi as u32);
        for (di, d) in m.defs.iter().enumerate() {
            let id = DefId {
                module,
                index: di as u32,
            };
            match &d.kind {
                DefKind::Binding { binding, .. } => {
                    let b = &m.bindings[*binding as usize];
                    if (d.exported && m.kind != ModuleKind::Test)
                        || is_package_entry(p, id)
                        || b.test
                    {
                        references.pending.push(id);
                    }
                }
                DefKind::External { ty, .. } => references.ty(ty),
            }
        }
        for b in &m.bindings {
            references.binding(&b.binding);
        }
    }
    references.pending.extend(&p.host_exports);
    // Exempt definitions still use values through written types (§4.3).
    for t in &p.types {
        match &t.body {
            TypeBody::Alias(t) => references.ty(t),
            TypeBody::Custom(ctors) => {
                for f in ctors.iter().flat_map(|c| &c.fields) {
                    references.ty(&f.ty);
                }
            }
        }
    }
    let used: HashSet<_> = references.pending.into_iter().collect();
    let mut diags = Vec::new();
    for (mi, m) in p.modules.iter().enumerate() {
        for (di, d) in m.defs.iter().enumerate() {
            let id = DefId {
                module: ModuleId(mi as u32),
                index: di as u32,
            };
            if !matches!(d.kind, DefKind::Binding { .. })
                || d.name.starts_with('_')
                || used.contains(&id)
            {
                continue;
            }
            let mut message = format!("top-level binding `{}` is not used", d.name);
            if m.kind == ModuleKind::Test && test_shaped.contains(&id) {
                message.push_str("; did you forget `@test`?");
            }
            diags.push(Diagnostic::warning(
                code::UNUSED_BINDING,
                Some(m.source),
                d.span,
                message,
            ));
        }
    }
    diags
}

struct References<'a> {
    p: &'a Program,
    pending: Vec<DefId>,
}

impl References<'_> {
    fn ty(&mut self, t: &Type) {
        let mut sources = Vec::new();
        crate::order::type_sources(self.p, t, &mut Default::default(), &mut sources);
        for (_, value) in sources {
            if let ValueRef::Def(d) = value {
                self.pending.push(d);
            }
        }
    }

    fn binding(&mut self, b: &Binding) {
        binding_types(b, &mut |t| self.ty(t));
        self.pattern(&b.pattern);
        self.expr(&b.value);
    }

    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            match s {
                Stmt::Bind(b) => {
                    self.pattern(&b.pattern);
                    self.expr(&b.value);
                }
                Stmt::Env { value, .. } | Stmt::Expr(value) => self.expr(value),
            }
        }
    }

    fn format(&mut self, f: Option<&Format>) {
        if let Some(arg) = f.and_then(|f| f.arg.as_ref()) {
            self.expr(arg);
        }
    }

    fn pattern(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Named { pattern, .. } => self.pattern(pattern),
            PatternKind::Constructor { args, .. }
            | PatternKind::Tuple(args)
            | PatternKind::Error(args) => args.iter().for_each(|p| self.pattern(p)),
            PatternKind::List { items, .. } => items.iter().for_each(|p| self.pattern(p)),
            PatternKind::BitArray(segs) => {
                for s in segs {
                    self.pattern(&s.value);
                    self.format(s.format.as_ref());
                }
            }
            _ => {}
        }
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Name(ValueRef::Def(d)) => self.pending.push(*d),
            ExprKind::Tuple(items) => items.iter().for_each(|e| self.expr(e)),
            ExprKind::Record { base, fields } => {
                if let Some(base) = base {
                    self.expr(base);
                }
                fields.iter().for_each(|(_, value)| self.expr(value));
            }
            ExprKind::List { items, rest } => {
                items
                    .iter()
                    .chain(rest.as_deref())
                    .for_each(|e| self.expr(e));
            }
            ExprKind::BitArray(segs) => {
                for s in segs {
                    self.expr(&s.value);
                    self.format(s.format.as_ref());
                }
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Fn(f) => {
                for p in &f.params {
                    if let Param::Pattern { pattern, .. } = p {
                        self.pattern(pattern);
                    }
                }
                self.block(&f.body);
            }
            ExprKind::Case { subject, clauses } => {
                self.expr(subject);
                for c in clauses {
                    c.patterns.iter().for_each(|p| self.pattern(p));
                    c.guard.iter().for_each(|g| self.expr(g));
                    self.expr(&c.body);
                }
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
                self.expr(target)
            }
            ExprKind::RecordUpdate { base, fields, .. } => {
                self.expr(base);
                fields.iter().for_each(|(_, e)| self.expr(e));
            }
            ExprKind::Unary { expr, .. } => self.expr(expr),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Name(_)
            | ExprKind::Error => {}
        }
    }
}
