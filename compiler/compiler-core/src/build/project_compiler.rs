// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2020 The Gleam contributors

use crate::{
    Error, Result,
    analyse::TargetSupport,
    build::{
        Mode, Module, Package, Target, package_compiler::PackageCompiler,
        package_loader::StaleTracker, telemetry::Telemetry,
    },
    config::PackageConfig,
    dep_tree,
    error::DefinedModuleOrigin,
    io::{FileSystemReader, FileSystemWriter},
    manifest::{ManifestPackage, ManifestPackageSource},
    paths::ProjectPaths,
    type_::{self, ModuleFunction},
    uid::UniqueIdGenerator,
    version::COMPILER_VERSION,
    warning::{WarningEmitter, WarningEmitterIO},
};
use ecow::EcoString;
use hexpm::version::Version;
use itertools::Itertools;
use std::{
    cmp,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use super::{
    Codegen, Compile, Outcome,
    package_compiler::{CachedWarnings, Compiled},
};

use camino::Utf8PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub mode: Mode,
    pub target: Option<Target>,
    pub compile: Compile,
    pub codegen: Codegen,
    pub warnings_as_errors: bool,
    pub root_target_support: TargetSupport,
    pub no_print_progress: bool,
}

#[derive(Debug)]
pub struct Built {
    pub root_package: Package,
    pub module_interfaces: imbl::HashMap<EcoString, type_::ModuleInterface>,
}

impl Built {
    pub fn get_main_function(
        &self,
        module: &EcoString,
        target: Target,
    ) -> Result<ModuleFunction, Error> {
        match self.module_interfaces.get(module) {
            Some(module_data) => module_data.get_main_function(target),
            None => Err(Error::ModuleDoesNotExist {
                module: module.clone(),
                suggestion: None,
            }),
        }
    }

    pub fn minimum_required_version(&self) -> Version {
        self.module_interfaces
            .values()
            .map(|interface| &interface.minimum_required_version)
            .reduce(|one_version, other_version| cmp::max(one_version, other_version))
            .cloned()
            .unwrap_or(Version::new(0, 1, 0))
    }
}

#[derive(Debug)]
pub struct ProjectCompiler<IO> {
    // The i.toml config for the root package of the project
    pub config: PackageConfig,
    pub packages: HashMap<String, ManifestPackage>,
    importable_modules: imbl::HashMap<EcoString, type_::ModuleInterface>,
    pub(crate) defined_modules: imbl::HashMap<EcoString, DefinedModuleOrigin>,
    stale_modules: StaleTracker,
    /// The set of modules that have had partial compilation done since the last
    /// successful compilation.
    incomplete_modules: HashSet<EcoString>,
    warnings: WarningEmitter,
    telemetry: &'static dyn Telemetry,
    options: Options,
    paths: ProjectPaths,
    ids: UniqueIdGenerator,
    pub io: IO,
}

// TODO: test that tests cannot be imported into src
// TODO: test that dep cycles are not allowed between packages

