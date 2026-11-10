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
        "app/main.id : (x: A) -> A\napp/main.elapsed : (start: Int) -> Int ! Clock\n"
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

#[test]
fn blind_expected_subject_is_checked_before_audit_codegen() {
    // S4 ruling 2: even the old-rule audit must report a normal diagnostic
    // when it cannot give this new match subject its expected type.
    let source = include_str!("../../../tests/spec/6/a1_fitting_expected_subject.i");
    let dir = front_end_project(source);
    let (graph, diags) = ilang_cli::load(dir.path(), true, true);
    assert!(!diags.iter().any(ilang_package::Diagnostic::is_error));
    let checked = ilang_check::check(&graph);

    assert!(
        !checked
            .diags
            .iter()
            .any(ilang_package::Diagnostic::is_error)
    );
    for target in [ilang_wasm::Target::Ilang, ilang_wasm::Target::Wasi] {
        ilang_wasm::compile_wat(
            &graph,
            checked.program.as_ref().unwrap(),
            checked.types.as_ref().unwrap(),
            ilang_wasm::Options {
                tests: true,
                target,
            },
        )
        .unwrap();
    }
}

fn front_end_project(source: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    dir
}

#[test]
fn new_syntax_codegen_keeps_known_functions_and_pattern_stop_positions() {
    // A1 §3.9 / spec §6.12: a top-level clause block is a known function,
    // and a failing one-clause match reports at p, adding no match frame.
    let source = "f = { 0 -> 1  _ -> 2 }\nget = (o) { o |> { Ok(x) -> x } }\n@main\nmain = () { assert(f(1) == 2)  get(Error(Nil))  Nil }\n";
    let dir = front_end_project(source);
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["build", "--wat", "--opt", "0", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let wat = std::fs::read_to_string(dir.path().join("build/app.wat")).unwrap();
    let start = wat.find(";; app/main.f\n(func $f").unwrap();
    let id = wat[start..]
        .split("(func ")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();
    assert!(
        wat.contains(&format!("(call {id} ")),
        "f must have direct callers"
    );
    for opt in ["0", "2"] {
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["run", "--opt", opt, "--project"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(!out.status.success());
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(
            err.contains("pattern Ok(x) did not match Error(Nil)"),
            "{err}"
        );
        let (line, column) = ilang_syntax::span::LineIndex::new(source)
            .line_col(source.find("Ok(x)").unwrap() as u32);
        assert!(err.contains(&format!("main:{line}:{column}")), "{err}");
        assert_eq!(
            err.lines()
                .filter(|line| line.starts_with("app/main:"))
                .count(),
            2,
            "{err}"
        );
    }
}

#[test]
fn old_forms_have_helpful_errors_without_reserving_names() {
    // A3 ruling 1: valid new calls stay calls; hints belong to errors only.
    for (source, hint) in [
        ("export(x)", "header groups"),
        ("export()", "old empty header, omit it"),
        ("import()", "old empty header, omit it"),
        ("x = case(1)", "e |>"),
        ("import(i/core)", "header groups"),
        (
            /* old-syntax-check: E0101 */ "x = case 1 { 0 -> 1  _ -> 2 }",
            "e |>",
        ),
        (
            /* old-syntax-check: E0111,E0101 */ "x = 1 |> { n if True -> n }",
            "? condition",
        ),
        ("id = (x: a) { x }", "error[E0201]: `a` is not defined"),
    ] {
        let dir = front_end_project(source);
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["check", "--project"])
            .arg(dir.path())
            .output()
            .unwrap();
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(!out.status.success(), "{source}");
        assert!(err.contains(hint), "{source}: {err}");
    }
    let dir = front_end_project(
        "import = (x) { x }\nexport = import\ncase = export\nif = case\nx = if(1)\n",
    );
    let out = Command::new(env!("CARGO_BIN_EXE_i"))
        .args(["check", "--project"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn migration_flags_are_removed() {
    for flag in [
        "--from-old",
        "--keep-layout",
        "--report",
        "--no-choices-split",
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["fmt", flag])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("unexpected argument"));
    }
}

#[test]
fn disjoint_comparisons_are_warnings_in_check_and_test() {
    // W520/§6.9: warning-only programs succeed; both comparisons have Bool.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir_all(dir.path().join("test")).unwrap();
    std::fs::write(
        dir.path().join("test/comparison.i"),
        "value = 1 == 2\n@test\ncomparison = () { assert(1 != 2) }\n",
    )
    .unwrap();
    for args in [
        vec!["check", "--types"],
        vec!["test", "--opt", "0"],
        vec!["test", "--opt", "2"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .args(&args)
            .arg("--project")
            .arg(dir.path())
            .output()
            .unwrap();
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(out.status.success(), "{args:?}: {err}");
        assert_eq!(err.matches("warning[W0503]").count(), 2, "{err}");
        assert!(!err.contains("error["), "{err}");
        let stdout = String::from_utf8(out.stdout).unwrap();
        if args[0] == "check" {
            assert!(stdout.contains("app/comparison.value : Bool"), "{stdout}");
        } else {
            assert!(stdout.contains("1 passed"), "{stdout}");
        }
    }
}
