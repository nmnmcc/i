//! Unused-binding warnings are nonfatal: a warning must leave `i check` successful.

/// The `[dependencies]` table with the standard library `i` of this
/// repository, which every root package needs (E0641).
fn library_dependencies() -> String {
    let library = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../stdlib")
        .canonicalize()
        .unwrap();
    format!(
        "[dependencies]\ni = {{ path = {:?} }}\n",
        library.to_str().unwrap()
    )
}

#[test]
fn unused_binding_warning_keeps_check_successful() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        format!(
            "name = \"app\"\nhostcall = true\n{}",
            library_dependencies()
        ),
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("test")).unwrap();
    std::fs::write(
        dir.path().join("test/forgotten.i"),
        "forgotten = () { assert(True()) }\n",
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
