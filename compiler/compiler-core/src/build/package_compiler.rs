// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2020 The Gleam contributors

#[cfg(test)]
mod tests;

use crate::analyse::TargetSupport;
use crate::ast::TypedModule;
use crate::build::package_loader::CacheFiles;

use crate::error::{DefinedModuleOrigin, FailedModule, SkipReason, SkippedModule};

use crate::metadata;
use crate::requirement::Requirement;
use crate::type_::PRELUDE_MODULE_NAME;
use crate::type_::printer::Names;
use crate::{
    Error, Result, Warning,
    ast::UntypedModule,
    build::{
        Mode, Module, Origin, Outcome, SourceFingerprint, Target,
        package_loader::{PackageLoader, StaleTracker},
    },
    config::PackageConfig,
    io::{FileSystemReader, FileSystemWriter},
    parse::extra::ModuleExtra,
    paths, type_,
    uid::UniqueIdGenerator,
    warning::{TypeWarningEmitter, WarningEmitter},
};
use ecow::EcoString;
use src_span::{LineNumbers, SrcSpan};
use std::{
    collections::{HashMap, HashSet},
    time::SystemTime,
};

use camino::{Utf8Path, Utf8PathBuf};

use super::{TargetCodegenConfiguration, Telemetry};

#[derive(Debug)]
pub struct Compiled {
    /// The modules which were just compiled
    pub modules: Vec<Module>,
    /// The names of all cached modules, which are not present in the `modules` field.
    pub cached_module_names: Vec<EcoString>,
}

#[derive(Debug)]
pub struct PackageCompiler<'a, IO> {
    pub io: IO,
    pub out: &'a Utf8Path,
    pub root: &'a Utf8Path,
    pub mode: Mode,
    pub target: &'a TargetCodegenConfiguration,
    pub config: &'a PackageConfig,
    pub ids: UniqueIdGenerator,
    pub write_metadata: bool,
    /// If set to false the compiler won't load and analyse any of the package's
    /// modules and always succeed compilation returning no compile modules.
    ///
    pub compile_modules: bool,
    pub target_support: TargetSupport,
    pub cached_warnings: CachedWarnings,
}

