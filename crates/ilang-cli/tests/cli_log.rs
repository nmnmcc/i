use std::{path::Path, process::Command};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["run", "--opt", "0", "--project"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/cli-log"))
        .arg("--")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn valid_arguments() {
    let out = run(&["--name", "world", "--count", "2"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(out.stdout, b"hello world\nhello world\ncli ok\n");
    assert!(String::from_utf8_lossy(&out.stderr).contains("info \"greeting\""));
}

#[test]
fn invalid_arguments() {
    let out = run(&["--nope"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("Usage:"));
}
