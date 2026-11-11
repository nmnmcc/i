//! Loading the package graph (§2.1, §2.6).

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
