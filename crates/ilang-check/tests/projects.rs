//! Golden project tests: each directory in `tests/projects/` is a package.
//! It is loaded and resolved, and the diagnostics are compared with its
//! `expected.out`. `UPDATE=1` writes the files. These test names and
//! resolution; types are tested in `tests/types.rs`.

use std::fs;
use std::path::Path;

fn run(dir: &Path) -> String {
    let (graph, mut diags) = ilang_package::load(dir);
    if graph.root.is_some() {
        let (_, more) = ilang_check::resolve_program(&graph);
        diags.extend(more);
    }
    let mut out = String::new();
    for line in graph.sources.render_all(&diags) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

#[test]
fn projects() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/projects");
    let update = std::env::var_os("UPDATE").is_some();
    let mut dirs: Vec<_> = fs::read_dir(&base)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("i.toml").exists())
        .collect();
    dirs.sort();
    assert!(!dirs.is_empty());
    let mut failed = Vec::new();
    for dir in &dirs {
        let actual = run(dir);
        let path = dir.join("expected.out");
        if update {
            fs::write(&path, &actual).unwrap();
            continue;
        }
        let expected = fs::read_to_string(&path).unwrap_or_default();
        if expected != actual {
            eprintln!("=== {} ===\n{actual}", dir.display());
            failed.push(dir.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert!(
        failed.is_empty(),
        "projects differ: {failed:?} (run with UPDATE=1 to bless)"
    );
}

/// The `ok_*` projects resolve with no diagnostics at all.
#[test]
fn ok_projects_are_clean() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/projects");
    for e in fs::read_dir(&base).unwrap() {
        let dir = e.unwrap().path();
        if dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("ok_")
        {
            assert_eq!(run(&dir), "", "{}", dir.display());
        }
    }
}

#[test]
fn an_external_fail_need_retains_the_host_error_shape() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    let source = "@external(\"app\", \"value\")\nvalue : () <- (Fail(String)) -> Int\n";
    fs::write(dir.path().join("src/main.i"), source).unwrap();
    let (graph, load_errors) = ilang_package::load(dir.path());
    assert!(load_errors.is_empty(), "{load_errors:?}");
    let checked = ilang_check::check(&graph);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let (_, external) = program
        .external_fns
        .iter()
        .find(|(id, _)| {
            program.def(**id).name == "value" && program.module(id.module).path == "app/main"
        })
        .unwrap();
    assert_eq!(external.errors, [ilang_check::types::HostType::String]);
    for (need, code) in [
        ("Assert", "E0319"),
        ("Fail(String), Assert", "E0319"),
        ("Fail(List(Int))", "E0318"),
        ("Fail(\"literal\")", "E0318"),
    ] {
        fs::write(
            dir.path().join("src/main.i"),
            source.replace("Fail(String)", need),
        )
        .unwrap();
        let (graph, _) = ilang_package::load(dir.path());
        let checked = ilang_check::check(&graph);
        assert!(
            checked.diags.iter().any(|d| d.code == code),
            "{need}: {:?}",
            graph.sources.render_all(&checked.diags)
        );
    }
}
