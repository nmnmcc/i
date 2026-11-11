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

#[test]
fn formatter_converts_files_both_ways_and_keeps_boundary_comments() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("sample.i");
    std::fs::write(
        &original,
        "// item\nf = () {\n  // statement\n  x = 1\n  x\n}\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=is"])
        .arg(&original)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let lisp = original.with_extension("is");
    assert!(!original.exists());
    let text = std::fs::read_to_string(&lisp).unwrap();
    assert!(
        text.contains("// item") && text.contains("// statement"),
        "{text}"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=i"])
        .arg(&lisp)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!lisp.exists());
    let text = std::fs::read_to_string(&original).unwrap();
    assert!(
        text.contains("// item") && text.contains("// statement"),
        "{text}"
    );
}

#[test]
fn conversion_reports_unplaced_comments_and_keeps_existing_destination() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sample.i");
    std::fs::write(&file, "f = () { 1 // trailing\n}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=is"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("comment could not be preserved"));
    let lisp = file.with_extension("is");
    std::fs::write(&file, "existing = 42\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=i"])
        .arg(&lisp)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(file).unwrap(), "existing = 42\n");
    assert!(lisp.exists());
}

#[test]
fn conversion_does_not_write_an_unrepresentable_module() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sample.is");
    let source = "(= f (-> () _ { (consume { (-> (x) Int { x }) 1 }) }))\n";
    std::fs::write(&file, source).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=i"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), source);
    assert!(!file.with_extension("i").exists());
}

#[test]
fn macro_quote_formats_in_lisp_and_cannot_convert_to_normal() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("macros.is");
    std::fs::write(&file, "(@ macro (= wrap (-> (x) _ `(f ~x))))\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .arg("fmt")
        .arg(&file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let before = std::fs::read_to_string(&file).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=i"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    assert!(!file.with_extension("i").exists());
}
