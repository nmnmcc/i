//! `i test` selects tests by `[test].collection`; a value that selects
//! nothing, or is not a string, is an error and never a green "0 passed".

#[path = "common/test_project.rs"]
mod test_project;
use std::path::Path;
use std::process::Command;

fn package(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    if files.iter().any(|(_, source)| source.contains("std/test")) {
        test_project::configure(dir.path());
    }
    dir
}

fn i(args: &[&str], project: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .unwrap()
}

/// A package with one passing test in `std/test.tests`.
fn tested() -> tempfile::TempDir {
    package(&[
        ("i.toml", "name = \"app\"\n"),
        ("src/lib.i", " (twice)\n\ntwice = (n : Int) { n * 2 }\n"),
        (
            "test/lib_test.i",
            "(\n\ttestlib = (test) = std/test\n\tapp/lib\n)\n\ntest(\"good\", () { lib.twice(2) |> (4) { Nil } & (_) { panic(\"no\") } })\n",
        ),
    ])
}

/// Sets `[test].collection` of the package to `value`.
fn set_collection(project: &Path, value: toml_edit::Item) {
    let path = project.join("i.toml");
    let mut manifest: toml_edit::DocumentMut =
        std::fs::read_to_string(&path).unwrap().parse().unwrap();
    manifest["test"]["collection"] = value;
    std::fs::write(path, manifest.to_string()).unwrap();
}

#[test]
fn a_missing_collection_is_an_error_not_zero_tests() {
    let dir = tested();
    set_collection(dir.path(), toml_edit::value("std/test.nothing"));
    let out = i(&["test"], dir.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    assert!(
        stderr.contains("there is no test collection \"std/test.nothing\""),
        "{stderr}"
    );
    assert!(!stdout.contains("passed"), "{stdout}");
}

#[test]
fn a_non_string_collection_is_rejected() {
    let dir = tested();
    for (value, kind) in [
        (toml_edit::value(1), "integer"),
        (
            toml_edit::value(toml_edit::Array::from_iter(["std/test.tests"])),
            "array",
        ),
    ] {
        set_collection(dir.path(), value);
        let out = i(&["test"], dir.path());
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
        assert!(
            stderr.contains(&format!(
                "error: `[test].collection` must be a string, not {kind}"
            )),
            "{stderr}"
        );
        assert!(!stdout.contains("passed"), "{stdout}");
    }
}

#[test]
fn the_configured_collection_still_runs_its_tests() {
    let dir = tested();
    let out = i(&["test"], dir.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("pass app/lib_test.good"), "{stdout}");
    assert!(stdout.contains("1 passed, 0 failed"), "{stdout}");
}
