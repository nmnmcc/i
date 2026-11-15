use std::process::Command;

#[test]
fn lisp_modules_are_checked_with_source_locations() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/math.is"),
        "(, twice)\n(= twice (-> (, x) _ (* x 2)))\n",
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
fn conversion_refuses_an_unplaced_comment_and_keeps_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sample.i");
    let source = "f = () { 1 // trailing\n}\n";
    std::fs::write(&file, source).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=is"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let errors = String::from_utf8_lossy(&out.stderr);
    assert!(errors.contains("sample.i:1:12:"), "{errors}");
    assert!(errors.contains("trailing"), "{errors}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), source);
    assert!(!file.with_extension("is").exists());
}

#[test]
fn conversion_keeps_an_existing_destination() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sample.i");
    std::fs::write(&file, "f = () { 1 }\n").unwrap();
    let lisp = file.with_extension("is");
    std::fs::write(&lisp, "existing = 42\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=is"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "f = () { 1 }\n");
    assert_eq!(std::fs::read_to_string(&lisp).unwrap(), "existing = 42\n");
}

#[test]
fn conversion_does_not_write_an_unrepresentable_module() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sample.is");
    // A macro definition has no normal spelling, including one with no
    // quote in its body.
    let source = "(@ macro (= f (-> (, x) _ x)))\n";
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
fn conversion_accepts_a_block_containing_a_function_and_value() {
    // The former unrepresentable fixture is now a normal block: every
    // expression `{ … }` is a computation.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sample.is");
    let source = "(= f (-> (,) _ { (consume { (-> (, x) Int { x }) 1 }) }))\n";
    std::fs::write(&file, source).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=i"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!file.exists());
    let normal = file.with_extension("i");
    assert!(normal.exists());
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--check"])
        .arg(&normal)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn interpolation_keeps_its_result_through_both_surface_conversions() {
    // Interpolation becomes concatenation before the core tree.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    let mut file = dir.path().join("src/main.i");
    std::fs::write(
        &file,
        "f = (x : String) { \"a\\(x)b\" }\nmain = () { assert(f(\"c\") == \"acb\") }\n",
    )
    .unwrap();
    for target in ["is", "i"] {
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["fmt", "--to", target])
            .arg(&file)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        file = file.with_extension(target);
        assert!(file.exists());
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["run", "--opt", "0", "--project"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn macro_quote_formats_in_lisp_and_cannot_convert_to_normal() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("macros.is");
    std::fs::write(&file, "(@ macro (= wrap (-> (, x) _ `(f ~x))))\n").unwrap();
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

#[test]
fn a_failed_write_lists_converted_failed_and_untouched_files() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("a.i");
    // A name whose temporary file cannot be created (NAME_MAX), so the write
    // fails in the middle of the batch after `first` succeeded.
    let middle = dir.path().join(format!("{}.i", "m".repeat(245)));
    let last = dir.path().join("z.i");
    for file in [&first, &middle, &last] {
        std::fs::write(file, "f = () { 1 }\n").unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=is"])
        .arg(&first)
        .arg(&middle)
        .arg(&last)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let errors = String::from_utf8_lossy(&output.stderr);
    let first_destination = first.with_extension("is");
    let middle_destination = middle.with_extension("is");

    // Group one: already converted, target written and source removed.
    assert!(errors.contains("already converted in this run"), "{errors}");
    assert!(
        errors.contains(&format!(
            "{} -> {}",
            first.display(),
            first_destination.display()
        )),
        "{errors}"
    );
    assert!(!first.exists());
    assert_eq!(
        std::fs::read_to_string(&first_destination).unwrap(),
        "(= f (-> (,) _ 1))\n"
    );

    // Group two: the failing file, with the state of both of its paths.
    assert!(errors.contains("failed:"), "{errors}");
    assert!(errors.contains(&middle.display().to_string()), "{errors}");
    assert!(errors.contains("source is unchanged"), "{errors}");
    assert!(errors.contains("destination was not written"), "{errors}");
    assert!(middle.exists());
    assert!(!middle_destination.exists());

    // Group three: untouched files.
    assert!(errors.contains("not processed"), "{errors}");
    assert!(errors.contains(&last.display().to_string()), "{errors}");
    assert!(last.exists());
    assert!(!last.with_extension("is").exists());
}

#[test]
fn a_refused_batch_lists_every_file_as_not_converted_to_lisp() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("a.i");
    let bad = dir.path().join("b.i");
    let last = dir.path().join("c.i");
    std::fs::write(&first, "f = () { 1 }\n").unwrap();
    std::fs::write(&bad, "g = () { 2 // trailing\n}\n").unwrap();
    std::fs::write(&last, "h = () { 3 }\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=is"])
        .arg(&first)
        .arg(&bad)
        .arg(&last)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(errors.contains("no files were converted"), "{errors}");
    for file in [&first, &bad, &last] {
        assert!(errors.contains(&file.display().to_string()), "{errors}");
        assert!(file.exists());
        assert!(!file.with_extension("is").exists());
    }
}

#[test]
fn a_refused_batch_lists_every_file_as_not_converted_to_normal() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("a.is");
    // A macro definition has no normal spelling.
    let bad = dir.path().join("b.is");
    let last = dir.path().join("c.is");
    std::fs::write(&first, "(= f (-> (,) _ 1))\n").unwrap();
    std::fs::write(&bad, "(@ macro (= f (-> (, x) _ x)))\n").unwrap();
    std::fs::write(&last, "(= h (-> (,) _ 3))\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["fmt", "--to=i"])
        .arg(&first)
        .arg(&bad)
        .arg(&last)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(errors.contains("no files were converted"), "{errors}");
    for file in [&first, &bad, &last] {
        assert!(errors.contains(&file.display().to_string()), "{errors}");
        assert!(file.exists());
        assert!(!file.with_extension("i").exists());
    }
}