impl<IO> ProjectCompiler<IO>
where
    IO: FileSystemWriter + FileSystemReader + Clone,
{
    pub fn new(
        config: PackageConfig,
        options: Options,
        packages: Vec<ManifestPackage>,
        telemetry: &'static dyn Telemetry,
        warning_emitter: Rc<dyn WarningEmitterIO>,
        paths: ProjectPaths,
        io: IO,
    ) -> Self {
        let packages = packages
            .into_iter()
            .map(|p| (p.name.to_string(), p))
            .collect();

        Self {
            importable_modules: imbl::HashMap::new(),
            defined_modules: imbl::HashMap::new(),
            stale_modules: StaleTracker::default(),
            incomplete_modules: HashSet::new(),
            ids: UniqueIdGenerator::new(),
            warnings: WarningEmitter::new(warning_emitter),
            telemetry,
            packages,
            options,
            config,
            paths,
            io,
        }
    }

    pub fn mode(&self) -> Mode {
        self.options.mode
    }

    pub fn target(&self) -> Target {
        self.options.target.unwrap_or(self.config.target)
    }

    pub fn reset_state_for_new_compile_run(&mut self) {
        // We make sure the stale module tracker is empty before we start, to
        // avoid mistakenly thinking a module is stale due to outdated state
        // from a previous build. A ProjectCompiler instance is re-used by the
        // LSP engine so state could be reused if we don't reset it.

        self.stale_modules.empty();

        // We also clear the defined modules, otherwise the language server
        // would start throwing errors for modules defined twice when compiling
        // a second time!
        self.defined_modules.clear();
    }

    fn retain_only_production_packages(&mut self) {
        let mut production = HashSet::new();
        let mut queue: Vec<_> = self.config.dependencies.keys().collect();
        while let Some(name) = queue.pop() {
            if production.insert(name.clone())
                && let Some(pkg) = self.packages.get(name.as_str())
            {
                queue.extend(pkg.requirements.iter());
            }
        }
        self.packages
            .retain(|name, _| production.contains(name.as_str()));
    }

    /// Compiles all packages in the project and returns the compiled
    /// information from the root package
    pub fn compile(mut self) -> Result<Built> {
        self.reset_state_for_new_compile_run();

        // In production mode, skip dev-only dependencies entirely so they
        // are never compiled.
        if self.mode() == Mode::Prod {
            self.retain_only_production_packages();
        }

        // Each package may specify an i version that it supports, so we
        // verify that this version is appropriate.
        self.check_i_version_and_build_configuration()?;

        // Dependencies are compiled first.
        let _ = self.compile_dependencies()?;

        // We reset the warning count as we don't want to fail the build if a
        // dependency has warnings, only if the root package does.
        self.warnings.reset_count();

        let root_package = self.compile_root_package().into_result()?;

        // TODO: test
        if self.options.warnings_as_errors && self.warnings.count() > 0 {
            return Err(Error::ForbiddenWarnings {
                count: self.warnings.count(),
            });
        }

        Ok(Built {
            root_package,
            module_interfaces: self.importable_modules,
        })
    }

    pub fn compile_root_package(&mut self) -> Outcome<Package, Error> {
        let config = self.config.clone();
        self.compile_i_package(&config, true, self.paths.root().to_path_buf())
            .map(
                |Compiled {
                     modules,
                     cached_module_names,
                 }| Package {
                    config,
                    modules,
                    cached_module_names,
                },
            )
    }

    /// Checks that version file found in the build directory matches the
    /// current version of gleam, and if build-impacting configuration
    /// options are the same. If not, we will clear the build directory
    /// before continuing. This will ensure that upgrading gleam will not
    /// leave one with confusing or hard to debug states, and changing
    /// config won't result in anything being missing.
    pub fn check_i_version_and_build_configuration(&self) -> Result<(), Error> {
        let build_path = self
            .paths
            .build_directory_for_target(self.mode(), self.target());
        let path = self
            .paths
            .build_configuration_fingerprint(self.mode(), self.target());

        let fingerprint = format!(
            "ilang-frontend-v2 {COMPILER_VERSION} wasm {}",
            env!("ILANG_CORE_REVISION")
        );

        if self.io.is_file(&path) {
            let previous = self.io.read(&path)?;
            if previous == fingerprint {
                return Ok(());
            }
        }

        // Either file is missing our the config does not match. Time to rebuild
        tracing::info!("removing_build_state_for_differing_configuration");
        self.io.delete_directory(&build_path)?;

        // Recreate build directory with new updated version file
        self.io.mkdir(&build_path)?;
        self.io.write(&path, &fingerprint)
    }

    pub fn compile_dependencies(&mut self) -> Result<Vec<Module>, Error> {
        assert!(
            self.stale_modules.is_empty(),
            "The project compiler stale tracker was not emptied from the previous compilation"
        );

        let sequence = order_packages(&self.packages)?;
        let mut modules = vec![];

        for name in sequence {
            let compiled = self.load_cache_or_compile_package(&name)?;
            modules.extend(compiled);
        }

        Ok(modules)
    }

    fn load_cache_or_compile_package(&mut self, name: &str) -> Result<Vec<Module>, Error> {
        // TODO: We could remove this clone if we split out the compilation of
        // packages into their own classes and then only mutate self after we no
        // longer need to have the package borrowed from self.packages.
        let package = self.packages.get(name).expect("Missing package").clone();
        let _ = usable_build_tools(&package)?;
        let result = self.compile_i_dep_package(&package);

        // TODO: test. This one is not covered by the integration tests.
        if result.is_err() {
            tracing::debug!(package=%name, "removing_failed_build");
            let path = self.paths.build_directory_for_package(
                self.mode(),
                self.target(),
                package.application_name(),
            );
            self.io.delete_directory(&path)?;
        }

        result
    }

    fn compile_i_dep_package(
        &mut self,
        package: &ManifestPackage,
    ) -> Result<Vec<Module>, Error> {
        // TODO: Test
        let package_root = match &package.source {
            // If the path is relative it is relative to the root of the
            // project, not to the current working directory. The language server
            // could have the working directory and the project root in different
            // places.
            ManifestPackageSource::Local { path } if path.is_relative() => {
                self.io.canonicalise(&self.paths.root().join(path))?
            }

            // If the path is absolute we can use it as-is.
            ManifestPackageSource::Local { path } => path.clone(),

            ManifestPackageSource::Git { .. } | ManifestPackageSource::Hex { .. } => {
                return Err(Error::UnsupportedBuildTool {
                    package: package.name.to_string(),
                    build_tools: vec!["iLang requires local path dependencies".into()],
                });
            }
        };
        let config_path = package_root.join("i.toml");
        let config = PackageConfig::read(config_path, &self.io)?;
        self.compile_i_package(&config, false, package_root)
            .into_result()
            .map(|compiled| compiled.modules)
    }

    fn compile_i_package(
        &mut self,
        config: &PackageConfig,
        is_root: bool,
        root_path: Utf8PathBuf,
    ) -> Outcome<Compiled, Error> {
        // The bundled standard library is an implicit dependency of every
        // user package, including local packages checked by the language server.
        let mut config = config.clone();
        if config.name != crate::STDLIB_PACKAGE_NAME
            && let Some(stdlib) = self.packages.get(crate::STDLIB_PACKAGE_NAME)
            && let ManifestPackageSource::Local { path } = &stdlib.source
        {
            let path = if path.is_relative() {
                self.paths.root().join(path)
            } else {
                path.clone()
            };
            let _ = config
                .dependencies
                .entry(crate::STDLIB_PACKAGE_NAME.into())
                .or_insert(crate::requirement::Requirement::Path { path });
        }
        let config = &config;
        let out_path =
            self.paths
                .build_directory_for_package(self.mode(), self.target(), &config.name);
        let mode = if is_root { self.mode() } else { Mode::Prod };
        let target = super::TargetCodegenConfiguration::Wasm;

        let mut compiler = PackageCompiler::new(
            config,
            mode,
            &root_path,
            &out_path,
            &target,
            self.ids.clone(),
            self.io.clone(),
        );
        compiler.write_metadata = true;
        compiler.compile_modules = !(self.options.compile == Compile::DepsOnly && is_root);
        compiler.target_support = if is_root {
            // When compiling the root package it is context specific as to whether we need to
            // enforce that all functions have an implementation for the current target.
            // Typically we do, but if we are using `gleam run -m $module` to run a module that
            // belongs to a dependency we don't need to enforce this as we don't want to fail
            // compilation. It's impossible for a dependecy module to call functions from the root
            // package, so it's OK if they could not be compiled.
            self.options.root_target_support
        } else {
            // When compiling dependencies we don't enforce that all functions have an
            // implementation for the current target. It is OK if they have APIs that are
            // unaccessible so long as they are not used by the root package.
            TargetSupport::NotEnforced
        };
        compiler.cached_warnings = if is_root {
            CachedWarnings::Use
        } else {
            CachedWarnings::Ignore
        };

        // Analyse source modules and write reusable type metadata.
        compiler.compile(
            &self.warnings,
            &mut self.importable_modules,
            &mut self.defined_modules,
            &mut self.stale_modules,
            &mut self.incomplete_modules,
            self.telemetry,
        )
    }
}

