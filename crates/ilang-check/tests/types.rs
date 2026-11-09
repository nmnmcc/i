//! Golden type tests. Each `tests/types/<case>.i` is the module `app/main`
//! of a package; each `tests/types/<case>/` with an `i.toml` is a package.
//! The shown type of every top-level name (§4.7), then the problems, are
//! compared with `<case>.out` or `<case>/expected.out`. `library.out` has
//! the types of the library modules. `UPDATE=1` writes the files.

use ilang_package::{Diagnostic, PackageGraph};
use ilang_syntax::span::LineIndex;
use std::fs;
use std::path::Path;

fn output(graph: &PackageGraph, short: bool, library: bool) -> String {
    let checked = ilang_check::check(graph);
    let mut out = String::new();
    if let (Some(program), Some(types)) = (&checked.program, &checked.types) {
        for (i, m) in program.modules.iter().enumerate() {
            let wanted = if library {
                m.package == graph.library
            } else {
                Some(m.package) == graph.root
            };
            if !wanted {
                continue;
            }
            for line in types.lines(program, ilang_check::hir::ModuleId(i as u32)) {
                let line = if short {
                    line.strip_prefix("app/main.").unwrap_or(&line).to_string()
                } else {
                    line
                };
                out.push_str(&line);
                out.push('\n');
            }
        }
    }
    for d in &checked.diags {
        out.push_str(&render(graph, d, short));
        out.push('\n');
    }
    out
}

fn render(graph: &PackageGraph, d: &Diagnostic, short: bool) -> String {
    if !short {
        return graph.sources.render(d);
    }
    let pos = d.source.map_or(String::new(), |s| {
        let (line, col) = LineIndex::new(&graph.sources.get(s).text).line_col(d.span.start);
        format!("{line}:{col}: ")
    });
    format!(
        "{pos}{}[{}]: {}",
        if d.is_error() { "error" } else { "warning" },
        d.code,
        d.message
    )
}

fn compare(path: &Path, actual: &str, failed: &mut Vec<String>) {
    if std::env::var_os("UPDATE").is_some() {
        fs::write(path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(path).unwrap_or_default();
    if expected != actual {
        eprintln!("=== {} ===\n{actual}", path.display());
        failed.push(path.display().to_string());
    }
}

#[test]
fn types() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/types");
    let mut entries: Vec<_> = fs::read_dir(&base)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    let mut failed = Vec::new();
    let mut count = 0;
    for path in &entries {
        if path.extension().is_some_and(|e| e == "i") {
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
            fs::create_dir_all(dir.path().join("src")).unwrap();
            fs::copy(path, dir.path().join("src/main.i")).unwrap();
            let (graph, diags) = ilang_package::load(dir.path());
            assert!(diags.is_empty());
            let actual = output(&graph, true, false);
            compare(&path.with_extension("out"), &actual, &mut failed);
            count += 1;
        } else if path.join("i.toml").exists() {
            let (graph, diags) = ilang_package::load(path);
            assert!(diags.is_empty());
            let actual = output(&graph, false, false);
            compare(&path.join("expected.out"), &actual, &mut failed);
            count += 1;
        }
    }
    assert!(count > 0);
    assert!(
        failed.is_empty(),
        "types differ: {failed:#?} (run with UPDATE=1 to bless)"
    );
}

/// The library modules type check with no problems, and their types are
/// those of the spec tables (§9, §12, §13).
#[test]
fn library() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    let (graph, _) = ilang_package::load(dir.path());
    let actual = output(&graph, false, true);
    let mut failed = Vec::new();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/types/library.out");
    compare(&path, &actual, &mut failed);
    assert!(
        failed.is_empty(),
        "library types differ (run with UPDATE=1 to bless)"
    );
}
