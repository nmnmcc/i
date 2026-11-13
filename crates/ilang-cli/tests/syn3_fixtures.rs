//! The syntax-three fixtures of `tests/syn3`, read by the default parser.
//! The library is round-tripped through the syntax-three printer first,
//! preserving the source tree and comments.
//!
//! A fixture is a package directory with any of:
//!
//! - `expected.txt`: the lines that `i run` prints;
//! - `types.txt`: lines that `i check --types` prints (others may be there);
//! - `errors.txt`: `i check` fails, and each line is in its output.

use ilang_syntax::lexer::lex3;
use ilang_syntax::syn3;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "i") {
            out.push(path);
        }
    }
}

/// Checks the library against the syntax-three printer.
fn round_trip_library() {
    let lib = repository().join("lib");
    let mut files = Vec::new();
    sources(&lib, &mut files);
    let parsed: Vec<_> = files
        .iter()
        .map(|path| {
            let source = std::fs::read_to_string(path).unwrap();
            let (module, diags) = syn3::parse_module(&source);
            assert!(diags.is_empty(), "{}: {diags:?}", path.display());
            (path, source, module)
        })
        .collect();
    for (path, source, module) in &parsed {
        let lexed = lex3(source);
        let text = syn3::print_with_comments(module, source, &lexed.comments);
        let (round_trip, diags) = syn3::parse_module(&text);
        assert!(diags.is_empty(), "{}: {diags:?}", path.display());
        assert_eq!(&round_trip, module, "{}", path.display());
    }
}

/// Runs `i` on a fixture: whether it succeeded, its output, and both
/// streams together.
fn i(args: &[&str], project: &Path) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let all = format!("{stdout}{}", String::from_utf8_lossy(&output.stderr));
    (output.status.success(), stdout, all)
}

#[test]
fn syntax_three_fixtures() {
    round_trip_library();
    let mut fixtures: Vec<PathBuf> = std::fs::read_dir(repository().join("tests/syn3"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    fixtures.sort();
    assert!(!fixtures.is_empty());
    let mut problems = Vec::new();
    for dir in &fixtures {
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let read = |file: &str| std::fs::read_to_string(dir.join(file)).ok();
        if let Some(errors) = read("errors.txt") {
            let (ok, _, all) = i(&["check"], dir);
            if ok {
                problems.push(format!("{name}: `i check` passed, but errors are expected"));
            }
            for line in errors.lines().filter(|l| !all.contains(l)) {
                problems.push(format!("{name}: missing error `{line}` in:\n{all}"));
            }
        }
        if let Some(types) = read("types.txt") {
            let (ok, _, all) = i(&["check", "--types"], dir);
            if !ok {
                problems.push(format!("{name}: `i check --types` failed:\n{all}"));
            }
            for line in types.lines().filter(|l| !all.lines().any(|x| x == *l)) {
                problems.push(format!("{name}: missing type `{line}` in:\n{all}"));
            }
        }
        if let Some(expected) = read("expected.txt") {
            let (ok, stdout, all) = i(&["run"], dir);
            if !ok || stdout.trim_end() != expected.trim_end() {
                problems.push(format!(
                    "{name}: `i run` gave:\n{all}\nexpected:\n{expected}"
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n\n"));
}
