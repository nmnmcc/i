//! `ilang_check::parse` parses all of the compiled package, but of the
//! library and of the dependencies only the modules that an import reaches
//! (and the prelude, the runtime modules, and the modules named by the
//! manifest). An unreached module is not read: its problems are not
//! reported, while a reached module's problems are. The library is the
//! package `i` (`stdlib/`), a path dependency of `app`.

mod common;

use ilang_package::PackageGraph;
use std::fs;
use std::path::Path;

fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let p = dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
}

/// Writes the package `app` and, beside it, one package per entry of
/// `packages` (name, its dependency names, its files). `app` depends on
/// `app_deps` and on the standard library `i`. Each package names its
/// dependencies by path.
fn load(
    app_deps: &[&str],
    app: &[(&str, &str)],
    packages: &[(&str, &[&str], &[(&str, &str)])],
) -> (tempfile::TempDir, PackageGraph) {
    let dir = tempfile::tempdir().unwrap();
    let manifest = |name: &str, deps: &[&str], library: bool| {
        let mut text = format!("name = \"{name}\"\n");
        if !deps.is_empty() || library {
            text.push_str("[dependencies]\n");
            for d in deps {
                text.push_str(&format!("{d} = {{ path = \"../{d}\" }}\n"));
            }
            if library {
                text.push_str(&common::stdlib_dependency());
            }
        }
        text
    };
    for (name, deps, files) in packages {
        let root = dir.path().join(name);
        write(&root, files);
        fs::write(root.join("i.toml"), manifest(name, deps, false)).unwrap();
    }
    let root = dir.path().join("app");
    write(&root, app);
    fs::write(root.join("i.toml"), manifest("app", app_deps, true)).unwrap();
    let (graph, diags) = ilang_package::load(&root);
    assert!(diags.is_empty(), "{diags:?}");
    (dir, graph)
}

fn parsed_paths(graph: &PackageGraph) -> Vec<String> {
    let (parsed, _) = ilang_check::parse(graph);
    parsed.into_iter().map(|p| p.path).collect()
}

fn rendered(graph: &PackageGraph) -> Vec<String> {
    let checked = ilang_check::check(graph);
    graph.sources.render_all(&checked.diags)
}

const USED: &str = "(\n\tgreet\n)\ngreet = () { 1 }\n";
/// A syntax error.
const BROKEN: &str = "(\n\tbroken\n)\nbroken = ( { \n";
/// A name that is not defined.
const WRONG: &str = "(\n\twrong\n)\nwrong = () { undefined_here }\n";

#[test]
fn unimported_dependency_modules_are_not_parsed_or_reported() {
    let (_dir, graph) = load(
        &["lib"],
        &[(
            "src/main.i",
            "(\n\tlib/used\n)\nmain = () { used.greet() }\n",
        )],
        &[(
            "lib",
            &[],
            &[
                ("src/used.i", USED),
                ("src/broken.i", BROKEN),
                ("src/wrong.i", WRONG),
            ],
        )],
    );
    let paths = parsed_paths(&graph);
    assert!(paths.contains(&"lib/used".to_string()), "{paths:?}");
    assert!(!paths.contains(&"lib/broken".to_string()), "{paths:?}");
    assert!(!paths.contains(&"lib/wrong".to_string()), "{paths:?}");
    let (_, diags) = ilang_check::parse(&graph);
    assert!(diags.is_empty(), "{diags:?}");
    let checked = ilang_check::check(&graph);
    let shown = graph.sources.render_all(&checked.diags);
    assert!(shown.is_empty(), "{shown:#?}");
    assert!(checked.program.is_some());
}

#[test]
fn imported_dependency_modules_report_their_problems() {
    for (import, code) in [("lib/broken", ""), ("lib/wrong", "E0201")] {
        let main = format!("(\n\t{import}\n)\nmain = () {{ Nil }}\n");
        let (_dir, graph) = load(
            &["lib"],
            &[("src/main.i", &main)],
            &[(
                "lib",
                &[],
                &[
                    ("src/used.i", USED),
                    ("src/broken.i", BROKEN),
                    ("src/wrong.i", WRONG),
                ],
            )],
        );
        let paths = parsed_paths(&graph);
        assert!(paths.contains(&import.to_string()), "{paths:?}");
        assert!(!paths.contains(&"lib/used".to_string()), "{paths:?}");
        let checked = ilang_check::check(&graph);
        let file = import.strip_prefix("lib/").unwrap();
        let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
        assert!(!errors.is_empty(), "{import}");
        for d in &errors {
            let source = graph.sources.get(d.source.unwrap());
            assert!(
                source.display.ends_with(&format!("src/{file}.i")),
                "{import}: {}",
                graph.sources.render(d)
            );
            if !code.is_empty() {
                assert_eq!(d.code, code, "{}", graph.sources.render(d));
            }
        }
    }
}

#[test]
fn unimported_root_modules_are_still_parsed_and_reported() {
    let (_dir, graph) = load(
        &[],
        &[
            ("src/main.i", "main = () { Nil }\n"),
            ("src/orphan.i", WRONG),
        ],
        &[],
    );
    assert!(parsed_paths(&graph).contains(&"app/orphan".to_string()));
    let shown = rendered(&graph);
    assert!(
        shown
            .iter()
            .any(|d| d.contains("src/orphan.i") && d.contains("E0201")),
        "{shown:#?}"
    );
    let (_dir, graph) = load(
        &[],
        &[
            ("src/main.i", "main = () { Nil }\n"),
            ("src/orphan.i", BROKEN),
        ],
        &[],
    );
    let (_, diags) = ilang_check::parse(&graph);
    assert!(!diags.is_empty());
    assert!(diags.iter().all(|d| {
        graph
            .sources
            .get(d.source.unwrap())
            .display
            .ends_with("src/orphan.i")
    }));
}

