//! The seventh SYN2 decision: package-scoped conventional and named entries.
use ilang_cli::{BuildOptions, Level, Target, build, build_entry};
use std::path::Path;

fn project(manifest: &str, files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), manifest).unwrap();
    for (path, source) in files {
        let file = dir.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, source).unwrap();
    }
    dir
}
fn options() -> BuildOptions {
    BuildOptions {
        tests: false,
        opt: Level::O0,
        offline: true,
        target: Target::Ilang,
    }
}
fn entry(path: &Path, name: Option<&str>) -> Option<String> {
    let built = build_entry(path, options(), name).unwrap_or_else(|e| panic!("{e}"));
    built
        .host_program()
        .unwrap()
        .abi()
        .entry
        .as_ref()
        .map(|e| e.name.clone())
}
#[test]
fn default_and_private_named_function_keep_the_existing_metadata_shape() {
    // Rule 2 and ruling [entries]: only src/main.main is the default;
    // named entries may point at an unexported function in another module.
    let dir = project(
        "name = \"app\"\n[entries]\nworker = \"app/tools.work\"\n",
        &[
            ("src/main.i", "main = () { Nil }"),
            ("src/tools.i", "work = () { Nil }\nmain = (x) { x }"),
        ],
    );
    assert_eq!(entry(dir.path(), None).as_deref(), Some("app/main.main"));
    assert_eq!(
        entry(dir.path(), Some("worker")).as_deref(),
        Some("app/tools.work")
    );
}
#[test]
fn missing_default_and_other_module_main_have_no_entry() {
    // Rules 4/5: main has no special meaning in other modules.
    let dir = project("name = \"app\"\n", &[("src/tools.i", "main = (x) { x }")]);
    assert_eq!(entry(dir.path(), None), None);
}
#[test]
fn every_declared_target_is_checked_and_selection_must_exist() {
    // Rule 5: value parameters are an error, even for an unselected target.
    let dir = project(
        "name = \"app\"\n[entries]\nbad = \"app/tools.work\"\n",
        &[
            ("src/main.i", "main = () { Nil }"),
            ("src/tools.i", "work = (x) { x }"),
        ],
    );
    assert!(
        build(dir.path(), options())
            .err()
            .unwrap()
            .to_string()
            .contains("no parameters")
    );
    let dir = project("name = \"app\"\n", &[("src/main.i", "main = () { Nil }")]);
    assert!(
        build_entry(dir.path(), options(), Some("missing"))
            .err()
            .unwrap()
            .to_string()
            .contains("unknown entry")
    );
}
#[test]
fn dependency_conventions_and_entry_tables_have_no_effect() {
    // Rule 5: dependency package entries and main functions are ignored.
    let root = tempfile::tempdir().unwrap();
    let dep = root.path().join("dep");
    std::fs::create_dir_all(dep.join("src")).unwrap();
    std::fs::write(dep.join("i.toml"), "name = \"dep\"\nentries = 42\n").unwrap();
    std::fs::write(dep.join("src/main.i"), "main = (x) { x }\n").unwrap();
    let app = root.path().join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::write(
        app.join("i.toml"),
        "name = \"app\"\n[dependencies]\ndep = { path = \"../dep\" }\n",
    )
    .unwrap();
    std::fs::write(app.join("src/main.i"), "main = () { Nil }\n").unwrap();
    assert_eq!(entry(&app, None).as_deref(), Some("app/main.main"));
}

#[test]
fn a_bare_bar_is_not_a_pipe() {
    // A bare `|` is not a pipe; the pipe remains `|>`.
    let source = "main = () { False | True }";
    let dir = project("name = \"app\"\n", &[("src/main.i", source)]);
    let (graph, diagnostics) = ilang_cli::load(dir.path(), false, true);
    assert!(diagnostics.is_empty());
    let checked = ilang_check::check(&graph);
    let diagnostic = checked
        .diags
        .iter()
        .find(|d| d.is_error())
        .expect("bare pipe error");
    assert_eq!(diagnostic.span.start as usize, source.find('|').unwrap());
    // A function that returns Bool is an ordinary valid pipe target.
    let dir = project(
        "name = \"app\"\n",
        &[("src/main.i", "main = () { True |> (value) { value } }")],
    );
    assert!(build(dir.path(), options()).is_ok());
}
