// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2022 The Gleam contributors

#[cfg(test)]
mod tests;

use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

use camino::{Utf8Path, Utf8PathBuf};

// TODO: emit warnings for cached modules even if they are not compiled again.

use crate::{
    Error, Result,
    build::{Origin, module_loader::ModuleLoader},
    dep_tree,
    error::{DefinedModuleOrigin, FileIoAction, FileIoCause, FileKind, ImportCycleLocationDetails},
    io::{FileSystemReader, FileSystemWriter, files_with_extension},
    metadata,
    paths::ProjectPaths,
    type_,
    uid::UniqueIdGenerator,
    warning::WarningEmitter,
};
use ecow::EcoString;
use itertools::Itertools;
use regex::Regex;
use src_span::SrcSpan;
use vec1::Vec1;

use super::{
    Mode, Target,
    module_loader::read_source,
    package_compiler::{CachedModule, CachedWarnings, Input, Loaded, UncompiledModule},
};

#[derive(Debug)]
pub struct PackageLoader<'a, IO> {
    io: IO,
    ids: UniqueIdGenerator,
    mode: Mode,
    paths: ProjectPaths,
    warnings: &'a WarningEmitter,
    artefact_directory: &'a Utf8Path,
    package_name: &'a EcoString,
    target: Target,
    stale_modules: &'a mut StaleTracker,
    already_defined_modules: &'a mut imbl::HashMap<EcoString, DefinedModuleOrigin>,
    incomplete_modules: &'a HashSet<EcoString>,
    cached_warnings: CachedWarnings,
}

