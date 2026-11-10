//! What a custom type is, whatever a module calls it: its package, its
//! module in that package, and its name. Two types of a program are the
//! same type when they have the same identity; across two versions of a
//! program, as `i publish` compares them, a type keeps its identity when it
//! stays in the same place (§2.7).

use crate::hir::{Program, TypeId};
use ilang_package::{PackageGraph, PackageId};
use std::path::PathBuf;

/// What a package is (§2.7).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackageIdentity {
    /// The library package `i`.
    Library,
    /// The package that is compiled.
    Root,
    /// A package got by location: the location, with its `//dir`, and the
    /// major version. A new major version is another package (§2.7).
    Location { location: String, major: u64 },
    /// A path dependency: its canonical directory.
    Path(PathBuf),
    /// A package with no location and no directory, by its name.
    Named(String),
}

/// What a custom type is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeIdentity {
    pub package: PackageIdentity,
    /// The module path inside its package, without the package name, as
    /// `geometry/shapes`.
    pub module: String,
    /// The name of the type.
    pub name: String,
}

/// What the package `id` of `graph` is.
pub fn package_identity(graph: &PackageGraph, id: PackageId) -> PackageIdentity {
    let p = &graph.packages[id];
    if id == graph.library {
        return PackageIdentity::Library;
    }
    if Some(id) == graph.root {
        return PackageIdentity::Root;
    }
    match (&p.location, &p.dir) {
        (Some(l), _) => PackageIdentity::Location {
            location: l.location.clone(),
            major: l.version.major,
        },
        (None, Some(dir)) => PackageIdentity::Path(dir.clone()),
        (None, None) => PackageIdentity::Named(p.name.clone()),
    }
}

/// What the custom type `id` of `program`, resolved from `graph`, is.
pub fn type_identity(graph: &PackageGraph, program: &Program, id: TypeId) -> TypeIdentity {
    let t = program.ty(id);
    let m = program.module(t.module);
    TypeIdentity {
        package: package_identity(graph, m.package),
        module: m
            .path
            .split_once('/')
            .map_or(m.path.as_str(), |(_, p)| p)
            .to_string(),
        name: t.name.clone(),
    }
}
