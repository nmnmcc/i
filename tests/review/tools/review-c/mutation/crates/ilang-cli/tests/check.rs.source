//! `i check --types` prints the type of every top-level name.

use std::process::Command;

#[test]
fn check_types() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        "id = (x) { x }\nClock { Clock(now: () -> Int) }\nelapsed = (start: Int, clock ! Clock) { clock.now() - start }\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--types", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "app/main.id : (x: a) -> a\napp/main.elapsed : (start: Int) -> Int ! Clock\n"
    );
}

#[test]
fn check_reports_type_errors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), "x: String = 1\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("error[E0501]"), "{err}");
}
