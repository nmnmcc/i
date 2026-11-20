//! A project parses its own modules and the import closure, not every module
//! of every dependency. The library is the package `i` (`stdlib/`), an
//! ordinary path dependency of `app`.

mod common;

use std::fs;

/// The manifest of `app`: the standard library `i` and the given further
/// `[dependencies]` lines.
fn app_manifest(dependencies: &str) -> String {
    format!(
        "name = \"app\"\n[dependencies]\n{}{dependencies}",
        common::stdlib_dependency()
    )
}

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let p = dir.path().join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    dir
}

#[test]
fn unimported_dependency_modules_are_not_parsed_or_reported() {
    let dir = project(&[
        ("app/i.toml", &app_manifest("dep = { path = \"../dep\" }\n")),
        ("app/src/main.i", "(\n\tdep/used\n)\nmain = () { used.x }\n"),
        ("dep/i.toml", "name = \"dep\"\n"),
        ("dep/src/used.i", "(x)\nx = 1\n"),
        ("dep/src/broken.i", "this is not i syntax\n"),
    ]);
    let (graph, diags) = ilang_package::load(&dir.path().join("app"));
    let rendered = graph.sources.render_all(&diags);
    assert!(rendered.is_empty(), "{rendered:?}");
    let checked = ilang_check::check(&graph);
    let rendered = graph.sources.render_all(&checked.diags);
    assert!(rendered.is_empty(), "{rendered:?}");
    let program = checked.program.unwrap();
    assert!(program.modules.iter().any(|m| m.path == "dep/used"));
    assert!(!program.modules.iter().any(|m| m.path == "dep/broken"));
}

#[test]
fn unimported_root_modules_are_still_checked() {
    let dir = project(&[
        ("app/i.toml", &app_manifest("")),
        ("app/src/main.i", "main = () { () }\n"),
        ("app/src/other.i", "broken = missing_name\n"),
    ]);
    let (graph, diags) = ilang_package::load(&dir.path().join("app"));
    assert!(diags.is_empty(), "{diags:?}");
    let checked = ilang_check::check(&graph);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.message.contains("missing_name")),
        "{:?}",
        checked.diags
    );
}

#[test]
fn an_imported_dependency_module_is_parsed_and_its_error_reported() {
    let dir = project(&[
        ("app/i.toml", &app_manifest("dep = { path = \"../dep\" }\n")),
        (
            "app/src/main.i",
            "(\n\tdep/broken\n)\nmain = () { broken.x }\n",
        ),
        ("dep/i.toml", "name = \"dep\"\n"),
        ("dep/src/broken.i", "this is not i syntax\n"),
    ]);
    let (graph, diags) = ilang_package::load(&dir.path().join("app"));
    assert!(diags.is_empty(), "{diags:?}");
    let checked = ilang_check::check(&graph);
    assert!(!checked.diags.is_empty());
}

/// The library `i` is a dependency like any other: of it only the modules
/// that an import, the prelude, or code generation (`CODEGEN_LIBRARY`)
/// reaches are parsed. The prelude is the built-in package; every `i/…`
/// module comes from the dependency `i`.
#[test]
fn library_modules_are_parsed_only_when_reached() {
    let dir = project(&[
        ("app/i.toml", &app_manifest("")),
        ("app/src/main.i", "main = () { () }\n"),
    ]);
    let (graph, diags) = ilang_package::load(&dir.path().join("app"));
    assert!(diags.is_empty(), "{diags:?}");
    let library = graph.standard_library().expect("the dependency `i`");
    assert_ne!(library, graph.library);
    assert_eq!(graph.packages[library].name, "i");
    let (parsed, diags) = ilang_check::parse(&graph);
    assert!(diags.is_empty(), "{diags:?}");
    let paths: Vec<&str> = parsed.iter().map(|p| p.path.as_str()).collect();
    assert!(paths.contains(&"$prelude"), "{paths:?}");
    assert!(paths.contains(&"app/main"), "{paths:?}");
    for reached in ilang_check::CODEGEN_LIBRARY {
        assert!(paths.contains(reached), "{reached}: {paths:?}");
    }
    for p in &parsed {
        if p.path.starts_with("i/") {
            assert_eq!(p.package, library, "{}", p.path);
        } else if p.path == "$prelude" {
            assert_eq!(p.package, graph.library);
        }
    }
    for unreached in ["i/json", "i/list", "i/net", "i/fetch"] {
        assert!(!paths.contains(&unreached), "{unreached}: {paths:?}");
    }
    // An import reaches its module, and not its siblings.
    let dir = project(&[
        ("app/i.toml", &app_manifest("")),
        ("app/src/main.i", "(\n\ti/io\n)\nmain = () { () }\n"),
    ]);
    let (graph, _) = ilang_package::load(&dir.path().join("app"));
    let (parsed, _) = ilang_check::parse(&graph);
    let paths: Vec<&str> = parsed.iter().map(|p| p.path.as_str()).collect();
    assert!(paths.contains(&"i/io"), "{paths:?}");
    for unreached in ["i/fs", "i/process", "i/tls", "i/fetch", "i/stdio"] {
        assert!(!paths.contains(&unreached), "{unreached}: {paths:?}");
    }
}
