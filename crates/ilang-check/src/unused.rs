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
                DefKind::Binding { .. } => {
                    if (d.exported && m.kind != ModuleKind::Test) || is_package_entry(p, id) {
                        references.pending.push(id);
                    }
                }
                DefKind::External { ty, .. } => references.ty(ty),
            }
        }
        for b in &m.bindings {
            // A chain name does not use its own shape groups: the name
            // and its groups are used together (below).
            let chain = b.defs.iter().any(|d| p.chain_groups.contains_key(d));
            if !chain {
                references.binding(&b.binding);
            }
        }
    }
    references.pending.extend(&p.chain_uses);
    references.pending.extend(&p.host_exports);
    references.pending.extend(&p.declared_entries);
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
    let mut used: HashSet<_> = references.pending.into_iter().collect();
    // A direct call of a chain is a call of one of its shape groups; a use
    // of any of them, or of the name, is a use of the whole chain.
    for (chain, groups) in &p.chain_groups {
        if used.contains(chain) || groups.iter().any(|(_, g)| used.contains(g)) {
            used.insert(*chain);
            used.extend(groups.iter().map(|(_, g)| *g));
        }
    }
    let mut diags = Vec::new();
    for (mi, m) in p.modules.iter().enumerate() {
        for (di, d) in m.defs.iter().enumerate() {
            let id = DefId {
                module: ModuleId(mi as u32),
                index: di as u32,
            };
            if !matches!(d.kind, DefKind::Binding { .. })
                || d.name.starts_with('_')
                || d.name.contains('%')
                || used.contains(&id)
            {
                continue;
            }
            let mut message = format!("top-level binding `{}` is not used", d.name);
            if m.kind == ModuleKind::Test && test_shaped.contains(&id) {
                message.push_str("; did you forget to register this function with `test`?");
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
        let references = crate::order::references(&b.pattern, &b.value);
        self.pending.extend(references.into_iter().map(|(d, _)| d));
    }
}
