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
            ("src/app.i", "run = () { () }\n"),
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
            ("test/worker.i", "run = () { () }\n"),
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
                "name = \"app\"\n[dependencies]\nu = { path = \"../util\" }\nv = { path = \"../util\" }\ni = { path = \"../stdlib\" }\n[dev-dependencies]\nt = { path = \"../tools\" }\n",
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
    // The built-in package, app, util, tools, and the path package named
    // `i`. One package for one directory.
    assert_eq!(graph.packages.len(), 5);
    let root = &graph.packages[graph.root.unwrap()];
    let dependency = |name: &str| {
        root.dependencies
            .iter()
            .find(|(n, _)| n == name)
            .and_then(|(_, id)| *id)
    };
    assert_eq!(dependency("u"), dependency("v"));
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
    let util = &graph.packages[dependency("u").unwrap()];
    assert_eq!(util.modules.len(), 1);
    assert!(util.dev_dependencies.is_empty());
    assert_eq!(
        graph.sources.get(util.modules[0].source).display,
        "../util/src/lib.i"
    );
    // The built-in package has only the prelude; the library modules are in
    // the package named `i`, an ordinary dependency.
    let builtin = &graph.packages[graph.library];
    assert_eq!(builtin.name, ilang_package::library::BUILTIN_NAME);
    assert!(builtin.dir.is_none());
    let paths: Vec<&str> = builtin.modules.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(paths, ["$prelude"]);
    let (name, library) = root.dependencies.iter().find(|(n, _)| n == "i").unwrap();
    assert_eq!(name, "i");
    let library = library.unwrap();
    assert!(graph.is_standard_library(library));
    assert!(graph.is_language(library));
    assert!(graph.is_language(graph.library));
    assert!(!graph.is_language(graph.root.unwrap()));
    assert_eq!(graph.standard_library(), Some(library));
    let paths: Vec<&str> = graph.packages[library]
        .modules
        .iter()
        .map(|m| m.path.as_str())
        .collect();
    assert_eq!(paths, ["i/core"]);
    assert_eq!(graph.standard_library_module("i/core"), Some((library, 0)));
    assert_eq!(graph.standard_library_module("i/syntax"), None);
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

/// The repository's `stdlib/`, the package `i`.
fn stdlib() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../stdlib")
        .canonicalize()
        .unwrap()
}

#[test]
fn standard_library_modules() {
    // The library modules come from the package `i` in `stdlib/`, here a
    // path dependency, and none is built in.
    let dir = tempfile::tempdir().unwrap();
    let manifest = format!(
        "name = \"app\"\n[dependencies]\ni = {{ path = \"{}\" }}\n",
        stdlib().display()
    );
    write(
        dir.path(),
        &[("i.toml", manifest.as_str()), ("src/app.i", "x = 1\n")],
    );
    let (graph, diags) = load(dir.path());
    assert!(diags.is_empty(), "{:?}", graph.sources.render_all(&diags));
    let root = &graph.packages[graph.root.unwrap()];
    let library = root.dependencies[0].1.unwrap();
    assert_eq!(graph.standard_library(), Some(library));
    let package = &graph.packages[library];
    assert_eq!(package.name, "i");
    assert_eq!(package.dir.as_deref(), Some(stdlib().as_path()));
    // A dependency: only its `src/` modules.
    assert!(package.modules.iter().all(|m| m.kind == ModuleKind::Src));
    let paths: Vec<&str> = package.modules.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "i/assertion",
            "i/big",
            "i/bit_array",
            "i/bits",
            "i/bool",
            "i/calendar",
            "i/cli",
            "i/clock",
            "i/control",
            "i/core",
            "i/crypto",
            "i/csv",
            "i/deferred",
            "i/dict",
            "i/fail",
            "i/fetch",
            "i/float",
            "i/float_text",
            "i/fs",
            "i/http",
            "i/int",
            "i/io",
            "i/json",
            "i/key",
            "i/list",
            "i/log",
            "i/net",
            "i/option",
            "i/order",
            "i/pair",
            "i/path",
            "i/process",
            "i/property",
            "i/queue",
            "i/random",
            "i/regex",
            "i/request",
            "i/resource",
            "i/result",
            "i/schema",
            "i/semaphore",
            "i/sequence",
            "i/set",
            "i/stdio",
            "i/stream",
            "i/string",
            "i/syntax",
            "i/task",
            "i/test",
            "i/time",
            "i/tls",
            "i/uri",
        ]
    );
    assert_eq!(paths.len(), 52);
    // The runtime modules are among them.
    for path in ilang_package::library::RUNTIME {
        assert!(graph.standard_library_module(path).is_some(), "{path}");
    }
    // The built-in package has only the prelude.
    let builtin = &graph.packages[graph.library];
    assert_eq!(builtin.modules.len(), 1);
    assert_eq!(builtin.modules[0].kind, ModuleKind::Prelude);
}

