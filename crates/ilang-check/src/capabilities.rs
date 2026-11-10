//! Keep compiler-owned host capabilities in compiler-managed instances.

use crate::hir::*;
use crate::{code, order};
use ilang_package::{Diagnostic, ModuleKind, PackageGraph};
use std::collections::{HashSet, VecDeque};

fn reaches_compiler(program: &Program, initial: impl IntoIterator<Item = DefId>) -> bool {
    let mut queue: VecDeque<_> = initial.into_iter().collect();
    let mut visited = HashSet::new();
    while let Some(id) = queue.pop_front() {
        if !visited.insert(id) {
            continue;
        }
        match &program.def(id).kind {
            DefKind::External { module, .. } if module.starts_with("i:compiler/") => return true,
            DefKind::Binding { binding, .. } => {
                let b = &program.module(id.module).bindings[*binding as usize].binding;
                queue.extend(
                    order::references(&b.pattern, &b.value)
                        .into_iter()
                        .map(|(id, _)| id),
                );
            }
            _ => {}
        }
    }
    false
}

pub fn check(program: &Program, graph: &PackageGraph, diags: &mut Vec<Diagnostic>) {
    let Some(root) = program.root else {
        return;
    };
    let mut roots = program.host_exports.clone();
    for m in &program.modules {
        if m.package == root {
            roots.extend(
                m.bindings
                    .iter()
                    .filter(|b| b.main)
                    .flat_map(|b| b.defs.iter().copied()),
            );
        }
    }
    if roots.is_empty() {
        return;
    }
    for &d in &roots {
        if reaches_compiler(program, [d]) {
            let m = program.module(d.module);
            diags.push(Diagnostic::error(
                code::COMPILER_CAPABILITY,
                Some(m.source),
                program.def(d).span,
                "an ordinary entry cannot reach a compiler-owned operation (§2.1)",
            ));
        }
    }
    let mut packages = HashSet::from([graph.library]);
    let mut stack = vec![root];
    while let Some(p) = stack.pop() {
        if packages.insert(p) {
            stack.extend(
                graph.packages[p]
                    .dependencies
                    .iter()
                    .filter_map(|(_, id)| *id),
            );
        }
    }
    for m in &program.modules {
        if !packages.contains(&m.package) || m.kind == ModuleKind::Test {
            continue;
        }
        for b in &m.bindings {
            let refs = order::references(&b.binding.pattern, &b.binding.value)
                .into_iter()
                .filter_map(|(d, inside)| (!inside).then_some(d));
            if reaches_compiler(program, refs) {
                diags.push(Diagnostic::error(code::COMPILER_CAPABILITY, Some(m.source), b.binding.span,
                    "an ordinary entry's initialization cannot reach a compiler-owned operation (§2.1)"));
            }
        }
    }
}
