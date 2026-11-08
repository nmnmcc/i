// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

use std::collections::HashSet;

use camino::Utf8Path;
use ecow::EcoString;

use crate::{
    Error,
    build::{
        self, NullTelemetry, Outcome, PackageCompiler, StaleTracker, TargetCodegenConfiguration,
        package_compiler::Compiled,
    },
    config::PackageConfig,
    error::DefinedModuleOrigin,
    io::{FileSystemWriter, memory::InMemoryFileSystem},
    uid::UniqueIdGenerator,
    warning::WarningEmitter,
};

fn compile_modules(
    package_name: &str,
    module: &str,
    existing_modules: Vec<(&str, &str)>,
) -> Outcome<Compiled, Error> {
    let fs = InMemoryFileSystem::new();
    fs.write(
        Utf8Path::new(&format!("/src/{module}.i")),
        "pub fn main() -> Nil { Nil }",
    )
    .expect("write module");

    let mut config = PackageConfig::default();
    config.name = package_name.into();

    let compiler = PackageCompiler::new(
        &config,
        build::Mode::Dev,
        Utf8Path::new("/"),
        Utf8Path::new("/out"),
        &TargetCodegenConfiguration::Wasm,
        UniqueIdGenerator::new(),
        fs,
    );

    let mut already_defined_modules = existing_modules
        .into_iter()
        .map(|(package_name, module_name)| {
            (
                EcoString::from(module_name),
                DefinedModuleOrigin {
                    package_name: EcoString::from(package_name),
                    path: Utf8Path::new(&format!("{package_name}/{module_name}.i")).to_path_buf(),
                },
            )
        })
        .collect();

    compiler.compile(
        &WarningEmitter::null(),
        &mut imbl::HashMap::new(),
        &mut already_defined_modules,
        &mut StaleTracker::default(),
        &mut HashSet::new(),
        &NullTelemetry,
    )
}

#[test]
pub fn different_packages_defining_duplicate_module() {
    let output = compile_modules("a_package", "a_module", vec![("dep1", "a_module")])
        .into_result()
        .expect_err("should produce an error")
        .pretty_string();

    insta::assert_snapshot!(insta::internals::AutoName, output);
}

#[test]
pub fn same_package_defining_duplicate_module() {
    let output = compile_modules("a_package", "a_module", vec![("a_package", "a_module")])
        .into_result()
        .expect_err("should produce an error")
        .pretty_string();

    insta::assert_snapshot!(insta::internals::AutoName, output);
}

#[test]
fn wasm_module_names_do_not_reserve_erlang_runtime_namespace() {
    for name in ["code", "erlang", "lists", "user"] {
        let compiled = compile_modules("my_package", name, vec![])
            .into_result()
            .expect("Wasm modules do not share Erlang's global module namespace");
        assert_eq!(compiled.modules.len(), 1);
        assert_eq!(compiled.modules[0].name, name);
    }
}

#[test]
fn checking_a_package_writes_metadata_without_platform_artifacts() {
    let fs = InMemoryFileSystem::new();
    fs.write(Utf8Path::new("/src/app.i"), "pub fn main() { Nil }")
        .expect("write source");
    let mut config = PackageConfig::default();
    config.name = "app".into();
    let _ = PackageCompiler::new(
        &config,
        build::Mode::Lsp,
        Utf8Path::new("/"),
        Utf8Path::new("/out"),
        &TargetCodegenConfiguration::Wasm,
        UniqueIdGenerator::new(),
        fs.clone(),
    )
    .compile(
        &WarningEmitter::null(),
        &mut imbl::HashMap::new(),
        &mut imbl::HashMap::new(),
        &mut StaleTracker::default(),
        &mut HashSet::new(),
        &NullTelemetry,
    )
    .into_result()
    .expect("type check source");

    let files = fs.files();
    assert!(files.iter().any(|path| path.extension() == Some("cache")));
    assert!(
        files
            .iter()
            .all(|path| matches!(path.extension(), Some("i" | "cache" | "cache_meta")))
    );
}