#[test]
fn imports_are_followed_through_dependency_packages() {
    let (_dir, graph) = load(
        &["a"],
        &[("src/main.i", "(\n\ta/x\n)\nmain = () { x.run() }\n")],
        &[
            (
                "a",
                &["b"],
                &[
                    (
                        "src/x.i",
                        "(\n\tb/y\n)\n(\n\trun\n)\nrun = () { y.greet() }\n",
                    ),
                    ("src/unused.i", BROKEN),
                ],
            ),
            (
                "b",
                &[],
                &[
                    (
                        "src/y.i",
                        "(\n\tb/deeper/z\n)\n(\n\tgreet\n)\ngreet = () { z.greet() }\n",
                    ),
                    ("src/deeper/z.i", USED),
                    ("src/other.i", BROKEN),
                ],
            ),
        ],
    );
    let paths = parsed_paths(&graph);
    for reached in ["app/main", "a/x", "b/y", "b/deeper/z"] {
        assert!(paths.contains(&reached.to_string()), "{reached}: {paths:?}");
    }
    for unreached in ["a/unused", "b/other"] {
        assert!(
            !paths.contains(&unreached.to_string()),
            "{unreached}: {paths:?}"
        );
    }
    let shown = rendered(&graph);
    assert!(shown.is_empty(), "{shown:#?}");
}

/// Of the library, a program with no imports has the prelude, the
/// runtime modules, and the modules that code generation calls
/// (`CODEGEN_LIBRARY`), with their imports, in the order of the graph: the
/// built-in prelude, the compiled package, then the dependency `i`, whose
/// modules are in file order.
#[test]
fn library_modules_are_parsed_only_when_reached() {
    let (_dir, graph) = load(&[], &[("src/main.i", "main = () { Nil }\n")], &[]);
    assert_eq!(
        parsed_paths(&graph),
        [
            "$prelude",
            "app/main",
            "i/big",
            "i/bits",
            "i/control",
            "i/core",
            "i/fail",
            "i/float_text",
            "i/key",
            "i/random",
            "i/request",
            "i/resource",
            "i/task",
        ]
    );
    // They are found in the dependency `i`, not in the built-in package.
    let library = graph.standard_library().unwrap();
    assert_ne!(library, graph.library);
    let (parsed, _) = ilang_check::parse(&graph);
    for p in &parsed {
        if p.path.starts_with("i/") {
            assert_eq!(p.package, library, "{}", p.path);
        } else if p.path == "$prelude" {
            assert_eq!(p.package, graph.library);
        }
    }
    // Every other module of `i` is not parsed.
    let paths = parsed_paths(&graph);
    for unreached in ["i/json", "i/list", "i/syntax", "i/test"] {
        assert!(!paths.contains(&unreached.to_string()), "{paths:?}");
    }
    let (_dir, graph) = load(
        &[],
        &[("src/main.i", "(\n\ti/io\n)\nmain = () { Nil }\n")],
        &[],
    );
    let paths = parsed_paths(&graph);
    assert!(paths.contains(&"i/io".to_string()), "{paths:?}");
    assert!(!paths.contains(&"i/net".to_string()), "{paths:?}");
    for unreached in ["i/fs", "i/process", "i/tls", "i/fetch", "i/stdio"] {
        assert!(!paths.contains(&unreached.to_string()), "{paths:?}");
    }
}

/// A `[wasm]` export may name a dependency module that nothing imports.
#[test]
fn manifest_exports_reach_dependency_modules() {
    let dir = tempfile::tempdir().unwrap();
    write(
        &dir.path().join("lib"),
        &[
            ("i.toml", "name = \"lib\"\n"),
            ("src/used.i", USED),
            ("src/broken.i", BROKEN),
        ],
    );
    write(
        &dir.path().join("app"),
        &[
            (
                "i.toml",
                format!(
                    "name = \"app\"\n[dependencies]\nlib = {{ path = \"../lib\" }}\n{}[wasm]\nexports = [\"lib/used.greet\"]\n",
                    common::stdlib_dependency()
                )
                .as_str(),
            ),
            ("src/main.i", "main = () { Nil }\n"),
        ],
    );
    let (graph, diags) = ilang_package::load(&dir.path().join("app"));
    assert!(diags.is_empty(), "{diags:?}");
    let paths = parsed_paths(&graph);
    assert!(paths.contains(&"lib/used".to_string()), "{paths:?}");
    assert!(!paths.contains(&"lib/broken".to_string()), "{paths:?}");
}

/// Repair advice still names modules that were not parsed because nothing
/// imports them, as when every module was parsed.
#[test]
fn advice_names_unparsed_modules() {
    let (_dir, graph) = load(
        &["lib"],
        &[("src/main.i", "main = () { greet() }\n")],
        &[(
            "lib",
            &[],
            &[("src/used.i", USED), ("src/broken.i", BROKEN)],
        )],
    );
    assert!(!parsed_paths(&graph).contains(&"lib/used".to_string()));
    let shown = rendered(&graph);
    assert!(
        shown
            .iter()
            .any(|d| d
                .contains("`greet` is not defined. Import `(greet) = lib/used` to use `greet`.")),
        "{shown:#?}"
    );
    let (_dir, graph) = load(
        &["lib"],
        &[("src/main.i", "main = () { used }\n")],
        &[("lib", &[], &[("src/used.i", USED)])],
    );
    let shown = rendered(&graph);
    assert!(
        shown
            .iter()
            .any(|d| d.contains("Import `lib/used` to use the module `used`.")),
        "{shown:#?}"
    );
}
