use std::process::Command;

#[test]
fn lisp_modules_are_checked_with_source_locations() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/math.is"),
        "(, twice)\n(= twice (-> (x) _ (* x 2)))\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "(app/math)\nvalue = math.twice(3)\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    std::fs::write(
        dir.path().join("src/math.is"),
        "(, twice)\n(: twice String)\n(= twice 1)\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(errors.contains("math.is:3:"), "{errors}");

    std::fs::write(dir.path().join("src/math.i"), "twice = (x) { x * 2 }\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E0624"));
}
