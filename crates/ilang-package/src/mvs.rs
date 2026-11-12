//! Minimal version selection.
//!
//! A package with a location is a location and a major version. Starting
//! from the requirements of the compiled package, every version that is
//! reached has its `i.toml` read, and its dependencies are reached too, each
//! with its lowest version. The version used for each package is the highest
//! one that is reached.
//!
//! This module only does the selection. Reading an `i.toml` at a version is
//! given as a function, so the selection does not know about git.

use crate::manifest::Version;
use std::collections::{BTreeMap, HashSet};

/// A package with a location: the location, with its `//dir`, and the
/// major version.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageKey {
    /// The repository, then `//` and the directory when there is one.
    pub location: String,
    pub major: u64,
}

impl PackageKey {
    pub fn new(location: impl Into<String>, version: Version) -> PackageKey {
        PackageKey {
            location: location.into(),
            major: version.major,
        }
    }
}

/// One package at its lowest version, and where the requirement comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Requirement<F> {
    pub key: PackageKey,
    pub version: Version,
    pub from: F,
}

/// The result of a selection.
#[derive(Clone, Debug)]
pub struct Selection<F> {
    /// The version used for each package that is reached.
    pub versions: BTreeMap<PackageKey, Version>,
    /// Every version that is reached, with the first requirement that
    /// reached it, in the order they were reached.
    pub reached: Vec<Requirement<F>>,
}

impl<F> Selection<F> {
    /// The first requirement that reached the version used for `key`.
    pub fn origin(&self, key: &PackageKey) -> Option<&F> {
        let v = self.versions.get(key)?;
        self.reached
            .iter()
            .find(|r| &r.key == key && r.version == *v)
            .map(|r| &r.from)
    }
}

/// Chooses a version for each package that `roots` reach.
///
/// `dependencies` reads the `i.toml` of a package at a version, and gives
/// its dependencies with their lowest versions. It is called once for each
/// version that is reached, with the first requirement that reached it.
/// When it fails, the error is kept, the version is still reached, and its
/// dependencies are not.
pub fn select<F: Clone, E>(
    roots: impl IntoIterator<Item = Requirement<F>>,
    mut dependencies: impl FnMut(&Requirement<F>) -> Result<Vec<Requirement<F>>, E>,
) -> (Selection<F>, Vec<E>) {
    let mut versions: BTreeMap<PackageKey, Version> = BTreeMap::new();
    let mut reached: Vec<Requirement<F>> = Vec::new();
    let mut seen: HashSet<(PackageKey, Version)> = HashSet::new();
    let mut errors = Vec::new();
    let mut work: Vec<Requirement<F>> = roots.into_iter().collect();
    work.reverse();
    while let Some(req) = work.pop() {
        if !seen.insert((req.key.clone(), req.version)) {
            continue;
        }
        let used = versions.entry(req.key.clone()).or_insert(req.version);
        if req.version > *used {
            *used = req.version;
        }
        match dependencies(&req) {
            Ok(deps) => work.extend(deps.into_iter().rev()),
            Err(e) => errors.push(e),
        }
        reached.push(req);
    }
    (Selection { versions, reached }, errors)
}
