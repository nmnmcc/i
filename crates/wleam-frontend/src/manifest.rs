//! Local, deterministic package resolution shared by the CLI and language server.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use camino::{Utf8Path, Utf8PathBuf};
use gleam_core::{
    config::PackageConfig,
    manifest::{Manifest, ManifestPackage, ManifestPackageSource},
    requirement::Requirement,
};

use crate::Error;

mod bundled {
    include!(concat!(env!("OUT_DIR"), "/bundled_stdlib.rs"));
}

/// A package in the resolved local dependency graph.
#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub root: PathBuf,
    pub config: PackageConfig,
    pub is_root: bool,
    pub is_stdlib: bool,
    /// Only the dependency edges active for this compilation.
    pub dependencies: BTreeSet<String>,
}

/// Locate the standard library distributed with this compiler.
///
/// An explicit override is useful for developing the standard library. Installed
/// distributions can place sources next to the executable; otherwise the exact
/// sources embedded at compiler build time are materialised in the user's cache.
pub fn bundled_stdlib_path() -> Result<PathBuf, Error> {
    if let Some(path) = std::env::var_os("WLEAM_STDLIB") {
        return verify_stdlib_location(PathBuf::from(path));
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        for candidate in [
            directory.join("../lib/wleam/stdlib"),
            directory.join("../share/wleam/stdlib"),
            directory.join("stdlib"),
        ] {
            if candidate.join("gleam.toml").is_file() {
                return verify_stdlib_location(candidate);
            }
        }
    }
    let development = PathBuf::from(bundled::DEVELOPMENT_STDLIB);
    if development.join("gleam.toml").is_file() {
        return verify_stdlib_location(development);
    }
    extract_stdlib()
}

fn verify_stdlib_location(path: PathBuf) -> Result<PathBuf, Error> {
    let canonical = canonical_directory(&path)?;
    let manifest_path = canonical.join("gleam.toml");
    let source =
        fs::read_to_string(&manifest_path).map_err(|error| Error::io(&manifest_path, error))?;
    let manifest: toml::Value = toml::from_str(&source).map_err(|error| {
        Error::project(
            &manifest_path,
            format!("Invalid standard library manifest: {error}"),
        )
    })?;
    if manifest.get("name").and_then(toml::Value::as_str) != Some("gleam_stdlib") {
        return Err(Error::project(
            &manifest_path,
            "The standard library directory must contain a package named gleam_stdlib",
        ));
    }
    Ok(canonical)
}

fn extract_stdlib() -> Result<PathBuf, Error> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| std::env::temp_dir().join(format!("wleam-{}", std::process::id())));
    let root = cache.join("wleam/stdlib").join(bundled::STDLIB_ID);
    materialise_stdlib(&root)?;
    verify_stdlib_location(root)
}

fn materialise_stdlib(root: &Path) -> Result<(), Error> {
    static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);
    for (relative, source) in bundled::STDLIB_FILES {
        let path = root.join(relative);
        if fs::read(&path).ok().as_deref() == Some(source.as_bytes()) {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
        }
        // Sources are immutable content-addressed resources. Concurrent writers
        // write identical bytes; each thread uses its own temporary path.
        let temporary = path.with_extension(format!(
            "wleam-{}-{}.tmp",
            std::process::id(),
            NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed),
        ));
        fs::write(&temporary, source).map_err(|error| Error::io(&temporary, error))?;
        if let Err(error) = fs::rename(&temporary, &path) {
            // Windows refuses to rename over an existing destination. Another
            // compiler may have completed the identical extraction meanwhile.
            let already_written = fs::read(&path).ok().as_deref() == Some(source.as_bytes());
            let _ = fs::remove_file(&temporary);
            if !already_written {
                return Err(Error::io(&path, error));
            }
        }
    }
    Ok(())
}

/// Make the bundled standard library an explicit direct dependency for analysis.
pub fn prepare_package_config(config: &mut PackageConfig) -> Result<(), Error> {
    if config.name != "gleam_stdlib" && !config.dependencies.contains_key("gleam_stdlib") {
        let path = bundled_stdlib_path()?;
        let path = utf8_path(path)?;
        config
            .dependencies
            .insert("gleam_stdlib".into(), Requirement::Path { path });
    }
    Ok(())
}

/// Resolve packages in deterministic dependency-first order.
///
/// Development dependencies are included only for the root package. A shared
/// package reached through more than one path is loaded once; conflicting paths
/// for the same package name, aliases, and cycles are errors.
pub fn load_packages(root: &Path, include_dev: bool) -> Result<Vec<Package>, Error> {
    let root = canonical_directory(root)?;
    let mut resolver = Resolver {
        root: root.clone(),
        include_dev,
        stdlib: None,
        names: BTreeMap::new(),
        completed: BTreeSet::new(),
        stack: Vec::new(),
        packages: Vec::new(),
    };
    resolver.visit(root, None)?;
    Ok(resolver.packages)
}