impl<'a, IO> PackageCompiler<'a, IO>
where
    IO: FileSystemReader + FileSystemWriter + Clone,
{
    pub fn new(
        config: &'a PackageConfig,
        mode: Mode,
        root: &'a Utf8Path,
        out: &'a Utf8Path,
        target: &'a TargetCodegenConfiguration,
        ids: UniqueIdGenerator,
        io: IO,
    ) -> Self {
        Self {
            io,
            ids,
            out,
            root,
            mode,
            config,
            target,
            write_metadata: true,
            compile_modules: true,
            target_support: TargetSupport::NotEnforced,
            cached_warnings: CachedWarnings::Ignore,
        }
    }

    /// Compile the package.
    /// Returns a list of modules that were compiled. Any modules that were read
    /// from the cache will not be returned.
    // TODO: return the cached modules.
    pub fn compile(
        mut self,
        warnings: &WarningEmitter,
        existing_modules: &mut imbl::HashMap<EcoString, type_::ModuleInterface>,
        already_defined_modules: &mut imbl::HashMap<EcoString, DefinedModuleOrigin>,
        stale_modules: &mut StaleTracker,
        incomplete_modules: &mut HashSet<EcoString>,
        telemetry: &dyn Telemetry,
    ) -> Outcome<Compiled, Error> {
        let span = tracing::info_span!("compile", package = %self.config.name.as_str());
        let _enter = span.enter();

        // Ensure that the package is compatible with this version of i
        if let Err(error) = self.config.check_i_compatibility() {
            return error.into();
        }

        let artefact_directory = self.out.join(paths::ARTEFACT_DIRECTORY_NAME);

        let loader = PackageLoader::new(
            self.io.clone(),
            self.ids.clone(),
            self.mode,
            self.root,
            self.cached_warnings,
            warnings,
            &artefact_directory,
            self.target.target(),
            &self.config.name,
            stale_modules,
            already_defined_modules,
            incomplete_modules,
        );

        let loaded = if self.compile_modules {
            match loader.run() {
                Ok(loaded) => loaded,
                Err(error) => return error.into(),
            }
        } else {
            Loaded::empty()
        };

        let mut cached_module_names = Vec::new();

        // Load the cached modules that have previously been compiled
        for module in loaded.cached.into_iter() {
            // Emit any cached warnings.
            // Note that `self.cached_warnings` is set to `Ignore` (such as for
            // dependency packages) then this field will not be populated.
            if let Err(error) = self.emit_warnings(warnings, &module) {
                return error.into();
            }

            cached_module_names.push(module.name.clone());

            // Register the cached module so its type information etc can be
            // used for compiling futher modules.
            _ = existing_modules.insert(module.name.clone(), module);
        }

        if !loaded.to_compile.is_empty() {
            // Print that work is being done
            telemetry.checking_package(&self.config.name);
        }

        // Type check the modules that are new or have changed
        tracing::info!(count=%loaded.to_compile.len(), "analysing_modules");
        let outcome =
            PackageModulesAnalyser::new(&self, warnings, existing_modules, incomplete_modules)
                .analyse(loaded.to_compile);

        let mut modules = match outcome {
            Outcome::Ok(modules) => modules,
            Outcome::PartialFailure(modules, errors) => {
                return Outcome::PartialFailure(
                    Compiled {
                        modules,
                        cached_module_names,
                    },
                    errors,
                );
            }
            Outcome::TotalFailure(errors) => return Outcome::TotalFailure(errors),
        };

        if let Err(error) = self.encode_and_write_metadata(&mut modules) {
            return error.into();
        }

        Outcome::Ok(Compiled {
            modules,
            cached_module_names,
        })
    }

    fn encode_and_write_metadata(&mut self, modules: &mut [Module]) -> Result<()> {
        if !self.write_metadata {
            tracing::debug!("package_metadata_writing_disabled");
            return Ok(());
        }
        if modules.is_empty() {
            return Ok(());
        }

        let artefact_dir = self.out.join(paths::ARTEFACT_DIRECTORY_NAME);

        tracing::debug!("writing_module_caches");
        for module in modules {
            let cache_files = CacheFiles::new(&artefact_dir, &module.name);

            let result = if self.cached_warnings.should_use() {
                metadata::encode(&module.ast.type_info)
            } else {
                // Dependency packages don't get warnings persisted as the
                // programmer doesn't want to be told every time about warnings they
                // cannot fix directly.
                let warnings = std::mem::take(&mut module.ast.type_info.warnings);
                let result = metadata::encode(&module.ast.type_info);
                module.ast.type_info.warnings = warnings;
                result
            };

            // Write cache file
            let bytes = result.expect("Failed to serialise module cache");
            self.io.write_bytes(&cache_files.cache_path, &bytes)?;

            // Write cache metadata
            let info = CacheMetadata {
                mtime: module.mtime,
                dependencies: module.dependencies.clone(),
                fingerprint: SourceFingerprint::new(&module.code),
                line_numbers: module.ast.type_info.line_numbers.clone(),
            };
            self.io
                .write_bytes(&cache_files.meta_path, &info.to_binary())?;
        }
        Ok(())
    }

    fn emit_warnings(
        &self,
        warnings: &WarningEmitter,
        module: &type_::ModuleInterface,
    ) -> Result<()> {
        for warning in &module.warnings {
            let src = self.io.read(&module.src_path)?;
            warnings.emit(Warning::Type {
                path: module.src_path.clone(),
                src: src.into(),
                warning: Box::new(warning.clone()),
            });
        }

        Ok(())
    }
}

