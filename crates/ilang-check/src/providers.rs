//! Name resolution for target provider plans (§2.6.1).
//!
//! A plan is resolved using the package which declared each cell, not the
//! package whose entry eventually uses it.

use crate::hir::{DefId, ModuleId, Program, TypeId};
use ilang_package::{ModuleKind, PackageGraph, PackageId};

/// The failure to resolve a provider-plan path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathError {
    InvalidPath,
    UnknownPackage,
    UnknownModule,
    UnknownName,
    Private,
}

fn module(
    graph: &PackageGraph,
    program: &Program,
    authority: PackageId,
    path: &str,
) -> Result<(ModuleId, String), PathError> {
    let (module, name) = path.rsplit_once('.').ok_or(PathError::InvalidPath)?;
    let (prefix, rest) = module.split_once('/').ok_or(PathError::InvalidPath)?;
    if name.is_empty() || rest.is_empty() {
        return Err(PathError::InvalidPath);
    }
    let package = &graph.packages[authority];
    let target = if prefix == "i" {
        graph.library
    } else if prefix == package.name {
        authority
    } else {
        package
            .dependencies
            .iter()
            .find(|(name, _)| name == prefix)
            .and_then(|(_, id)| *id)
            .ok_or(PathError::UnknownPackage)?
    };
    let canonical = format!("{}/{rest}", graph.packages[target].name);
    let id = program
        .modules
        .iter()
        .position(|m| m.package == target && m.path == canonical && m.kind == ModuleKind::Src)
        .ok_or(PathError::UnknownModule)?;
    Ok((ModuleId(id as u32), name.to_owned()))
}

/// Resolve a service constructor under the declaring package's authority.
pub fn resolve_service(
    graph: &PackageGraph,
    program: &Program,
    authority: PackageId,
    path: &str,
) -> Result<TypeId, PathError> {
    let (module, name) = module(graph, program, authority, path)?;
    let id = program
        .module(module)
        .types
        .iter()
        .copied()
        .find(|id| program.ty(*id).name == name)
        .ok_or(PathError::UnknownName)?;
    if program.module(module).package != authority && !program.ty(id).exported {
        return Err(PathError::Private);
    }
    Ok(id)
}

/// Resolve a factory or adapter binding under the declaring package's authority.
pub fn resolve_binding(
    graph: &PackageGraph,
    program: &Program,
    authority: PackageId,
    path: &str,
) -> Result<DefId, PathError> {
    let (module, name) = module(graph, program, authority, path)?;
    let (index, def) = program
        .module(module)
        .defs
        .iter()
        .enumerate()
        .find(|(_, def)| def.name == name)
        .ok_or(PathError::UnknownName)?;
    if program.module(module).package != authority && !def.exported {
        return Err(PathError::Private);
    }
    Ok(DefId {
        module,
        index: index as u32,
    })
}
