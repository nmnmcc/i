//! A project parses its own modules and the import closure, not every module
//! of every dependency.

use std::fs;

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
        (
            "app/i.toml",
            "name = \"app\"\n[dependencies]\ndep = { path = \"../dep\" }\n",
        ),
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
        ("app/i.toml", "name = \"app\"\n"),
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
        (
            "app/i.toml",
            "name = \"app\"\n[dependencies]\ndep = { path = \"../dep\" }\n",
        ),
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
