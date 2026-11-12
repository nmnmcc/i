//! Loading the package graph.

use ilang_package::{ModuleKind, code, load};
use std::fs;
use std::path::Path;

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let p = dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
}

#[test]
fn source_entries_do_not_load_test_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            (
                "i.toml",
                "name = \"app\"\n[entries]\nrun = \"app.run\"\n[dev-dependencies]\nmissing = { path = \"missing\" }\n",
            ),
            ("src/app.i", "run = () { Nil }\n"),
            ("test/broken.i", "not i source\n"),
            ("test/app.i", "also not i source\n"),
        ],
    );
    let options = ilang_package::LoadOptions {
        tests: false,
        ..Default::default()
    };
    let (graph, diags) = ilang_package::load_with(dir.path(), &options);
    assert!(diags.is_empty(), "{diags:?}");
    let root = &graph.packages[graph.root.unwrap()];
    assert_eq!(
        root.modules
            .iter()
            .map(|m| m.path.as_str())
            .collect::<Vec<_>>(),
        ["app/app"]
    );
}

#[test]
fn test_module_entries_load_test_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            (
                "i.toml",
                "name = \"app\"\n[entries]\nrun = \"worker.run\"\n[dev-dependencies]\ndep = { path = \"dep\" }\n",
            ),
            ("test/worker.i", "run = () { Nil }\n"),
            ("dep/i.toml", "name = \"dep\"\n"),
            ("dep/src/value.i", "value = 1\n"),
        ],
    );
    let options = ilang_package::LoadOptions {
        tests: false,
        ..Default::default()
    };
    let (graph, diags) = ilang_package::load_with(dir.path(), &options);
    assert!(diags.is_empty(), "{diags:?}");
    let root = &graph.packages[graph.root.unwrap()];
    assert!(
        root.modules
            .iter()
            .any(|m| m.path == "app/worker" && m.kind == ModuleKind::Test)
    );
    assert!(
        root.dev_dependencies
            .iter()
            .any(|(name, id)| name == "dep" && id.is_some())
    );
}

#[test]
fn graph_with_path_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            (
                "app/i.toml",
                "name = \"app\"\n[dependencies]\nu = { path = \"../util\" }\nv = { path = \"../util\" }\n[dev-dependencies]\nt = { path = \"../tools\" }\n",
            ),
            ("app/src/main.i", "x = 1\n"),
            ("app/src/a/b.i", "x = 1\n"),
            ("app/test/t.i", "x = 1\n"),
            (
                "util/i.toml",
                "name = \"util\"\n[dev-dependencies]\nzzz = { path = \"../nothing\" }\n",
            ),
            ("util/src/lib.i", "x = 1\n"),
            ("util/test/lib_test.i", "x = 1\n"),
            ("tools/i.toml", "name = \"tools\"\n"),
        ],
    );
    let (graph, diags) = load(&dir.path().join("app"));
    assert!(diags.is_empty(), "{diags:?}");
    // `i`, app, util, tools. One package for one directory.
    assert_eq!(graph.packages.len(), 4);
    let root = &graph.packages[graph.root.unwrap()];
    assert_eq!(root.dependencies[0].1, root.dependencies[1].1);
    let paths: Vec<(&str, ModuleKind)> = root
        .modules
        .iter()
        .map(|m| (m.path.as_str(), m.kind))
        .collect();
    assert_eq!(
        paths,
        [
            ("app/a/b", ModuleKind::Src),
            ("app/main", ModuleKind::Src),
            ("app/t", ModuleKind::Test)
        ]
    );
    // Dependencies: no test modules and no dev-dependencies.
    let util = &graph.packages[root.dependencies[0].1.unwrap()];
    assert_eq!(util.modules.len(), 1);
    assert!(util.dev_dependencies.is_empty());
    assert_eq!(
        graph.sources.get(util.modules[0].source).display,
        "../util/src/lib.i"
    );
    // The library package.
    let lib = &graph.packages[graph.library];
    assert_eq!(lib.name, "i");
    let paths: Vec<&str> = lib.modules.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "i/prelude",
            "i/fail",
            "i/control",
            "i/resource",
            "i/assertion",
            "i/request",
            "i/key",
            "i/core",
            "i/float_text",
            "i/big",
            "i/bits",
            "i/record",
            "i/io",
            "i/task",
            "i/clock",
            "i/random",
            "i/syntax",
            "i/fs",
            "i/process",
            "i/net",
            "i/tls",
            "i/fetch",
            "i/stdio"
        ]
    );
}

#[test]
fn circles_and_missing() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            (
                "a/i.toml",
                "name = \"a\"\n[dependencies]\nb = { path = \"../b\" }\n",
            ),
            (
                "b/i.toml",
                "name = \"b\"\n[dependencies]\nc = { path = \"../c\" }\n",
            ),
            (
                "c/i.toml",
                "name = \"c\"\n[dependencies]\na = { path = \"../a\" }\n",
            ),
        ],
    );
    let (_, diags) = load(&dir.path().join("a"));
    let codes: Vec<_> = diags.iter().map(|d| d.code).collect();
    assert_eq!(codes, [code::PACKAGE_CIRCLE]);

    let (graph, diags) = load(&dir.path().join("nothing"));
    assert!(graph.root.is_none());
    assert_eq!(diags[0].code, code::NO_PACKAGE);
}

#[test]
fn standard_library_modules() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    let (graph, diags) = load(&path);
    assert!(diags.is_empty(), "{diags:?}");
    let root = &graph.packages[graph.root.unwrap()];
    let paths: Vec<&str> = root
        .modules
        .iter()
        .filter(|m| m.kind == ModuleKind::Src)
        .map(|m| m.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "std/bit_array",
            "std/bool",
            "std/calendar",
            "std/cli",
            "std/crypto",
            "std/csv",
            "std/dict",
            "std/float",
            "std/http",
            "std/int",
            "std/json",
            "std/list",
            "std/log",
            "std/option",
            "std/order",
            "std/pair",
            "std/path",
            "std/property",
            "std/regex",
            "std/result",
            "std/schema",
            "std/sequence",
            "std/set",
            "std/stream",
            "std/string",
            "std/test",
            "std/time",
            "std/uri",
        ]
    );
}

#[test]
fn provider_dependency_defaults_keep_authority_under_root_overrides() {
    use ilang_package::providers::{Cell, Target};
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            (
                "app/i.toml",
                "name=\"app\"\n[dependencies]\ndep={path=\"../dep\"}\n[providers.\"dep/main.S\"]\nhost=\"app/main.fake\"\n",
            ),
            (
                "dep/i.toml",
                "name=\"dep\"\n[providers.\"dep/main.S\"]\njs=\"dep/main.js\"\nhost=\"dep/main.host\"\n",
            ),
        ],
    );
    let (graph, diags) = load(&dir.path().join("app"));
    assert!(diags.is_empty(), "{diags:?}");
    let root = graph.root.unwrap();
    let rows = graph.selected_provider_rows();
    let row = rows.iter().find(|r| r.row.service == "dep/main.S").unwrap();
    assert_ne!(row.package, root);
    assert_ne!(row.cells[&Target::Js].package, root);
    assert_eq!(row.cells[&Target::Host].package, root);
    assert!(
        matches!(row.cells[&Target::Host].cell, Cell::Factory { factory, .. } if factory == "app/main.fake")
    );
}