#[test]
fn the_standard_library_compiled_as_the_root() {
    // `stdlib/` itself, compiled: it needs no dependency `i`, and its own
    // modules are the library.
    let (graph, diags) = load(&stdlib());
    assert!(diags.is_empty(), "{:?}", graph.sources.render_all(&diags));
    let root = graph.root.unwrap();
    assert_eq!(graph.standard_library(), Some(root));
    assert_eq!(
        graph
            .standard_library_module("i/core")
            .map(|(package, _)| package),
        Some(root)
    );
    let src = graph.packages[root]
        .modules
        .iter()
        .filter(|m| m.kind == ModuleKind::Src)
        .count();
    assert_eq!(src, 52);
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
            ("dep/src/main.i", "(value)\nvalue = 42\nother = () { () }\n"),
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
        if table == "dev-dependencies" {
            // The runtime modules are part of every compiled program, so a
            // dev-dependency alone does not resolve the library for `src/`.
            assert!(
                diags
                    .iter()
                    .any(|d| d.code == ilang_package::code::NO_LIBRARY),
                "{:?}",
                graph.sources.render_all(&diags)
            );
            continue;
        }
        let rendered = graph.sources.render_all(&diags);
        assert!(rendered.is_empty(), "{table}: {rendered:#?}");
        let root = &graph.packages[graph.root.unwrap()];
        let deps = &root.dependencies;
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

/// The `E0640` and `E0641` diagnostics of loading the package in `dir`:
/// code, message, and the messages of their notes.
fn library_problems(dir: &Path) -> Vec<(&'static str, String, Vec<String>)> {
    let (_, diags) = load(dir);
    diags
        .iter()
        .filter(|d| d.code == code::LEGACY_LIBRARY || d.code == code::NO_LIBRARY)
        .map(|d| {
            (
                d.code,
                d.message.clone(),
                d.related.iter().map(|n| n.message.clone()).collect(),
            )
        })
        .collect()
}

const LEGACY_MESSAGE: &str = "the standard library is delivered by the package manager, not as a copy in the project\n  help: delete `std/` and remove the `std = { path = \"std\" }` dependency, then add:\n        i = \"github.com/nmnmcc/i//stdlib@<latest>\"";

/// A project made before the library was a package has `std = { path =
/// "std" }`, or the copied `std/` directory: either is `E0640` (instead of
/// `E0641`), on the package name.
#[test]
fn the_old_copied_std_is_an_error() {
    // The old dependency line, without the directory.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            (
                "i.toml",
                "name = \"app\"\n[dependencies]\nstd = { path = \"std\" }\n",
            ),
            ("src/app.i", "x = 1\n"),
        ],
    );
    assert_eq!(
        library_problems(dir.path()),
        [(code::LEGACY_LIBRARY, LEGACY_MESSAGE.to_string(), vec![])]
    );
    let (graph, diags) = load(dir.path());
    let d = diags
        .iter()
        .find(|d| d.code == code::LEGACY_LIBRARY)
        .unwrap();
    assert!(d.is_error());
    assert_eq!(
        graph.sources.render(d).lines().next().unwrap(),
        "i.toml:1:8: error[E0640]: the standard library is delivered by the package manager, not as a copy in the project"
    );

    // The copied directory, without the dependency line. It is not the old
    // copy unchanged, so a note says to back it up first.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            ("i.toml", "name = \"app\"\n"),
            ("src/app.i", "x = 1\n"),
            ("std/i.toml", "name = \"std\"\n"),
            ("std/src/list.i", "mine = 1\n"),
        ],
    );
    assert_eq!(
        library_problems(dir.path()),
        [(
            code::LEGACY_LIBRARY,
            LEGACY_MESSAGE.to_string(),
            vec!["this copy has changes; back it up before deleting it".to_string()]
        )]
    );

    // Both, with the modified copy: one error, with the note.
    write(
        dir.path(),
        &[(
            "i.toml",
            "name = \"app\"\n[dependencies]\nstd = { path = \"std\" }\n",
        )],
    );
    let problems = library_problems(dir.path());
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].0, code::LEGACY_LIBRARY);
    assert_eq!(
        problems[0].2,
        ["this copy has changes; back it up before deleting it"]
    );
}