/// Build the upstream checking compiler's manifest without network resolution.
pub fn dependency_manifest(root: &Utf8Path) -> Result<Manifest, Error> {
    let packages = load_packages(root.as_std_path(), true)?;
    let root_package = packages
        .iter()
        .find(|package| package.is_root)
        .ok_or_else(|| {
            Error::project(root.as_std_path(), "The project graph has no root package")
        })?;
    let mut requirements = root_package.config.dependencies.clone();
    for (name, requirement) in &root_package.config.dev_dependencies {
        requirements.insert(name.clone(), requirement.clone());
    }
    let packages = packages
        .into_iter()
        .filter(|package| !package.is_root)
        .map(|package| {
            Ok(ManifestPackage {
                name: package.name.into(),
                version: package.config.version,
                build_tools: vec!["gleam".into()],
                otp_app: None,
                requirements: package.dependencies.into_iter().map(Into::into).collect(),
                source: ManifestPackageSource::Local {
                    path: utf8_path(package.root)?,
                },
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(Manifest {
        requirements,
        packages,
    })
}

struct Resolver {
    root: PathBuf,
    include_dev: bool,
    stdlib: Option<PathBuf>,
    names: BTreeMap<String, PathBuf>,
    completed: BTreeSet<PathBuf>,
    stack: Vec<(String, PathBuf)>,
    packages: Vec<Package>,
}

impl Resolver {
    fn visit(&mut self, root: PathBuf, expected_name: Option<&str>) -> Result<(), Error> {
        let root = canonical_directory(&root)?;
        let manifest_path = root.join("gleam.toml");
        let mut config = read_config(&manifest_path)?;
        if let Some(expected) = expected_name
            && expected != config.name.as_str()
        {
            return Err(Error::project(
                &manifest_path,
                format!(
                    "Dependency {expected:?} points to package {:?}; dependency names must match the package manifest",
                    config.name,
                ),
            ));
        }
        let name = config.name.to_string();
        if let Some(index) = self.stack.iter().position(|(_, path)| path == &root) {
            let mut cycle = self.stack[index..]
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>();
            cycle.push(&name);
            return Err(Error::project(
                &manifest_path,
                format!("Local dependency cycle: {}", cycle.join(" -> ")),
            ));
        }
        if let Some(previous) = self.names.get(&name)
            && previous != &root
        {
            return Err(Error::project(
                &manifest_path,
                format!(
                    "Package {name:?} resolves to two directories: {} and {}",
                    previous.display(),
                    root.display(),
                ),
            ));
        }
        if self.completed.contains(&root) {
            return Ok(());
        }
        self.names.insert(name.clone(), root.clone());
        self.stack.push((name.clone(), root.clone()));
        let is_root = root == self.root;
        if is_root && let Some(Requirement::Path { path }) = config.dependencies.get("gleam_stdlib")
        {
            self.stdlib = Some(canonical_directory(&root.join(path.as_std_path()))?);
        }
        if config.name != "gleam_stdlib" && !config.dependencies.contains_key("gleam_stdlib") {
            let path = match &self.stdlib {
                Some(path) => path.clone(),
                None => {
                    let path = bundled_stdlib_path()?;
                    self.stdlib = Some(path.clone());
                    path
                }
            };
            config.dependencies.insert(
                "gleam_stdlib".into(),
                Requirement::Path {
                    path: utf8_path(path)?,
                },
            );
        }
        let include_dev = is_root && self.include_dev;
        let mut active = BTreeMap::new();
        for (dependency, requirement) in &config.dependencies {
            active.insert(dependency.to_string(), requirement.clone());
        }
        if include_dev {
            for (dependency, requirement) in &config.dev_dependencies {
                if active
                    .insert(dependency.to_string(), requirement.clone())
                    .is_some()
                {
                    return Err(Error::project(
                        &manifest_path,
                        format!(
                            "Dependency {dependency:?} is listed in both dependencies and dev-dependencies",
                        ),
                    ));
                }
            }
        }
        let mut dependencies = BTreeSet::new();
        let mut canonical_dependencies = HashMap::new();
        for (dependency, requirement) in active {
            let dependency_root = match requirement {
                Requirement::Path { path } => canonical_directory(&root.join(path.as_std_path()))?,
                Requirement::Hex { .. } => {
                    return Err(Error::project(
                        &manifest_path,
                        format!(
                            "Dependency {dependency:?} uses an online package requirement. Wleam supports local path dependencies; use {{ path = \"...\" }} instead",
                        ),
                    ));
                }
                Requirement::Git { .. } => {
                    return Err(Error::project(
                        &manifest_path,
                        format!(
                            "Dependency {dependency:?} uses Git resolution. Check it out locally and use {{ path = \"...\" }} instead",
                        ),
                    ));
                }
            };
            self.visit(dependency_root.clone(), Some(&dependency))?;
            canonical_dependencies.insert(
                dependency.clone(),
                Requirement::Path {
                    path: utf8_path(dependency_root)?,
                },
            );
            dependencies.insert(dependency);
        }
        for (name, requirement) in &mut config.dependencies {
            if let Some(canonical) = canonical_dependencies.get(name.as_str()) {
                *requirement = canonical.clone();
            }
        }
        if include_dev {
            for (name, requirement) in &mut config.dev_dependencies {
                if let Some(canonical) = canonical_dependencies.get(name.as_str()) {
                    *requirement = canonical.clone();
                }
            }
        }
        self.stack.pop();
        self.completed.insert(root.clone());
        self.packages.push(Package {
            is_stdlib: name == "gleam_stdlib",
            name,
            root,
            config,
            is_root,
            dependencies,
        });
        Ok(())
    }
}

fn read_config(path: &Path) -> Result<PackageConfig, Error> {
    let source = fs::read_to_string(path).map_err(|error| Error::io(path, error))?;
    let mut value: toml::Value = toml::from_str(&source)
        .map_err(|error| Error::project(path, format!("Invalid gleam.toml: {error}")))?;
    let table = value
        .as_table_mut()
        .ok_or_else(|| Error::project(path, "The project manifest must be a TOML table"))?;
    if let Some(target) = table.get("target")
        && target.as_str() != Some("wasm")
    {
        return Err(Error::project(
            path,
            "Wleam only supports target = \"wasm\"; remove the old target or change it to wasm",
        ));
    }
    for legacy in ["erlang", "javascript"] {
        if table.contains_key(legacy) {
            return Err(Error::project(
                path,
                format!(
                    "The [{legacy}] configuration belongs to an unsupported backend; remove it when migrating to Wleam"
                ),
            ));
        }
    }
    table.insert("target".into(), toml::Value::String("wasm".into()));
    let config: PackageConfig = value
        .try_into()
        .map_err(|error| Error::project(path, format!("Invalid package configuration: {error}")))?;
    config.check_gleam_compatibility().map_err(Error::Gleam)?;
    Ok(config)
}

fn canonical_directory(path: &Path) -> Result<PathBuf, Error> {
    let path = if path.file_name().is_some_and(|name| name == "gleam.toml") {
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
    } else {
        path
    };
    let canonical = path
        .canonicalize()
        .map_err(|error| Error::io(path, error))?;
    if !canonical.is_dir() {
        return Err(Error::project(
            path,
            "A package path must refer to a directory containing gleam.toml",
        ));
    }
    Ok(canonical)
}

fn utf8_path(path: PathBuf) -> Result<Utf8PathBuf, Error> {
    Utf8PathBuf::from_path_buf(path.clone())
        .map_err(|_| Error::project(path, "Wleam package paths must be valid UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_sources_can_be_extracted_and_repaired() {
        let directory = tempfile::TempDir::new().unwrap();
        materialise_stdlib(directory.path()).unwrap();
        for (relative, source) in bundled::STDLIB_FILES {
            assert_eq!(
                fs::read_to_string(directory.path().join(relative)).unwrap(),
                *source
            );
        }
        let damaged = directory.path().join("gleam.toml");
        fs::write(&damaged, "damaged cache").unwrap();
        materialise_stdlib(directory.path()).unwrap();
        assert_eq!(
            verify_stdlib_location(directory.path().to_path_buf()).unwrap(),
            directory.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn parallel_extraction_does_not_publish_partial_files() {
        let directory = tempfile::TempDir::new().unwrap();
        std::thread::scope(|scope| {
            let mut workers = Vec::new();
            for _ in 0..4 {
                let root = directory.path();
                workers
                    .push(scope.spawn(move || {
                        materialise_stdlib(root).map_err(|error| error.to_string())
                    }));
            }
            for worker in workers {
                worker.join().unwrap().unwrap();
            }
        });
        for (relative, source) in bundled::STDLIB_FILES {
            assert_eq!(
                fs::read_to_string(directory.path().join(relative)).unwrap(),
                *source
            );
        }
    }
}
