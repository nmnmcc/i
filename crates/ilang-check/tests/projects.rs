//! Golden project tests: each directory in `tests/projects/` is a package.
//! It is loaded and checked, and the diagnostics are compared with its
//! `expected.out`. `UPDATE=1` writes the files.

use std::fs;
use std::path::Path;

fn run(dir: &Path) -> String {
    let (graph, mut diags) = ilang_package::load(dir);
    if graph.root.is_some() {
        let (_, more) = ilang_check::check(&graph);
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
