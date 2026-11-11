//! Golden tests: each `tests/golden/*.i` is parsed, and the canonical print,
//! the explicit print when it differs, and the errors with `@line:col` are
//! compared with the `.out` file next to it. `UPDATE=1` writes the files.

mod common;

use ilang_syntax::print::{print_module, print_module_explicit};
use ilang_syntax::{parse_module, span::LineIndex};
use std::fs;
use std::path::Path;

fn render(src: &str) -> String {
    let parse = parse_module;
    let (module, diags) = parse(src);
    let mut out = print_module(&module);
    let explicit = print_module_explicit(&module);
    let differ: Vec<&str> = explicit
        .split("\n\n")
        .zip(out.split("\n\n"))
        .filter(|(e, o)| e != o)
        .map(|(e, _)| e.trim_end())
        .collect();
    if !differ.is_empty() {
        out.push_str("--- explicit ---\n");
        out.push_str(&differ.join("\n"));
        out.push('\n');
    }
    if diags.is_empty() {
        assert_eq!(ilang_syntax::print::unprintable(&module), None);
        // The migration marker keeps old empty tuples parseable until the
        // fixtures are rewritten. It does not belong to the printed AST.
        let printed = print_module(&module);
        let marked = if src.lines().any(|line| line.trim() == "// syntax: syn1") {
            format!("// syntax: syn1\n{printed}")
        } else {
            printed
        };
        let (again, diags2) = parse(&marked);
        assert!(
            diags2.is_empty(),
            "the print does not parse: {diags2:?}\n{out}"
        );
        assert_eq!(
            common::shape(&module),
            common::shape(&again),
            "print -> parse changed the tree"
        );
    } else {
        out.push_str("--- errors ---\n");
        let lines = LineIndex::new(src);
        for d in diags {
            let (line, col) = lines.line_col(d.span.start);
            out.push_str(&format!(
                "{} @{line}:{col} {}\n",
                d.code.as_str(),
                d.message
            ));
        }
    }
    out
}

#[test]
fn golden() {
    check_goldens();
}

fn check_goldens() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let update = std::env::var_os("UPDATE").is_some();
    let retired = dir.with_file_name("golden_old");
    if update && retired.exists() {
        for entry in fs::read_dir(&retired).unwrap() {
            let path = entry.unwrap().path();
            assert_eq!(path.extension().and_then(|e| e.to_str()), Some("out"));
            fs::remove_file(path).unwrap();
        }
        fs::remove_dir(retired).unwrap();
    }
    // Removing a parser input also removes its derived output on regeneration.
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "out") && !path.with_extension("i").exists() {
            assert!(update, "orphan golden output: {}", path.display());
            fs::remove_file(path).unwrap();
        }
    }
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "i"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty());
    let mut failed = Vec::new();
    for path in &paths {
        let src = fs::read_to_string(path).unwrap();
        let actual = render(&src);
        let out = path.with_extension("out");
        if update {
            fs::write(&out, &actual).unwrap();
            continue;
        }
        let expected = fs::read_to_string(&out).unwrap_or_default();
        if expected != actual {
            eprintln!("=== {} ===\n{actual}", path.display());
            failed.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert!(
        failed.is_empty(),
        "golden files differ: {failed:?} (run with UPDATE=1 to bless)"
    );
}
