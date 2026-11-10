//! External dependencies of a compile-time macro program.
use crate::hir::{DefId, DefKind, ModuleId, Program};
use crate::order;
use ilang_package::PackageId;
use std::collections::{BTreeSet, VecDeque};

/// External declarations from outside `i` reachable from the macro entries or
/// from non-direct initializers in the compiled macro modules (§8). `modules`
/// is the compiler-selected import closure for this macro program.
pub fn foreign_externals(
    program: &Program,
    library: PackageId,
    modules: &[ModuleId],
    entries: &[DefId],
) -> Vec<DefId> {
    let mut queue: VecDeque<_> = entries.iter().copied().collect();
    for &module in modules {
        for binding in &program.module(module).bindings {
            if !binding.binding.direct {
                queue.extend(
                    order::references(&binding.binding.pattern, &binding.binding.value)
                        .into_iter()
                        .map(|(id, _)| id),
                );
            }
        }
    }
    let mut visited = BTreeSet::new();
    let mut foreign = BTreeSet::new();
    while let Some(id) = queue.pop_front() {
        if !visited.insert(id) {
            continue;
        }
        match &program.def(id).kind {
            DefKind::External { .. } => {
                if program.module(id.module).package != library {
                    foreign.insert(id);
                }
            }
            DefKind::Binding { binding, .. } => {
                let binding = &program.module(id.module).bindings[*binding as usize].binding;
                queue.extend(
                    order::references(&binding.pattern, &binding.value)
                        .into_iter()
                        .map(|(id, _)| id),
                );
            }
        }
    }
    foreign.into_iter().collect()
}
