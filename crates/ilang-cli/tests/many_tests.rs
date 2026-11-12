use std::process::Command;

#[test]
fn collected_tests_share_one_build() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["test", "--project", "tests/perf/many-tests", "--opt", "0"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("2000 passed, 0 failed"));
}

#[test]
fn ordinary_and_collected_tests_share_summary_and_filter() {
    let dir = tempfile::tempdir().unwrap();
    let std = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std");
    std::fs::write(dir.path().join("i.toml"), format!("name = \"mixed\"\n[test]\ncollection = \"std/test.tests\"\n[dev-dependencies]\nstd = {{ path = {:?} }}\n", std.to_str().unwrap())).unwrap();
    std::fs::create_dir(dir.path().join("test")).unwrap();
    std::fs::write(dir.path().join("test/mixed.i"), "(testlib = (test, skip, fails) = std/test)\n\ntest(\"ordinary\", () { assert(True) })\ntest(\"collected\", () { assert(True) })\nskip(\"ignored\", () { assert(False) })\nfails(\"expected\", () { assert(False) })\n").unwrap();
    let run = |filter: Option<&str>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_i"));
        command
            .args(["test", "--project"])
            .arg(dir.path())
            .args(["--opt", "0"]);
        if let Some(filter) = filter {
            command.arg(filter);
        }
        command.output().unwrap()
    };
    let output = run(None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pass mixed/mixed.ordinary"), "{stdout}");
    assert!(stdout.contains("3 passed, 0 failed, 1 skipped"), "{stdout}");
    let output = run(Some("collected"));
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed, 0 failed"));
}
