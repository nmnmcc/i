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
    let source = "// keep\n(= main (-> (,) _ {(= first 1) (= second 2) (= third 3) (+ first (+ second third))}))\n";
    let pretty = ilang_syntax::lisp::format(source).unwrap();
    assert_eq!(
        pretty,
        "// keep\n(= main\n  (-> (,) _ {(= first 1) (= second 2) (= third 3) (+ first (+ second third))}))\n"
    );
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
    let source =
        "// keep\nmain = () {\n first = 1\n second = 2\n third = 3\n first + second + third\n}\n";
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
        converted.lines().count() >= 3
            && converted.contains("// keep")
            && converted
                .lines()
                .all(|l| !l.trim_start().starts_with([')', '}']) && !l.contains('\t')),
        "{converted}"
    );
    assert_eq!(ilang_syntax::lisp::format(&converted).unwrap(), converted);
    assert!(fmt(&["--check"], dir.path()).status.success());
}

#[test]
fn check_reports_missing_blank_lines_after_header_groups() {
    let normal = "(\n\ti/core\n)\n(\n\tvalue\n)\nvalue = 1\n";
    let lisp = "(, (/ i core))\n(, value)\n(= value 1)\n";
    let dir = package(&[("src/normal.i", normal), ("src/lisp.is", lisp)]);
    let out = fmt(&["--check"], dir.path());
    assert_eq!(out.status.code(), Some(1));
    let listed = String::from_utf8_lossy(&out.stdout);
    assert!(
        listed.contains("normal.i") && listed.contains("lisp.is"),
        "{listed}"
    );
    assert_eq!(read(dir.path(), "src/normal.i"), normal);
    assert_eq!(read(dir.path(), "src/lisp.is"), lisp);
    assert!(fmt(&[], dir.path()).status.success());
    assert_eq!(
        read(dir.path(), "src/normal.i"),
        "(\n\ti/core\n)\n\n(\n\tvalue\n)\n\nvalue = 1\n"
    );
    assert_eq!(
        read(dir.path(), "src/lisp.is"),
        "(, (/ i core))\n\n(, value)\n\n(= value 1)\n"
    );
    assert!(fmt(&["--check"], dir.path()).status.success());
}

#[test]
fn constructor_values_and_patterns_are_written_as_calls() {
    let old = "f = (b : Bool) -> Int { b |> (True) { 1 } & (False) { 0 } }\ng = () { f(True) }\n";
    let called = "g = () { True() }\n";
    let dir = package(&[("src/old.i", old), ("src/called.i", called)]);
    let out = fmt(&[], dir.path());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let new = read(dir.path(), "src/old.i");
    assert!(
        new.contains("(True())") && new.contains("(False())") && new.contains("f(True())"),
        "{new}"
    );
    assert!(!new.contains("()()"), "{new}");
    let called_new = read(dir.path(), "src/called.i");
    assert!(called_new.contains("True()") && !called_new.contains("()()"), "{called_new}");
    assert!(fmt(&["--check"], dir.path()).status.success(), "a second format changes nothing");
    assert!(fmt(&[], dir.path()).status.success());
    assert_eq!(read(dir.path(), "src/old.i"), new);
    assert_eq!(read(dir.path(), "src/called.i"), called_new);
}

#[test]
fn conversion_to_lisp_writes_constructor_values_as_calls() {
    let source = "x = True\ny = Some(1)\nf = (b : Bool) -> Int { b |> (True) { 1 } & (False) { 0 } }\n";
    let dir = package(&[("src/main.i", source)]);
    let out = fmt(&["--to", "is"], dir.path());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let converted = read(dir.path(), "src/main.is");
    assert!(converted.contains("(= x (True))"), "{converted}");
    assert!(converted.contains("(Some 1)") && !converted.contains("((Some)"), "{converted}");
    assert!(converted.contains("(True)") && converted.contains("(False)"), "{converted}");
    assert!(fmt(&["--check"], dir.path()).status.success());
}