/// A structure we use to hold data about used to analyse the packages of a
/// module.
struct PackageModulesAnalyser<'a> {
    package_config: &'a PackageConfig,
    target_support: TargetSupport,
    target: Target,
    ids: &'a UniqueIdGenerator,
    warnings: &'a WarningEmitter,

    module_interfaces: &'a mut imbl::HashMap<EcoString, type_::ModuleInterface>,
    incomplete_modules: &'a mut HashSet<EcoString>,

    /// The direct dependencies of this package, as needed by the
    /// `ModuleAnalyzerConstructor`.
    direct_dependencies: HashMap<EcoString, Requirement>,

    /// The dev dependencies of this package, as needed by the
    /// `ModuleAnalyzerConstructor`.
    dev_dependencies: HashSet<EcoString>,

    /// Keeps track of the modules whose analysis had to be skipped because
    /// other modules they depend on failed to compile.
    skipped_modules: HashMap<EcoString, SkippedModule>,
    /// Keeps track of the modules whose compilation failed because of some
    /// error.
    failed_modules: HashMap<EcoString, FailedModule>,
    /// Keeps track of the modules that could be analysed successfully.
    analysed_modules: Vec<Module>,
}

impl<'a> PackageModulesAnalyser<'a> {
    pub fn new<'package_compiler, IO>(
        package_compiler: &'a PackageCompiler<'package_compiler, IO>,
        warnings: &'a WarningEmitter,
        module_interfaces: &'a mut imbl::HashMap<EcoString, type_::ModuleInterface>,
        incomplete_modules: &'a mut HashSet<EcoString>,
    ) -> Self {
        let direct_dependencies = package_compiler
            .config
            .dependencies_for(package_compiler.mode)
            .expect("Package deps");

        let dev_dependencies = package_compiler
            .config
            .dev_dependencies
            .keys()
            .cloned()
            .collect();

        Self {
            package_config: package_compiler.config,
            target_support: package_compiler.target_support,
            target: package_compiler.target.target(),
            ids: &package_compiler.ids,
            warnings,
            module_interfaces,
            incomplete_modules,

            direct_dependencies,
            dev_dependencies,

            skipped_modules: HashMap::new(),
            failed_modules: HashMap::new(),
            analysed_modules: Vec::new(),
        }
    }

    pub fn analyse(
        mut self,
        uncompiled_modules: Vec<UncompiledModule>,
    ) -> Outcome<Vec<Module>, Error> {
        // Insert the prelude
        // DUPE: preludeinsertion
        // TODO: Currently we do this here and also in the tests. It would be better
        // to have one place where we create all this required state for use in each
        // place.
        let _ = self
            .module_interfaces
            .insert(PRELUDE_MODULE_NAME.into(), type_::build_prelude(self.ids));

        for uncompiled_module in uncompiled_modules {
            let UncompiledModule {
                name,
                code,
                ast,
                path,
                mtime,
                origin,
                dependencies,
                extra,
                ..
            } = uncompiled_module;

            tracing::debug!(module = ?name, "Type checking");

            // We first need to check if the module can actually be compiled.
            // If we weren't able to compile one of the modules it depends on, then
            // we have to skip this one to avoid reporting false errors.
            if self.skip_if_erroring_dependencies(
                path.clone(),
                name.clone(),
                code.clone(),
                &dependencies,
            ) {
                continue;
            }

            match self.infer_module(path.clone(), code.clone(), origin, ast) {
                // In case of success we register the typed module and its
                // interface, making sure the module is no longer considered
                // incomplete.
                Outcome::Ok(ast) => {
                    let mut module = Module {
                        name,
                        code,
                        input_path: path,
                        mtime,
                        origin,
                        dependencies,
                        extra,
                        ast,
                    };
                    module.attach_doc_and_module_comments();

                    let _ = self.incomplete_modules.remove(&module.name);
                    self.register_module_interface(&module);
                    self.analysed_modules.push(module);
                }

                // If there's some errors in the module we mark it as incomplete
                // (so it won't be reloaded from the cache) and register the
                // failure.
                // We still register the successfully type checked interface so
                // that it can be used by the language server, even in case of
                // errors.
                Outcome::PartialFailure(ast, errors) => {
                    let names = ast.names.clone();
                    let mut module = Module {
                        name: name.clone(),
                        code: code.clone(),
                        input_path: path.clone(),
                        mtime,
                        origin,
                        dependencies,
                        extra,
                        ast,
                    };
                    module.attach_doc_and_module_comments();

                    let _ = self.incomplete_modules.insert(module.name.clone());
                    self.register_failed_module(path, name, code, names, errors);
                    self.register_module_interface(&module);
                    self.analysed_modules.push(module);
                }

                // In case of a total failure the module interface could not be
                // built at all! We just record that analysis failed.
                Outcome::TotalFailure(errors) => {
                    self.register_failed_module(path, name, code, Names::new(), errors);
                }
            };
        }

        self.into_outcome()
    }

    /// After typing all the modules, turn this struct into an outcome, based
    /// on the error we've ran into during the analysis of the given modules.
    fn into_outcome(self) -> Outcome<Vec<Module>, Error> {
        // There's three possible outcomes:
        // - all modules could be analysed, success
        // - no module could be analysed, total failure
        // - there's a mix of analysed and failed modules, partial failure

        // All modules could be analysed: there's no failure at all.
        if self.failed_modules.is_empty() {
            return Outcome::Ok(self.analysed_modules);
        }

        // No module could be analysed.
        if self.analysed_modules.is_empty() {
            return Outcome::TotalFailure(Error::Type {
                skipped_modules: self.skipped_modules.into_values().collect(),
                failed_modules: self.failed_modules,
            });
        }

        // There's a mix of analysed and failed modules.
        Outcome::PartialFailure(
            self.analysed_modules,
            Error::Type {
                skipped_modules: self.skipped_modules.into_values().collect(),
                failed_modules: self.failed_modules,
            },
        )
    }

    /// This adds the compiled module's interface to all the module interfaces
    /// so that its types can be imported into other modules.
    ///
    /// This also cheks the module is not empty and raises a warning if it is.
    fn register_module_interface(&mut self, module: &Module) {
        // Emit the empty module warning if the module has no definitions at all.
        // Modules with only private definitions already emit their own warnings.
        let interface = &module.ast.type_info;
        if interface.values.is_empty() && interface.types.is_empty() {
            self.warnings.emit(Warning::EmptyModule {
                path: module.input_path.clone(),
                name: module.name.clone(),
            });
        }

        let _ = self
            .module_interfaces
            .insert(module.name.clone(), interface.clone());
    }

    /// Registers the given module has failed with some errors.
    fn register_failed_module(
        &mut self,
        path: Utf8PathBuf,
        name: EcoString,
        code: EcoString,
        names: Names,
        errors: vec1::Vec1<type_::Error>,
    ) {
        let _ = self.failed_modules.insert(
            name,
            FailedModule {
                names: Box::new(names),
                path,
                src: code,
                errors,
            },
        );
    }

    /// Performs type inference for the given untyped module.
    fn infer_module(
        &self,
        path: Utf8PathBuf,
        code: EcoString,
        origin: Origin,
        module_ast: UntypedModule,
    ) -> Outcome<TypedModule, vec1::Vec1<type_::Error>> {
        let line_numbers = LineNumbers::new(&code);
        crate::analyse::ModuleAnalyzerConstructor {
            target: self.target,
            ids: self.ids,
            origin,
            importable_modules: self.module_interfaces,
            warnings: &TypeWarningEmitter::new(path.clone(), code, self.warnings.clone()),
            direct_dependencies: &self.direct_dependencies,
            dev_dependencies: &self.dev_dependencies,
            target_support: self.target_support,
            package_config: self.package_config,
        }
        .infer_module(module_ast, line_numbers, path)
    }

    /// Given an uncompiled module, this goes through the modules it depends on
    /// to see if any of them was skipped because of errors.
    /// If the module should be skipped because of this, it is marked as skipped
    /// and this will return `true`.
    ///
    fn skip_if_erroring_dependencies(
        &mut self,
        path: Utf8PathBuf,
        name: EcoString,
        code: EcoString,
        dependencies: &[(EcoString, SrcSpan)],
    ) -> bool {
        let reason_to_skip = dependencies
            .iter()
            .find_map(|(dependency, import_location)| {
                if self.failed_modules.contains_key(dependency) {
                    // We import a failing module directly, this module must be
                    // skipped too.
                    let reason = SkipReason::DependencyHasError {
                        name: dependency.clone(),
                    };
                    Some((*import_location, reason))
                } else if let Some(skipped_module) = self.skipped_modules.get(dependency) {
                    // We import a module that's not directly failing, but had
                    // to be skipped. This module must be skipped too!
                    let reason = SkipReason::DependencyWasSkipped {
                        name: dependency.clone(),
                        erroring_module: skipped_module.reason.erroring_module(),
                    };
                    Some((*import_location, reason))
                } else {
                    None
                }
            });

        match reason_to_skip {
            None => false,
            Some((location, reason)) => {
                let _ = self.skipped_modules.insert(
                    name.clone(),
                    SkippedModule {
                        path,
                        name,
                        code,
                        location,
                        reason,
                    },
                );
                true
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum Input {
    New { module: Box<UncompiledModule> },
    Cached { module: CachedModule },
}

impl Input {
    pub fn name(&self) -> &EcoString {
        match self {
            Input::New { module } => &module.name,
            Input::Cached { module } => &module.name,
        }
    }

    pub fn source_path(&self) -> &Utf8Path {
        match self {
            Input::New { module } => &module.path,
            Input::Cached { module } => &module.source_path,
        }
    }

    pub fn dependencies(&self) -> Vec<EcoString> {
        match self {
            Input::New { module } => module
                .dependencies
                .iter()
                .map(|(name, _)| name.clone())
                .collect(),
            Input::Cached { module } => module
                .dependencies
                .iter()
                .map(|(name, _)| name.clone())
                .collect(),
        }
    }

    /// Returns `true` if the input is [`New`].
    ///
    /// [`New`]: Input::New
    #[cfg(test)]
    #[must_use]
    pub(crate) fn is_new(&self) -> bool {
        matches!(self, Self::New { .. })
    }

    /// Returns `true` if the input is [`Cached`].
    ///
    /// [`Cached`]: Input::Cached
    #[cfg(test)]
    #[must_use]
    pub(crate) fn is_cached(&self) -> bool {
        matches!(self, Self::Cached { .. })
    }
}

#[derive(Debug)]
pub(crate) struct CachedModule {
    pub name: EcoString,
    pub origin: Origin,
    pub dependencies: Vec<(EcoString, SrcSpan)>,
    pub source_path: Utf8PathBuf,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct CacheMetadata {
    pub mtime: SystemTime,
    pub dependencies: Vec<(EcoString, SrcSpan)>,
    pub fingerprint: SourceFingerprint,
    pub line_numbers: LineNumbers,
}

impl CacheMetadata {
    pub fn to_binary(&self) -> Vec<u8> {
        bitcode::serialize(self).expect("Serializing cache info")
    }

    pub fn from_binary(bytes: &[u8]) -> Result<Self, ()> {
        match bitcode::deserialize(bytes) {
            Ok(data) => Ok(data),
            Err(_) => Err(()),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Loaded {
    pub to_compile: Vec<UncompiledModule>,
    pub cached: Vec<type_::ModuleInterface>,
}

impl Loaded {
    fn empty() -> Self {
        Self {
            to_compile: vec![],
            cached: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UncompiledModule {
    pub path: Utf8PathBuf,
    pub name: EcoString,
    pub code: EcoString,
    pub mtime: SystemTime,
    pub origin: Origin,
    pub package: EcoString,
    pub dependencies: Vec<(EcoString, SrcSpan)>,
    pub ast: UntypedModule,
    pub extra: ModuleExtra,
}

#[derive(Debug, Clone, Copy)]
pub enum CachedWarnings {
    Use,
    Ignore,
}
impl CachedWarnings {
    pub(crate) fn should_use(&self) -> bool {
        match self {
            CachedWarnings::Use => true,
            CachedWarnings::Ignore => false,
        }
    }
}
