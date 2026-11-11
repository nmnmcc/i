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
