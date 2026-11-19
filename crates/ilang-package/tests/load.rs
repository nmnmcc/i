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

/// A small package named `i`, at `dir`, for the dependency `i` that every
/// compiled package has.
const LIBRARY: [(&str, &str); 2] = [("i.toml", "name = \"i\"\n"), ("src/core.i", "x = 1\n")];

fn library(dir: &Path) {
    write(dir, &LIBRARY);
}

#[test]
fn source_entries_do_not_load_test_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            (
                "i.toml",
                "name = \"app\"\n[entries]\nrun = \"app.run\"\n[dependencies]\ni = { path = \"stdlib\" }\n[dev-dependencies]\nmissing = { path = \"missing\" }\n",
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
    library(&dir.path().join("stdlib"));
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
                "name = \"app\"\n[entries]\nrun = \"worker.run\"\n[dependencies]\ni = { path = \"stdlib\" }\n[dev-dependencies]\ndep = { path = \"dep\" }\n",
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
    library(&dir.path().join("stdlib"));
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
                "name = \"app\"\n[dependencies]\nu = { path = \"../util\" }\nv = { path = \"../util\" }\n[dev-dependencies]\nt = { path = \"../tools\" }\ni = { path = \"../../stdlib\" }\n",
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
    library(&dir.path().join("stdlib"));
    let (graph, diags) = load(&dir.path().join("app"));
    assert!(diags.is_empty(), "{diags:?}");
    // `i`, app, util, tools, and the path package named `i`. One package for
    // one directory.
    assert_eq!(graph.packages.len(), 5);
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
            "$prelude",
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
                "name = \"a\"\n[dependencies]\nb = { path = \"../b\" }\ni = { path = \"../stdlib\" }\n",
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
    library(&dir.path().join("stdlib"));
    let (_, diags) = load(&dir.path().join("a"));
    let codes: Vec<_> = diags.iter().map(|d| d.code).collect();
    assert_eq!(codes, [code::PACKAGE_CIRCLE]);

    let (graph, diags) = load(&dir.path().join("nothing"));
    assert!(graph.root.is_none());
    assert_eq!(diags[0].code, code::NO_PACKAGE);
}

#[test]
fn standard_library_modules() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../stdlib");
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
            "i/bit_array",
            "i/bool",
            "i/calendar",
            "i/cli",
            "i/crypto",
            "i/csv",
            "i/dict",
            "i/float",
            "i/http",
            "i/int",
            "i/json",
            "i/list",
            "i/log",
            "i/option",
            "i/order",
            "i/pair",
            "i/path",
            "i/property",
            "i/regex",
            "i/result",
            "i/schema",
            "i/sequence",
            "i/set",
            "i/stream",
            "i/string",
            "i/test",
            "i/time",
            "i/uri",
        ]
    );
}

// E5 removes dependency defaults: only the root's explicit entry is selected.
#[test]
fn dependency_defaults_cannot_replace_the_roots_explicit_entry() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            (
                "app/i.toml",
                "name=\"app\"\n[dependencies]\ndep={path=\"../dep\"}\ni={path=\"../stdlib\"}\n[entries]\nnative=\"main.main\"\n[providers.\"dep/main.S\"]\nhost=\"app/main.fake\"\n",
            ),
            (
                "app/src/main.i",
                "(dep = dep/main)\nmain = () { dep.value }\n",
            ),
            (
                "dep/i.toml",
                "name=\"dep\"\n[entries]\nnative=\"main.other\"\n[providers.\"dep/main.S\"]\njs=\"dep/main.js\"\nhost=\"dep/main.host\"\n",
            ),
            (
                "dep/src/main.i",
                "(value)\nvalue = 42\nother = () { Nil }\n",
            ),
        ],
    );
    library(&dir.path().join("stdlib"));
    let (graph, diagnostics) = load(&dir.path().join("app"));
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let root = &graph.packages[graph.root.unwrap()];
    assert_eq!(root.entries.len(), 1);
    assert_eq!(root.entries[0].0, "native");
    assert_eq!(root.entries[0].1.function, "main");
    let dep = &graph.packages[root.dependencies[0].1.unwrap()];
    assert_eq!(dep.name, "dep");
    assert!(
        dep.entries.is_empty(),
        "a dependency cannot select an entry"
    );
}

/// The `E0641` diagnostics of loading the package in `dir`, rendered.
fn no_library(dir: &Path) -> Vec<String> {
    let (graph, diags) = load(dir);
    let found: Vec<_> = diags
        .iter()
        .filter(|d| d.code == code::NO_LIBRARY)
        .cloned()
        .collect();
    graph.sources.render_all(&found)
}

#[test]
fn a_package_without_the_standard_library_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[("i.toml", "name = \"app\"\n"), ("src/app.i", "x = 1\n")],
    );
    assert_eq!(
        no_library(dir.path()),
        [
            "i.toml:1:8: error[E0641]: the standard library is not resolved for this package; \
          add `i = \"github.com/nmnmcc/i//stdlib@X.Y.Z\"` to [dependencies]"
        ]
    );
    let (_, diags) = load(dir.path());
    let d = diags.iter().find(|d| d.code == code::NO_LIBRARY).unwrap();
    assert!(d.is_error());
    assert_eq!(d.message, ilang_package::NO_LIBRARY_MESSAGE);
}

#[test]
fn the_standard_library_as_a_path_dependency() {
    for table in ["dependencies", "dev-dependencies"] {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            &[
                (
                    "app/i.toml",
                    &format!("name = \"app\"\n[{table}]\ni = {{ path = \"../stdlib\" }}\n"),
                ),
                ("app/src/app.i", "x = 1\n"),
                ("stdlib/i.toml", "name = \"i\"\n"),
                ("stdlib/src/core.i", "x = 1\n"),
            ],
        );
        let app = dir.path().join("app");
        let (graph, diags) = load(&app);
        let rendered = graph.sources.render_all(&diags);
        assert!(rendered.is_empty(), "{table}: {rendered:#?}");
        let root = &graph.packages[graph.root.unwrap()];
        let deps = if table == "dependencies" {
            &root.dependencies
        } else {
            &root.dev_dependencies
        };
        let (name, id) = &deps[0];
        assert_eq!(name, "i");
        let library = &graph.packages[id.unwrap()];
        assert_eq!(library.name, "i");
        assert_eq!(library.modules[0].path, "i/core");
    }
}

#[test]
fn a_package_named_i_needs_no_standard_library() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[("i.toml", "name = \"i\"\n"), ("src/core.i", "x = 1\n")],
    );
    let (graph, diags) = load(dir.path());
    let rendered = graph.sources.render_all(&diags);
    assert!(rendered.is_empty(), "{rendered:#?}");
}

#[test]
fn the_prelude_has_a_path_that_no_import_names() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), &[("i.toml", "name = \"i\"\n")]);
    let (graph, _) = load(dir.path());
    let lib = &graph.packages[graph.library];
    let prelude: Vec<_> = lib
        .modules
        .iter()
        .filter(|m| m.kind == ModuleKind::Prelude)
        .collect();
    assert_eq!(prelude.len(), 1);
    assert_eq!(prelude[0].path, ilang_package::library::PRELUDE_PATH);
    assert_eq!(prelude[0].path, "$prelude");
    assert!(lib.modules.iter().all(|m| m.path != "i/prelude"));
    assert_eq!(
        graph.sources.get(prelude[0].source).display,
        "<i>/prelude.i"
    );
}
