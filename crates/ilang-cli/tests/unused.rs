//! Scope UNUSED rule 1: a warning must leave `i check` successful.
#[test]
fn unused_binding_warning_keeps_check_successful() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("test")).unwrap();
    std::fs::write(
        dir.path().join("test/forgotten.i"),
        "forgotten = () { assert(True) }\n",
    )
    .unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--offline", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        String::from_utf8(result.stderr).unwrap(),
        "test/forgotten.i:1:1: warning[W0504]: top-level binding `forgotten` is not used; did you forget to register this function with `test`?\n"
    );
}