/// A dependency named `std` that is not the old copy (a location, or a path
/// outside the project) is an ordinary dependency, not `E0640`; the package
/// still needs `i`.
#[test]
fn a_std_dependency_elsewhere_is_not_the_old_copy() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            (
                "app/i.toml",
                "name = \"app\"\n[dependencies]\nstd = { path = \"../other\" }\n",
            ),
            ("app/src/app.i", "x = 1\n"),
            ("other/i.toml", "name = \"other\"\n"),
        ],
    );
    let problems = library_problems(&dir.path().join("app"));
    let codes: Vec<_> = problems.iter().map(|p| p.0).collect();
    assert_eq!(codes, [code::NO_LIBRARY], "{problems:?}");
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
    // The built-in package has nothing else, and no dependency can name it.
    assert_eq!(lib.modules.len(), 1);
    assert_eq!(lib.name, "$builtin");
    assert!(!ilang_package::is_name(&lib.name));
    assert_eq!(
        graph.sources.get(prelude[0].source).display,
        "<i>/prelude.i"
    );
}

/// The language hash covers the prelude and the modules of the package `i`,
/// and nothing of the other packages; it is the hash that a host grants to
/// the bindings of both.
#[test]
fn the_language_hash_covers_the_prelude_and_the_library() {
    use ilang_package::lock::{FileKind, PackageFile, hash};
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        &[
            (
                "app/i.toml",
                "name = \"app\"\n[dependencies]\ni = { path = \"../stdlib\" }\n",
            ),
            ("app/src/main.i", "x = 1\n"),
        ],
    );
    library(&dir.path().join("stdlib"));
    let (graph, diags) = load(&dir.path().join("app"));
    assert!(diags.is_empty(), "{diags:?}");
    let file = |path: &str, text: &str| PackageFile {
        path: path.into(),
        kind: FileKind::Regular,
        bytes: text.as_bytes().to_vec(),
    };
    let (prelude_path, prelude_text) = ilang_package::library::PRELUDE;
    let want = hash(&[file(prelude_path, prelude_text), file("i/core", "x = 1\n")]);
    assert_eq!(graph.language_hash(), want);

    // The compiled package does not take part.
    write(&dir.path().join("app"), &[("src/main.i", "x = 2\n")]);
    let (again, _) = load(&dir.path().join("app"));
    assert_eq!(again.language_hash(), want);

    // A change to `i` changes it.
    write(&dir.path().join("stdlib"), &[("src/core.i", "x = 2\n")]);
    let (changed, _) = load(&dir.path().join("app"));
    assert_ne!(changed.language_hash(), want);
}
