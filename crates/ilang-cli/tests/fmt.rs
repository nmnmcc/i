//! `i fmt`: the files it takes, `--check`, and files with syntax errors.

use std::path::Path;
use std::process::{Command, Output};

const UGLY: &str = "main = () {\n  x = 1\n  x\n}\n";
const PRETTY: &str = "main = () {\n\tx = 1\n\tx\n}\n";

fn package(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

fn fmt(args: &[&str], project: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .arg("fmt")
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .unwrap()
}

fn read(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join(path)).unwrap()
}

#[test]
fn formats_src_and_test() {
    let dir = package(&[
        ("i.toml", "name = \"app\"\n"),
        ("src/main.i", UGLY),
        ("src/deep/more.i", UGLY),
        ("test/main_test.i", UGLY),
        ("other/skipped.i", UGLY),
    ]);
    let out = fmt(&[], dir.path());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(read(dir.path(), "src/main.i"), PRETTY);
    assert_eq!(read(dir.path(), "src/deep/more.i"), PRETTY);
    assert_eq!(read(dir.path(), "test/main_test.i"), PRETTY);
    assert_eq!(read(dir.path(), "other/skipped.i"), UGLY);
}

#[test]
fn check_lists_files_that_would_change() {
    let dir = package(&[
        ("i.toml", "name = \"app\"\n"),
        ("src/a.i", UGLY),
        ("src/b.i", PRETTY),
    ]);
    let out = fmt(&["--check"], dir.path());
    assert_eq!(out.status.code(), Some(1));
    let listed = String::from_utf8_lossy(&out.stdout);
    assert!(listed.contains("a.i"), "{listed}");
    assert!(!listed.contains("b.i"), "{listed}");
    assert_eq!(read(dir.path(), "src/a.i"), UGLY, "--check writes nothing");

    let out = fmt(&[], dir.path());
    assert!(out.status.success());
    let out = fmt(&["--check"], dir.path());
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
}

#[test]
fn paths_are_files_or_directories() {
    let dir = package(&[("one.i", UGLY), ("more/two.i", UGLY), ("three.i", UGLY)]);
    let one = dir.path().join("one.i");
    let more = dir.path().join("more");
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .arg("fmt")
        .arg(&one)
        .arg(&more)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(read(dir.path(), "one.i"), PRETTY);
    assert_eq!(read(dir.path(), "more/two.i"), PRETTY);
    assert_eq!(read(dir.path(), "three.i"), UGLY);
}

#[test]
fn files_with_errors_are_left_as_they_are() {
    let bad = /* old-syntax-check: E0101 */ "main = () {\n  x = \n}\n";
    let dir = package(&[("src/bad.i", bad), ("src/good.i", UGLY)]);
    let out = fmt(&[], dir.path());
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("bad.i:"), "{err}");
    assert!(err.contains("error[E"), "{err}");
    assert_eq!(read(dir.path(), "src/bad.i"), bad);
    assert_eq!(read(dir.path(), "src/good.i"), PRETTY);
}

#[test]
fn lisp_files_are_formatted_in_default_roots_and_given_paths() {
    let source = "// keep\n(= main (-> () _ {(= first 1) (= second 2) (+ first second)}))\n";
    let pretty = ilang_syntax::lisp::format(source).unwrap();
    assert!(pretty.lines().count() >= 6, "{pretty}");
    let dir = package(&[
        ("i.toml", "name = \"app\"\n"),
        ("src/main.is", source),
        ("test/deep/main_test.is", source),
        ("other/main.is", source),
    ]);
    let out = fmt(&["--check"], dir.path());
    assert_eq!(out.status.code(), Some(1));
    let listed = String::from_utf8_lossy(&out.stdout);
    assert!(listed.contains("src/main.is") && listed.contains("test/deep/main_test.is"));
    assert!(!listed.contains("other"));
    assert_eq!(read(dir.path(), "src/main.is"), source);
    assert!(fmt(&[], dir.path()).status.success());
    assert_eq!(read(dir.path(), "src/main.is"), pretty);
    assert_eq!(read(dir.path(), "test/deep/main_test.is"), pretty);
    assert_eq!(read(dir.path(), "other/main.is"), source);
    assert!(fmt(&["--check"], dir.path()).status.success());
    for path in [dir.path().join("other"), dir.path().join("other/main.is")] {
        std::fs::write(dir.path().join("other/main.is"), source).unwrap();
        assert!(fmt(&[path.to_str().unwrap()], dir.path()).status.success());
        assert_eq!(read(dir.path(), "other/main.is"), pretty);
    }
}

#[test]
fn conversion_to_lisp_uses_pretty_layout() {
    let source = "// keep\nmain = () {\n first = 1\n second = 2\n first + second\n}\n";
    let dir = package(&[("src/main.i", source)]);
    let out = fmt(&["--to", "is"], dir.path());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!dir.path().join("src/main.i").exists());
    let converted = read(dir.path(), "src/main.is");
    assert!(
        converted.lines().count() >= 6 && converted.contains("// keep"),
        "{converted}"
    );
    assert_eq!(ilang_syntax::lisp::format(&converted).unwrap(), converted);
    assert!(fmt(&["--check"], dir.path()).status.success());
}
