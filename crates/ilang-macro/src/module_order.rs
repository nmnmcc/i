//! Deterministic dependency order for expansion of parsed modules.
use ilang_check::Parsed;
use ilang_package::{ModuleKind, PackageGraph, SourceId};
use std::collections::{HashMap, VecDeque};

/// Dependencies precede importers. An import cycle is left to the ordinary
/// resolver, which owns its source-located diagnostic. Missing imports are
/// similarly left to the resolver; they are not invented as graph vertices.
pub fn order(graph: &PackageGraph, parsed: &[Parsed]) -> Result<Vec<SourceId>, ImportCycle> {
    let paths = parsed
        .iter()
        .enumerate()
        .filter(|(_, m)| m.kind != ModuleKind::Prelude)
        .map(|(index, module)| ((module.package, module.path.as_str()), index))
        .collect::<HashMap<_, _>>();
    let mut outgoing = vec![Vec::new(); parsed.len()];
    let mut remaining = vec![0usize; parsed.len()];
    for (index, module) in parsed.iter().enumerate() {
        let package = &graph.packages[module.package];
        for import in &module.ast.imports {
            let first = &import.path[0].name;
            let id = if first == "i" {
                Some(graph.library)
            } else if *first == package.name {
                Some(module.package)
            } else {
                package
                    .dependencies
                    .iter()
                    .find(|(name, _)| name == first)
                    .and_then(|(_, id)| *id)
                    .or_else(|| {
                        (module.kind == ModuleKind::Test)
                            .then(|| {
                                package
                                    .dev_dependencies
                                    .iter()
                                    .find(|(name, _)| name == first)
                                    .and_then(|(_, id)| *id)
                            })
                            .flatten()
                    })
            };
            let Some(id) = id else { continue };
            let rest = import
                .path
                .iter()
                .skip(1)
                .map(|part| part.name.as_str())
                .collect::<Vec<_>>()
                .join("/");
            let path = format!("{}/{rest}", graph.packages[id].name);
            let Some(&dependency) = paths.get(&(id, path.as_str())) else {
                continue;
            };
            if parsed[dependency].kind == ModuleKind::Test && module.kind != ModuleKind::Test {
                continue;
            }
            if !outgoing[dependency].contains(&index) {
                outgoing[dependency].push(index);
                remaining[index] += 1;
            }
        }
    }
    let mut ready = remaining
        .iter()
        .enumerate()
        .filter_map(|(i, count)| (*count == 0).then_some(i))
        .collect::<VecDeque<_>>();
    let mut result = Vec::with_capacity(parsed.len());
    while let Some(index) = ready.pop_front() {
        result.push(parsed[index].source);
        for &dependent in &outgoing[index] {
            remaining[dependent] -= 1;
            if remaining[dependent] == 0 {
                ready.push_back(dependent);
            }
        }
    }
    if result.len() == parsed.len() {
        Ok(result)
    } else {
        Err(ImportCycle)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportCycle;
