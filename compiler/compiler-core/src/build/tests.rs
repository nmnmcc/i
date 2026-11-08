// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2022 The Gleam contributors

use crate::{Error, manifest::ManifestPackage};

use super::project_compiler::{BuildTool, usable_build_tools};

#[test]
fn usable_build_tool_unknown() {
    assert_eq!(
        usable_build_tools(&ManifestPackage::default().with_build_tools(&["unknown"])),
        Err(Error::UnsupportedBuildTool {
            package: "".into(),
            build_tools: vec!["unknown".into()],
        })
    )
}

#[test]
fn usable_build_tool_none() {
    assert_eq!(
        usable_build_tools(&ManifestPackage::default()),
        Err(Error::UnsupportedBuildTool {
            package: "".into(),
            build_tools: vec![],
        })
    )
}

#[test]
fn foreign_mix_package_requires_migration() {
    assert_eq!(
        usable_build_tools(&ManifestPackage::default().with_build_tools(&["mix"])),
        Err(Error::UnsupportedBuildTool {
            package: "".into(),
            build_tools: vec!["mix".into()]
        })
    )
}

#[test]
fn foreign_rebar_package_requires_migration() {
    assert_eq!(
        usable_build_tools(&ManifestPackage::default().with_build_tools(&["rebar3"])),
        Err(Error::UnsupportedBuildTool {
            package: "".into(),
            build_tools: vec!["rebar3".into()]
        })
    )
}

#[test]
fn usable_build_tool_only_i() {
    assert_eq!(
        usable_build_tools(&ManifestPackage::default().with_build_tools(&["i"])),
        Ok(vec![BuildTool::I])
    )
}

#[test]
fn foreign_mix_and_rebar_package_requires_migration() {
    assert_eq!(
        usable_build_tools(&ManifestPackage::default().with_build_tools(&["mix", "rebar3"])),
        Err(Error::UnsupportedBuildTool {
            package: "".into(),
            build_tools: vec!["mix".into(), "rebar3".into()]
        })
    )
}

#[test]
fn i_package_with_foreign_build_steps_requires_migration() {
    assert_eq!(
        usable_build_tools(&ManifestPackage::default().with_build_tools(&["i", "rebar3"])),
        Err(Error::UnsupportedBuildTool {
            package: "".into(),
            build_tools: vec!["i".into(), "rebar3".into()],
        })
    )
}

#[test]
fn local_dependencies_can_import_the_implicit_standard_library() {
    use std::rc::Rc;

    use camino::Utf8Path;

    use crate::{
        analyse::TargetSupport,
        config::PackageConfig,
        io::{FileSystemWriter, memory::InMemoryFileSystem},
        manifest::ManifestPackageSource,
        paths::ProjectPaths,
        requirement::Requirement,
        warning::NullWarningEmitterIO,
    };

    use super::{Codegen, Compile, Mode, NullTelemetry, Options, ProjectCompiler, Target};

    let io = InMemoryFileSystem::new();
    for (path, contents) in [
        ("/stdlib/i.toml", "name = \"i_stdlib\"\nversion = \"1.0.5\""),
        ("/stdlib/src/i/example.i", "pub fn identity(x) { x }"),
        ("/dep/i.toml", "name = \"local_dep\"\nversion = \"1.0.0\""),
        (
            "/dep/src/local_dep.i",
            "import i/example\npub fn value() { example.identity(42) }",
        ),
        (
            "/app/src/app.i",
            "import local_dep\npub fn main() { local_dep.value() }",
        ),
    ] {
        io.write(Utf8Path::new(path), contents)
            .expect("write fixture");
    }
    let mut config = PackageConfig::default();
    config.name = "app".into();
    let _ = config.dependencies.insert(
        "local_dep".into(),
        Requirement::Path {
            path: "/dep".into(),
        },
    );
    let stdlib = ManifestPackage {
        name: "i_stdlib".into(),
        source: ManifestPackageSource::Local {
            path: "/stdlib".into(),
        },
        build_tools: vec!["i".into()],
        ..ManifestPackage::default()
    };
    let dependency = ManifestPackage {
        name: "local_dep".into(),
        source: ManifestPackageSource::Local {
            path: "/dep".into(),
        },
        build_tools: vec!["i".into()],
        requirements: vec!["i_stdlib".into()],
        ..ManifestPackage::default()
    };
    let built = ProjectCompiler::new(
        config,
        Options {
            mode: Mode::Lsp,
            target: Some(Target::Wasm),
            compile: Compile::All,
            codegen: Codegen::None,
            warnings_as_errors: true,
            root_target_support: TargetSupport::Enforced,
            no_print_progress: true,
        },
        vec![stdlib, dependency],
        &NullTelemetry,
        Rc::new(NullWarningEmitterIO),
        ProjectPaths::new("/app".into()),
        io,
    )
    .compile()
    .expect("implicit standard library is visible from local dependencies");

    assert!(built.module_interfaces.contains_key("i/example"));
    assert!(built.module_interfaces.contains_key("local_dep"));
}

#[test]
fn checking_cache_schema_change_invalidates_old_artifacts() {
    use std::rc::Rc;

    use crate::{
        analyse::TargetSupport,
        config::PackageConfig,
        io::{FileSystemReader, FileSystemWriter, memory::InMemoryFileSystem},
        paths::ProjectPaths,
        warning::NullWarningEmitterIO,
    };

    use super::{Codegen, Compile, Mode, NullTelemetry, Options, ProjectCompiler, Target};

    let io = InMemoryFileSystem::new();
    let paths = ProjectPaths::new("/app".into());
    let fingerprint = paths.build_configuration_fingerprint(Mode::Lsp, Target::Wasm);
    let artifact = paths
        .build_directory_for_target(Mode::Lsp, Target::Wasm)
        .join("old.cache_meta");
    io.write(&fingerprint, "ilang-frontend-v1 1.19.2 wasm")
        .expect("write previous schema fingerprint");
    io.write(&artifact, "previous cache schema")
        .expect("write previous schema cache");

    let compiler = ProjectCompiler::new(
        PackageConfig {
            name: "app".into(),
            ..PackageConfig::default()
        },
        Options {
            mode: Mode::Lsp,
            target: Some(Target::Wasm),
            compile: Compile::All,
            codegen: Codegen::None,
            warnings_as_errors: false,
            root_target_support: TargetSupport::Enforced,
            no_print_progress: true,
        },
        vec![],
        &NullTelemetry,
        Rc::new(NullWarningEmitterIO),
        paths,
        io.clone(),
    );
    compiler
        .check_i_version_and_build_configuration()
        .expect("invalidate incompatible metadata");
    assert!(!io.is_file(&artifact));
    assert_eq!(
        io.read(&fingerprint)
            .expect("read current compiler fingerprint"),
        format!(
            "ilang-frontend-v2 {} wasm {}",
            env!("CARGO_PKG_VERSION"),
            env!("ILANG_CORE_REVISION")
        )
    );

    io.write(&artifact, "current cache schema")
        .expect("write current schema cache");
    compiler
        .check_i_version_and_build_configuration()
        .expect("reuse compatible metadata");
    assert!(io.is_file(&artifact));

    io.write(
        &fingerprint,
        &format!(
            "ilang-frontend-v2 {} wasm previous-compiler-revision",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("write a different compiler revision with the same schema and version");
    compiler
        .check_i_version_and_build_configuration()
        .expect("invalidate metadata produced by a different compiler implementation");
    assert!(!io.is_file(&artifact));
}