impl<'a, IO> PackageLoader<'a, IO>
where
    IO: FileSystemWriter + FileSystemReader + Clone,
{
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn new(
        io: IO,
        ids: UniqueIdGenerator,
        mode: Mode,
        root: &'a Utf8Path,
        cached_warnings: CachedWarnings,
        warnings: &'a WarningEmitter,
        artefact_directory: &'a Utf8Path,
        target: Target,
        package_name: &'a EcoString,
        stale_modules: &'a mut StaleTracker,
        already_defined_modules: &'a mut imbl::HashMap<EcoString, DefinedModuleOrigin>,
        incomplete_modules: &'a HashSet<EcoString>,
    ) -> Self {
        Self {
            io,
            ids,
            mode,
            paths: ProjectPaths::new(root.into()),
            warnings,
            target,
            package_name,
            cached_warnings,
            artefact_directory,
            stale_modules,
            already_defined_modules,
            incomplete_modules,
        }
    }

    pub(crate) fn run(mut self) -> Result<Loaded> {
        // First read the source files. This will use the `ModuleLoader`, which
        // compares source fingerprints to cached metadata to determine which
        // modules should be loaded.
        let mut inputs = self.read_sources_and_caches()?;

        // Check for any removed modules, by looking at cache files that don't exist in inputs.
        // Delete the cache files for removed modules and mark them as stale
        // to trigger refreshing dependent modules.
        for module in CacheFiles::modules_with_meta_files(&self.io, self.artefact_directory) {
            if !inputs.contains_key(&module) {
                tracing::debug!(%module, "module_removed");
                CacheFiles::new(self.artefact_directory, &module).delete(&self.io)?;
                self.stale_modules.add(module);
            }
        }

        // Determine order in which modules are to be processed
        let mut dep_location_map = HashMap::new();
        let deps = inputs
            .values()
            .map(|(_, module)| {
                let name = module.name().clone();
                let _ = dep_location_map.insert(name.clone(), module);
                (name, module.dependencies())
            })
            // Making sure that the module order is deterministic, to prevent different
            // compilations of the same project compiling in different orders. This could impact
            // any bugged outcomes, though not any where the compiler is working correctly, so it's
            // mostly to aid debugging.
            .sorted_by(|(a, _), (b, _)| a.cmp(b))
            .collect();
        let sequence = dep_tree::toposort_deps(deps)
            .map_err(|error| self.convert_deps_tree_error(error, dep_location_map))?;

        // Now that we have loaded sources and caches we check to see if any of
        // the caches need to be invalidated because their dependencies have
        // changed.
        let mut loaded = Loaded::default();
        for name in sequence {
            let (_, input) = inputs
                .remove(&name)
                .expect("Getting parsed module for name");

            match input {
                // A new uncached module is to be compiled
                Input::New { module } => {
                    tracing::debug!(module = %module.name, "new_module_to_be_compiled");
                    self.stale_modules.add(module.name.clone());
                    loaded.to_compile.push(*module);
                }

                // A cached module with dependencies that are stale must be
                // recompiled as the changes in the dependencies may have affect
                // the output, making the cache invalid.
                Input::Cached { module }
                    if self.stale_modules.includes_any(&module.dependencies) =>
                {
                    tracing::debug!(module = %module.name, "stale_module_to_be_compiled");
                    self.stale_modules.add(module.name.clone());
                    let module = self.load_stale_module(&module)?;
                    loaded.to_compile.push(module);
                }

                // A cached module with no stale dependencies can be used as-is
                // and does not need to be recompiled.
                Input::Cached { module } => {
                    tracing::debug!(module = %module.name, "module_to_load_from_cache");
                    let module = self.load_cached_module(module)?;
                    loaded.cached.push(module);
                }
            }
        }

        Ok(loaded)
    }

    fn load_cached_module(&self, info: CachedModule) -> Result<type_::ModuleInterface, Error> {
        let cache_files = CacheFiles::new(self.artefact_directory, &info.name);
        let bytes = self.io.read_bytes(&cache_files.cache_path)?;
        let mut module = match metadata::decode(bytes.as_slice(), self.ids.clone()) {
            Some(module) => module,
            None => {
                return Err(Error::FileIo {
                    kind: FileKind::File,
                    action: FileIoAction::Parse,
                    path: cache_files.cache_path,
                    cause: FileIoCause::CacheMetadataFormatIncorrect,
                });
            }
        };

        // Discard warnings if they should not be cached
        if !self.cached_warnings.should_use() {
            module.warnings.clear();
        }

        Ok(module)
    }

    fn read_sources_and_caches(
        &mut self,
    ) -> Result<HashMap<EcoString, (DefinedModuleOrigin, Input)>> {
        let span = tracing::info_span!("load");
        let _enter = span.enter();

        let mut inputs = Inputs::new(self.package_name.clone(), self.already_defined_modules);

        let src = self.paths.src_directory();
        let mut loader = ModuleLoader {
            io: self.io.clone(),
            warnings: self.warnings,
            mode: self.mode,
            target: self.target,
            package_name: self.package_name,
            artefact_directory: self.artefact_directory,
            origin: Origin::Src,
            incomplete_modules: self.incomplete_modules,
        };

        // Src
        for file in GleamFile::iterate_files_in_directory(&self.io, &src) {
            match file {
                Ok(file) => {
                    let input = loader.load(file)?;
                    inputs.insert(input)?;
                }
                Err(warning) => self.warnings.emit(warning),
            }
        }

        // Test and dev
        if self.mode.includes_dev_code() {
            let test = self.paths.test_directory();
            loader.origin = Origin::Test;

            for file in GleamFile::iterate_files_in_directory(&self.io, &test) {
                match file {
                    Ok(file) => {
                        let input = loader.load(file)?;
                        inputs.insert(input)?;
                    }
                    Err(warning) => self.warnings.emit(warning),
                }
            }

            let dev = self.paths.dev_directory();
            loader.origin = Origin::Dev;

            for file in GleamFile::iterate_files_in_directory(&self.io, &dev) {
                match file {
                    Ok(file) => {
                        let input = loader.load(file)?;
                        inputs.insert(input)?;
                    }
                    Err(warning) => self.warnings.emit(warning),
                }
            }
        }

        Ok(inputs.collection)
    }

    fn load_stale_module(&self, cached: &CachedModule) -> Result<UncompiledModule> {
        let mtime = self.io.modification_time(&cached.source_path)?;

        // We need to delete any existing cache files for this module.
        // While we figured it out this time because the module has stale dependencies,
        // next time the dependencies might no longer be stale, but we still need to be able to tell
        // that this module needs to be recompiled until it successfully compiles at least once.
        // This can happen if the stale dependency includes breaking changes.
        let cache_files = CacheFiles::new(self.artefact_directory, &cached.name);
        cache_files.delete(&self.io)?;

        read_source(
            self.io.clone(),
            self.target,
            cached.origin,
            cached.source_path.clone(),
            cached.name.clone(),
            self.package_name.clone(),
            mtime,
            self.warnings.clone(),
        )
    }

    fn convert_deps_tree_error(
        &self,
        e: dep_tree::Error,
        dep_location_map: HashMap<EcoString, &Input>,
    ) -> Error {
        match e {
            dep_tree::Error::Cycle(modules) => {
                let modules = modules
                    .iter()
                    .enumerate()
                    .map(|(i, module)| {
                        // cycles are in order of reference so get next in list or loop back to first
                        let index_of_imported = if i == 0 { modules.len() - 1 } else { i - 1 };
                        let imported_module = modules
                            .get(index_of_imported)
                            .expect("importing module must exist");
                        let input = dep_location_map.get(module).expect("dependency must exist");
                        let location = match input {
                            Input::New { module } => {
                                let (_, location) = module
                                    .dependencies
                                    .iter()
                                    .find(|(name, _)| name == imported_module)
                                    .expect("import must exist for there to be a cycle");
                                ImportCycleLocationDetails {
                                    location: *location,
                                    path: module.path.clone(),
                                    src: module.code.clone(),
                                }
                            }
                            Input::Cached {
                                module: cached_module,
                            } => {
                                let (_, location) = cached_module
                                    .dependencies
                                    .iter()
                                    .find(|(name, _)| name == imported_module)
                                    .expect("import must exist for there to be a cycle");
                                let src = self
                                    .io
                                    .read(&cached_module.source_path)
                                    .expect("failed to read source")
                                    .into();
                                ImportCycleLocationDetails {
                                    location: *location,
                                    path: cached_module.source_path.clone(),
                                    src,
                                }
                            }
                        };
                        (module.clone(), location)
                    })
                    .collect_vec();
                Error::ImportCycle {
                    modules: Vec1::try_from(modules)
                        .expect("at least 1 module must exist in cycle"),
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct StaleTracker(HashSet<EcoString>);

impl StaleTracker {
    fn add(&mut self, name: EcoString) {
        _ = self.0.insert(name);
    }

    fn includes_any(&self, names: &[(EcoString, SrcSpan)]) -> bool {
        names.iter().any(|(name, _)| self.0.contains(name.as_str()))
    }

    pub fn empty(&mut self) {
        let _ = self.0.drain(); // Clears the set but retains allocated memory
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug)]
pub struct Inputs<'a> {
    /// The name of the package for which we're loading the inputs.
    package: EcoString,
    collection: HashMap<EcoString, (DefinedModuleOrigin, Input)>,
    already_defined_modules: &'a mut imbl::HashMap<EcoString, DefinedModuleOrigin>,
}

impl<'a> Inputs<'a> {
    fn new(
        package: EcoString,
        already_defined_modules: &'a mut imbl::HashMap<EcoString, DefinedModuleOrigin>,
    ) -> Self {
        Self {
            package,
            collection: HashMap::new(),
            already_defined_modules,
        }
    }

    /// Insert a module into the hashmap. If there is already a module with the
    /// same name then an error is returned.
    fn insert(&mut self, input: Input) -> Result<()> {
        let name = input.name().clone();

        let origin = DefinedModuleOrigin {
            package_name: self.package.clone(),
            path: input.source_path().to_path_buf(),
        };

        if let Some(first) = self
            .already_defined_modules
            .insert(name.clone(), origin.clone())
        {
            return Err(Error::DuplicateModule {
                module: name,
                first,
                second: origin,
            });
        }

        if let Some((first, _)) = self
            .collection
            .insert(name.clone(), (origin.clone(), input))
        {
            return Err(Error::DuplicateModule {
                module: name,
                first,
                second: origin,
            });
        }

        Ok(())
    }
}

/// A Gleam source file (`.gleam`) and the module name deduced from it
pub struct GleamFile {
    pub path: Utf8PathBuf,
    pub module_name: EcoString,
}

static IS_GLEAM_PATH_PATTERN: OnceLock<Regex> = OnceLock::new();

impl GleamFile {
    pub fn new(dir: &Utf8Path, path: Utf8PathBuf) -> Self {
        Self {
            module_name: Self::module_name(&path, dir),
            path,
        }
    }

    /// Iterates over Gleam source files (`.gleam`) in a certain directory.
    /// Symlinks are followed.
    /// If the there is a .gleam file with a path that would be an
    /// invalid module name it should not be loaded. For example, if it
    /// has a uppercase letter in it.
    pub fn iterate_files_in_directory<'b>(
        io: &'b impl FileSystemReader,
        dir: &'b Utf8Path,
    ) -> impl Iterator<Item = Result<Self, crate::Warning>> + 'b {
        tracing::trace!("gleam_source_files {:?}", dir);
        files_with_extension(io, dir, "gleam").map(move |path| {
            if Self::is_gleam_path(&path, dir) {
                Ok(Self::new(dir, path))
            } else {
                Err(crate::Warning::InvalidSource { path })
            }
        })
    }

    pub fn cache_files(&self, artefact_directory: &Utf8Path) -> CacheFiles {
        CacheFiles::new(artefact_directory, &self.module_name)
    }

    fn module_name(path: &Utf8Path, dir: &Utf8Path) -> EcoString {
        // /path/to/project/_build/default/lib/the_package/src/my/module.gleam

        // my/module.gleam
        let mut module_path = path
            .strip_prefix(dir)
            .expect("Stripping package prefix from module path")
            .to_path_buf();

        // my/module
        let _ = module_path.set_extension("");

        // Stringify
        let name = module_path.to_string();

        // normalise windows paths
        name.replace("\\", "/").into()
    }

    fn is_gleam_path(path: &Utf8Path, dir: &Utf8Path) -> bool {
        IS_GLEAM_PATH_PATTERN
            .get_or_init(|| {
                Regex::new(&format!(
                    "^({module}{slash})*{module}\\.gleam$",
                    module = "[a-z][_a-z0-9]*",
                    slash = "(/|\\\\)",
                ))
                .expect("is_gleam_path() RE regex")
            })
            .is_match(
                path.strip_prefix(dir)
                    .expect("is_gleam_path(): strip_prefix")
                    .as_str(),
            )
    }
}

/// The collection of cache files paths related to a module.
/// These files are not guaranteed to exist.
pub struct CacheFiles {
    pub cache_path: Utf8PathBuf,
    pub meta_path: Utf8PathBuf,
}

impl CacheFiles {
    pub fn new(artefact_directory: &Utf8Path, module_name: &EcoString) -> Self {
        let file_name = module_name.replace("/", "@");
        let cache_path = artefact_directory
            .join(file_name.as_str())
            .with_extension("cache");
        let meta_path = artefact_directory
            .join(file_name.as_str())
            .with_extension("cache_meta");

        Self {
            cache_path,
            meta_path,
        }
    }

    pub fn delete(&self, io: &dyn FileSystemWriter) -> Result<()> {
        io.delete_file(&self.cache_path)?;
        io.delete_file(&self.meta_path)
    }

    /// Iterates over `.cache_meta` files in the given directory,
    /// and returns the respective module names.
    /// Symlinks are followed.
    pub fn modules_with_meta_files<'a>(
        io: &'a impl FileSystemReader,
        dir: &'a Utf8Path,
    ) -> impl Iterator<Item = EcoString> + 'a {
        tracing::trace!("CacheFiles::modules_with_meta_files {:?}", dir);
        files_with_extension(io, dir, "cache_meta").map(move |path| Self::module_name(dir, &path))
    }

    fn module_name(dir: &Utf8Path, path: &Utf8Path) -> EcoString {
        // /path/to/artefact/dir/my@module.cache_meta

        // my@module.cache_meta
        let mut module_path = path
            .strip_prefix(dir)
            .expect("Stripping package prefix from module path")
            .to_path_buf();

        // my@module
        let _ = module_path.set_extension("");

        // my/module
        module_path.to_string().replace("@", "/").into()
    }
}