impl<IO> ProjectCompiler<IO> {
    pub fn get_importable_modules(&self) -> &imbl::HashMap<EcoString, type_::ModuleInterface> {
        &self.importable_modules
    }
}

fn order_packages(packages: &HashMap<String, ManifestPackage>) -> Result<Vec<EcoString>, Error> {
    dep_tree::toposort_deps(
        packages
            .values()
            // Making sure that the package order is deterministic, to prevent different
            // compilations of the same project compiling in different orders. This could impact
            // any bugged outcomes, though not any where the compiler is working correctly, so it's
            // mostly to aid debugging.
            .sorted_by(|a, b| a.name.cmp(&b.name))
            .map(|package| (package.name.as_str().into(), package.requirements.to_vec()))
            .collect(),
    )
    .map_err(convert_deps_tree_error)
}

fn convert_deps_tree_error(e: dep_tree::Error) -> Error {
    match e {
        dep_tree::Error::Cycle(packages) => Error::PackageCycle { packages },
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum BuildTool {
    I,
}

/// iLang packages are checked as i source. Foreign build tools cannot
/// provide a Wasm implementation and must be migrated to local i packages.
pub(crate) fn usable_build_tools(package: &ManifestPackage) -> Result<Vec<BuildTool>, Error> {
    if !package.build_tools.is_empty() && package.build_tools.iter().all(|tool| tool == "i") {
        return Ok(vec![BuildTool::I]);
    }
    Err(Error::UnsupportedBuildTool {
        package: package.name.to_string(),
        build_tools: package.build_tools.clone(),
    })
}
