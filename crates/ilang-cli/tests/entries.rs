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
        target: Target::Native,
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
        "name = \"app\"\n[entries]\nworker = \"tools.work\"\n",
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
fn dotted_entry_path_is_relative_to_the_package() {
    let dir = project(
        "name = \"app\"\n[entries]\nworker = \"tools.jobs.work\"\n",
        &[("src/tools/jobs.i", "work = () { Nil }")],
    );
    assert_eq!(
        entry(dir.path(), Some("worker")).as_deref(),
        Some("app/tools/jobs.work")
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
        "name = \"app\"\n[entries]\nbad = \"tools.work\"\n",
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
    let source = /* old-syntax-check: E0101 */ "main = () { False | True }";
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

#[test]
fn cli_selects_entries_and_passes_arguments() {
    let dir = project(
        "name = \"app\"\n[entries]\ntool = \"tools.work\"\nother = \"tools.plain\"\n",
        &[
            (
                "src/main.i",
                "(\n\t(Io) = i/io\n)\nmain = () <- (io : Io) { io.print(\"default ok\") }\n",
            ),
            (
                "src/tools.i",
                "(\n\t(Io) = i/io\n\ti/process\n)\nwork = () <- (io : Io) {\n\tassert(process.args() == [\"alpha\", \"--beta\"])\n\tio.print(\"tool ok\")\n}\nplain = () <- (io : Io) { io.print(\"plain ok\") }\n",
            ),
        ],
    );
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_i"))
            .current_dir(dir.path())
            .args(args)
            .output()
            .unwrap()
    };
    let default = run(&["run", "--opt", "0"]);
    assert!(
        default.status.success(),
        "{}",
        String::from_utf8_lossy(&default.stderr)
    );
    assert_eq!(default.stdout, b"default ok\n");
    assert!(
        default.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&default.stderr)
    );
    let plain = run(&["run", "--opt", "0", "other"]);
    assert!(
        plain.status.success(),
        "{}",
        String::from_utf8_lossy(&plain.stderr)
    );
    assert_eq!(plain.stdout, b"plain ok\n");
    assert!(
        plain.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&plain.stderr)
    );
    let named = run(&["run", "--opt", "0", "tool", "--", "alpha", "--beta"]);
    assert!(
        named.status.success(),
        "{}",
        String::from_utf8_lossy(&named.stderr)
    );
    assert_eq!(named.stdout, b"tool ok\n");
    for name in ["missing", "tools.i", "src/tools.i"] {
        let unknown = run(&["run", name]);
        let error = String::from_utf8_lossy(&unknown.stderr);
        assert!(!unknown.status.success());
        assert!(
            error.contains("unknown entry") && error.contains("tool") && error.contains("other"),
            "{error}"
        );
    }
    let old_option = run(&["run", "--entry", "tool"]);
    assert!(!old_option.status.success());
    assert!(String::from_utf8_lossy(&old_option.stderr).contains("unexpected argument '--entry'"));
    let build = run(&["build", "--opt", "0", "tool"]);
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    // A native executable named after the entry.
    assert!(dir.path().join("build/tool").exists());
}
