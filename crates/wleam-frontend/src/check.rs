use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use camino::{Utf8Path, Utf8PathBuf};
use ecow::EcoString;
use gleam_core::{
    analyse::{ModuleAnalyzerConstructor, TargetSupport},
    ast::TypedModule,
    build::{Origin, Outcome, Target},
    error::{DefinedModuleOrigin, FailedModule, UnknownImportDetails},
    parse::{self, Parsed},
    type_::{ModuleInterface, build_prelude, printer::Names},
    uid::UniqueIdGenerator,
    warning::{TypeWarningEmitter, WarningEmitter},
};
use serde::{Deserialize, Serialize};
use src_span::{LineNumbers, SrcSpan};

use crate::{Error, Package, Result, cache, load_packages};

#[derive(Debug, Clone)]
pub struct CheckOptions {
    pub include_tests: bool,
    pub use_cache: bool,
    /// Open editor buffers keyed by absolute path or path relative to the root.
    pub source_overrides: BTreeMap<PathBuf, String>,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            include_tests: false,
            use_cache: true,
            source_overrides: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheStats {
    pub parsed: usize,
    pub checked: usize,
    pub memory_hits: usize,
    pub disk_hits: usize,
    pub cache_writes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub module_name: String,
    pub package_name: String,
    pub path: PathBuf,
    pub code: String,
    pub is_test: bool,
    pub origin: Origin,
}

#[derive(Debug, Clone)]
pub struct CheckedProject {
    pub package_name: String,
    pub root: PathBuf,
    /// Dependency-first, deterministic order. Trees are shared with in-process
    /// caches and must be treated as immutable by code generators.
    pub modules: Vec<Arc<TypedModule>>,
    pub sources: BTreeMap<String, SourceFile>,
    pub exports: Vec<String>,
    pub warnings: Vec<String>,
    pub stats: CacheStats,
}

impl CheckedProject {
    pub fn module(&self, name: &str) -> Option<&TypedModule> {
        self.modules
            .iter()
            .find(|module| module.name == name)
            .map(Arc::as_ref)
    }
}

/// A reusable checking session. A new session can reuse persisted typed trees;
/// an editor session additionally avoids serialisation and disk reloads.
#[derive(Debug)]
pub struct Project {
    root: PathBuf,
    packages: Vec<Package>,
    cache: BTreeMap<String, cache::Entry>,
    ids: UniqueIdGenerator,
}

impl Project {
    pub fn load(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let root = if root.file_name().is_some_and(|name| name == "gleam.toml") {
            root.parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
        } else {
            root
        };
        let root = fs::canonicalize(root).map_err(|error| Error::io(root, error))?;
        let packages = load_packages(&root, false)?;
        Ok(Self {
            root,
            packages,
            cache: BTreeMap::new(),
            ids: UniqueIdGenerator::new(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn packages(&self) -> &[Package] {
        &self.packages
    }

    pub fn check(&mut self, options: &CheckOptions) -> Result<CheckedProject> {
        // Manifests and dependency directories can change during an editor
        // session; resolving them again prevents stale dependency permissions.
        self.packages = load_packages(&self.root, options.include_tests)?;
        let root_package = self
            .packages
            .iter()
            .find(|package| package.is_root)
            .ok_or_else(|| Error::project(&self.root, "project has no root package"))?;
        let package_name = root_package.name.clone();
        let exports = root_package
            .config
            .wasm
            .exports
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let sources = collect_sources(&self.packages, &self.root, options)?;
        let directory = self.root.join("build/.wleam/frontend-v1");
        let mut stats = CacheStats::default();
        let packages: BTreeMap<_, _> = self
            .packages
            .iter()
            .map(|package| (package.name.clone(), package))
            .collect();
        let mut pending = BTreeMap::new();
        for (name, source) in &sources {
            let package = packages
                .get(&source.package_name)
                .expect("source's package is loaded");
            let key =
                cache::source_key(&source.code, &source.path, &package.config, source.is_test);
            let cached = if options.use_cache {
                self.cache
                    .get(name)
                    .cloned()
                    .map(|entry| (entry, CacheOrigin::Memory))
                    .or_else(|| {
                        cache::read(&directory, name).map(|entry| (entry, CacheOrigin::Disk))
                    })
            } else {
                None
            };
            // Reserve every cached ID before inferring any changed module,
            // including stale source entries whose dependents may still hit.
            if let Some((entry, _)) = &cached {
                self.ids.reserve_until(entry.next_id);
            }
            let cached = cached
                .filter(|(entry, _)| entry.source_key == key && entry.schema == cache::schema());
            pending.insert(
                name.clone(),
                Pending {
                    key,
                    cached,
                    parsed: None,
                    dependencies: Vec::new(),
                    parse_warnings: Vec::new(),
                },
            );
        }

        let mut roots = sources
            .iter()
            .filter(|(_, source)| source.package_name == package_name)
            .map(|(name, _)| name.clone())
            .collect::<BTreeSet<_>>();
        for export in &exports {
            let (module, function) = export.rsplit_once('.').ok_or_else(|| {
                Error::project(
                    self.root.join("gleam.toml"),
                    format!("Wasm export {export:?} must be written as module.function"),
                )
            })?;
            if module.is_empty() || function.is_empty() {
                return Err(Error::project(
                    self.root.join("gleam.toml"),
                    format!("Invalid Wasm export {export:?}"),
                ));
            }
            if !sources.contains_key(module) {
                return Err(Error::project(
                    self.root.join("gleam.toml"),
                    format!("Export refers to missing module {module:?}"),
                ));
            }
            roots.insert(module.to_owned());
        }
        let mut order = Vec::new();
        let mut visiting = Vec::new();
        let mut visited = HashSet::new();
        for name in roots {
            visit(
                &name,
                &sources,
                &mut pending,
                &mut visiting,
                &mut visited,
                &mut order,
                &mut stats,
            )?;
        }

        let mut interfaces = imbl::HashMap::<EcoString, ModuleInterface>::new();
        interfaces.insert("gleam".into(), build_prelude(&self.ids));
        let mut interface_hashes = BTreeMap::new();
        let mut modules = Vec::new();
        let mut warnings = Vec::new();
        for name in &order {
            let source = &sources[name];
            let package = packages[&source.package_name];
            let mut input = pending
                .remove(name)
                .expect("module was prepared by graph traversal");
            let dependency_interfaces = input
                .dependencies
                .iter()
                .filter(|(dependency, _)| dependency != "gleam")
                .map(|(dependency, _)| {
                    (
                        dependency.clone(),
                        interface_hashes
                            .get(dependency)
                            .cloned()
                            .expect("dependencies were checked first"),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            if let Some((entry, origin)) = input.cached.take()
                && entry.dependency_interfaces == dependency_interfaces
            {
                match origin {
                    CacheOrigin::Memory => stats.memory_hits += 1,
                    CacheOrigin::Disk => stats.disk_hits += 1,
                }
                interfaces.insert(name.as_str().into(), entry.module.type_info.clone());
                interface_hashes.insert(name.clone(), entry.interface_hash.clone());
                warnings.extend(entry.warnings.clone());
                modules.push(Arc::clone(&entry.module));
                self.cache.insert(name.clone(), entry);
                continue;
            }

            if input.parsed.is_none() {
                parse_input(source, &mut input, &mut stats)?;
            }
            let Parsed { module, extra } = input.parsed.take().expect("new module was parsed");
            let path = utf8_path(&source.path)?;
            let code = EcoString::from(source.code.as_str());
            let (emitter, emitted_warnings) = WarningEmitter::vector();
            let type_warnings = TypeWarningEmitter::new(path.clone(), code.clone(), emitter);
            let direct_dependencies = package
                .dependencies
                .iter()
                .map(|name| (EcoString::from(name.as_str()), ()))
                .collect::<HashMap<_, _>>();
            let dev_dependencies = package
                .config
                .dev_dependencies
                .keys()
                .cloned()
                .collect::<HashSet<_>>();
            let outcome = ModuleAnalyzerConstructor {
                target: Target::Wasm,
                ids: &self.ids,
                origin: source.origin,
                importable_modules: &interfaces,
                warnings: &type_warnings,
                direct_dependencies: &direct_dependencies,
                dev_dependencies: &dev_dependencies,
                target_support: TargetSupport::Enforced,
                package_config: &package.config,
            }
            .infer_module(module, LineNumbers::new(&code), path.clone());
            stats.checked += 1;
            let ast = match outcome {
                Outcome::Ok(module) => module,
                Outcome::PartialFailure(module, errors) => {
                    return Err(type_error(source, path, module.names, errors));
                }
                Outcome::TotalFailure(errors) => {
                    return Err(type_error(source, path, Names::new(), errors));
                }
            };
            let mut module = gleam_core::build::Module {
                name: name.as_str().into(),
                code,
                mtime: SystemTime::UNIX_EPOCH,
                input_path: path,
                origin: source.origin,
                ast,
                extra,
                dependencies: input
                    .dependencies
                    .iter()
                    .map(|(name, span)| (name.as_str().into(), *span))
                    .collect(),
            };
            module.attach_doc_and_module_comments();
            // Upstream type variables use RefCell during inference. Finished
            // trees share the core's Arc representation but stay on this thread.
            #[allow(clippy::arc_with_non_send_sync)]
            let module = Arc::new(module.ast);
            input.parse_warnings.extend(
                emitted_warnings
                    .take()
                    .into_iter()
                    .map(|warning| warning.to_pretty_string()),
            );
            let interface_hash = cache::interface_hash(&module.type_info);
            interfaces.insert(name.as_str().into(), module.type_info.clone());
            interface_hashes.insert(name.clone(), interface_hash.clone());
            warnings.extend(input.parse_warnings.clone());
            let entry = cache::Entry {
                schema: cache::schema(),
                source_key: input.key,
                dependencies: input.dependencies,
                dependency_interfaces,
                interface_hash,
                next_id: self.ids.next() + 1,
                module: Arc::clone(&module),
                warnings: input.parse_warnings,
            };
            if options.use_cache {
                match cache::write(&directory, name, &entry) {
                    Ok(()) => stats.cache_writes += 1,
                    Err(error) => warnings.push(format!("Could not cache module {name}: {error}")),
                }
                self.cache.insert(name.clone(), entry);
            }
            modules.push(module);
        }
        for export in &exports {
            let (module, function) = export.rsplit_once('.').expect("export syntax was checked");
            let exported = interfaces
                .get(module)
                .and_then(|interface| interface.get_public_value(function));
            if !exported.is_some_and(|value| value.variant.is_module_fn()) {
                return Err(Error::project(
                    self.root.join("gleam.toml"),
                    format!("Wasm export {export:?} must refer to a public function"),
                ));
            }
        }
        let reachable = order.into_iter().collect::<BTreeSet<_>>();
        let sources = sources
            .into_iter()
            .filter(|(name, _)| reachable.contains(name))
            .collect();
        Ok(CheckedProject {
            package_name,
            root: self.root.clone(),
            modules,
            sources,
            exports,
            warnings,
            stats,
        })
    }
}

pub fn check_project(root: impl AsRef<Path>, options: &CheckOptions) -> Result<CheckedProject> {
    Project::load(root)?.check(options)
}

#[derive(Debug, Clone, Copy)]
enum CacheOrigin {
    Memory,
    Disk,
}

struct Pending {
    key: String,
    cached: Option<(cache::Entry, CacheOrigin)>,
    parsed: Option<Parsed>,
    dependencies: Vec<(String, SrcSpan)>,
    parse_warnings: Vec<String>,
}

fn parse_input(source: &SourceFile, pending: &mut Pending, stats: &mut CacheStats) -> Result<()> {
    let path = utf8_path(&source.path)?;
    let (warnings, emitted) = WarningEmitter::vector();
    let mut parsed =
        parse::parse_module(path.clone(), &source.code, &warnings).map_err(|error| {
            gleam_core::Error::Parse {
                path,
                src: source.code.as_str().into(),
                error: Box::new(error),
            }
        })?;
    parsed.module.name = source.module_name.as_str().into();
    pending.dependencies = parsed
        .module
        .dependencies(Target::Wasm)
        .into_iter()
        .map(|(name, span)| (name.to_string(), span))
        .collect();
    pending.parse_warnings = emitted
        .take()
        .into_iter()
        .map(|warning| warning.to_pretty_string())
        .collect();
    pending.parsed = Some(parsed);
    stats.parsed += 1;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn visit(
    name: &str,
    sources: &BTreeMap<String, SourceFile>,
    pending: &mut BTreeMap<String, Pending>,
    visiting: &mut Vec<String>,
    visited: &mut HashSet<String>,
    order: &mut Vec<String>,
    stats: &mut CacheStats,
) -> Result<()> {
    if visited.contains(name) {
        return Ok(());
    }
    if let Some(start) = visiting.iter().position(|module| module == name) {
        let mut cycle = visiting[start..].to_vec();
        cycle.push(name.to_owned());
        return Err(Error::project(
            &sources[name].path,
            format!("Import cycle: {}", cycle.join(" -> ")),
        ));
    }
    visiting.push(name.to_owned());
    let source = &sources[name];
    let input = pending.get_mut(name).expect("source has a pending module");
    if let Some((entry, _)) = &input.cached {
        input.dependencies = entry.dependencies.clone();
    } else if input.parsed.is_none() {
        parse_input(source, input, stats)?;
    }
    let dependencies = input.dependencies.clone();
    for (dependency, span) in dependencies {
        if dependency == "gleam" {
            continue;
        }
        if !sources.contains_key(&dependency) {
            return Err(gleam_core::Error::UnknownImport {
                import: dependency.as_str().into(),
                details: Box::new(UnknownImportDetails {
                    module: name.into(),
                    location: span,
                    path: utf8_path(&source.path)?,
                    src: source.code.as_str().into(),
                    modules: sources.keys().map(|name| name.as_str().into()).collect(),
                }),
            }
            .into());
        }
        visit(
            &dependency,
            sources,
            pending,
            visiting,
            visited,
            order,
            stats,
        )?;
    }
    visiting.pop();
    visited.insert(name.to_owned());
    order.push(name.to_owned());
    Ok(())
}

fn type_error(
    source: &SourceFile,
    path: Utf8PathBuf,
    names: Names,
    errors: vec1::Vec1<gleam_core::type_::Error>,
) -> Error {
    let failed = FailedModule {
        path,
        src: source.code.as_str().into(),
        errors,
        names: Box::new(names),
    };
    gleam_core::Error::Type {
        skipped_modules: Vec::new(),
        failed_modules: HashMap::from([(source.module_name.as_str().into(), failed)]),
    }
    .into()
}

fn collect_sources(
    packages: &[Package],
    root: &Path,
    options: &CheckOptions,
) -> Result<BTreeMap<String, SourceFile>> {
    let overrides = options
        .source_overrides
        .iter()
        .map(|(path, source)| {
            let path = if path.is_absolute() {
                path.clone()
            } else {
                root.join(path)
            };
            (fs::canonicalize(&path).unwrap_or(path), source)
        })
        .collect::<BTreeMap<_, _>>();
    let mut sources = BTreeMap::<String, SourceFile>::new();
    for package in packages {
        let mut folders = vec![("src", Origin::Src)];
        if package.is_root && options.include_tests {
            folders.extend([("test", Origin::Test), ("dev", Origin::Dev)]);
        }
        for (folder, origin) in folders {
            let source_root = package.root.join(folder);
            let mut paths = BTreeSet::new();
            walk_sources(&source_root, &mut paths)?;
            for path in overrides.keys() {
                if path.starts_with(&source_root)
                    && path
                        .extension()
                        .is_some_and(|extension| extension == "gleam")
                {
                    paths.insert(path.clone());
                }
            }
            for path in paths {
                let relative = path
                    .strip_prefix(&source_root)
                    .map_err(|_| Error::project(&path, "source escaped its source directory"))?;
                let name = relative
                    .with_extension("")
                    .to_str()
                    .ok_or_else(|| Error::project(&path, "module paths must be UTF-8"))?
                    .replace('\\', "/");
                if !name.split('/').all(|segment| {
                    let mut characters = segment.bytes();
                    characters
                        .next()
                        .is_some_and(|first| first.is_ascii_lowercase())
                        && characters.all(|character| {
                            character.is_ascii_lowercase()
                                || character.is_ascii_digit()
                                || character == b'_'
                        })
                }) {
                    return Err(Error::project(
                        &path,
                        format!(
                            "Invalid module name {name:?}: each path segment must start with a lowercase ASCII letter and contain only lowercase letters, digits, or underscores",
                        ),
                    ));
                }
                let canonical = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                let code = match overrides.get(&canonical) {
                    Some(source) => (*source).clone(),
                    None => fs::read_to_string(&path).map_err(|error| Error::io(&path, error))?,
                };
                let source = SourceFile {
                    module_name: name.clone(),
                    package_name: package.name.clone(),
                    path,
                    code,
                    is_test: origin == Origin::Test,
                    origin,
                };
                if let Some(previous) = sources.get(&name) {
                    return Err(gleam_core::Error::DuplicateModule {
                        module: name.as_str().into(),
                        first: DefinedModuleOrigin {
                            package_name: previous.package_name.as_str().into(),
                            path: utf8_path(&previous.path)?,
                        },
                        second: DefinedModuleOrigin {
                            package_name: source.package_name.as_str().into(),
                            path: utf8_path(&source.path)?,
                        },
                    }
                    .into());
                }
                sources.insert(name, source);
            }
        }
    }
    Ok(sources)
}

fn walk_sources(directory: &Path, output: &mut BTreeSet<PathBuf>) -> Result<()> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Error::io(directory, error)),
    };
    for entry in entries {
        let entry = entry.map_err(|error| Error::io(directory, error))?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let path = entry.path();
        let kind = entry.file_type().map_err(|error| Error::io(&path, error))?;
        if kind.is_dir() {
            walk_sources(&path, output)?;
        } else if (kind.is_file() || kind.is_symlink())
            && path
                .extension()
                .is_some_and(|extension| extension == "gleam")
        {
            output.insert(path);
        }
    }
    Ok(())
}

fn utf8_path(path: &Path) -> Result<Utf8PathBuf> {
    Utf8Path::from_path(path)
        .map(Utf8Path::to_owned)
        .ok_or_else(|| Error::project(path, "source paths must be valid UTF-8"))
}
