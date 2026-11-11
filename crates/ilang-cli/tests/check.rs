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
    for target in [ilang_wasm::Target::Native, ilang_wasm::Target::Wasi] {
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
fn a_local_annotation_cannot_hide_an_entry_service() {
    // §4.8 retains the service through the local binding; §10.2 requires
    // a provider before entry. Both annotations have the same finite type.
    let annotated = "S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
@main
main(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }";
    for source in [
        annotated.to_string(),
        annotated.replace("local: () -> Nil ! N | S(N)", "local"),
    ] {
        let dir = front_end_project(&source);
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["check", "--types", "--project"])
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("app/main.main : () -> Nil ! N | S(N)")
        );
        for opt in ["0", "2"] {
            let out = Command::new(env!("CARGO_BIN_EXE_i"))
                .args(["run", "--opt", opt, "--project"])
                .arg(dir.path())
                .output()
                .unwrap();
            let err = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(1), "{err}");
            assert!(
                err.contains("error[E0523]") && !err.contains("trap"),
                "{err}"
            );
        }
    }
}

#[test]
fn a_captured_service_row_keeps_callback_dependencies() {
    // §4.5 rule 4 keeps S's argument equal across levels; §4.8 rule 9
    // removes the supplied S(Dependency), leaving Dependency itself.
    let source = "S(N) { S(() -> Nil ! N) }
Dependency { Dependency }
read = (d ! Dependency) { Nil }
use(N) = (s ! S(N)) { S(f) = s f() }
apply(N) = () { local: () -> Nil ! N = () { use() } local() }
@main
main = () { ! S(Dependency) = S(() { read() }) apply() }";
    for supplied in [false, true] {
        let source = if supplied {
            source.replace(
                "! S(Dependency)",
                "! Dependency = Dependency\n! S(Dependency)",
            )
        } else {
            source.to_string()
        };
        let dir = front_end_project(&source);
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .args(["check", "--types", "--project"])
            .arg(dir.path())
            .output()
            .unwrap();
        let main = if supplied {
            "app/main.main : () -> Nil\n"
        } else {
            "app/main.main : () -> Nil ! Dependency\n"
        };
        assert_eq!(
            out.status.success(),
            true,
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            String::from_utf8_lossy(&out.stdout).contains(main),
            "{}",
            String::from_utf8_lossy(&out.stdout)
        );
        for opt in ["0", "2"] {
            let out = Command::new(env!("CARGO_BIN_EXE_i"))
                .args(["run", "--opt", opt, "--project"])
                .arg(dir.path())
                .output()
                .unwrap();
            let err = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.success(), supplied, "{err}");
            if !supplied {
                assert!(
                    err.contains("error[E0523]") && !err.contains("trap"),
                    "{err}"
                );
            }
        }
    }
}

#[test]
fn lowering_rejects_an_uncoalesced_partial_result() {
    // §4.7 prohibits recursive types even when a caller of the compiler
    // API accidentally ignores the checker diagnostics.
    let dir = front_end_project("bad = (x) { x(x) }\n@main\nmain = () { Nil }");
    let (graph, _) = ilang_cli::load(dir.path(), false, true);
    let checked = ilang_check::check(&graph);
    assert!(checked.diags.iter().any(|d| d.code == "E0505"));
    assert!(
        !checked
            .types
            .as_ref()
            .unwrap()
            .coalescing_errors()
            .is_empty()
    );
    let result = ilang_wasm::compile_wat(
        &graph,
        checked.program.as_ref().unwrap(),
        checked.types.as_ref().unwrap(),
        ilang_wasm::Options {
            tests: false,
            target: ilang_wasm::Target::Native,
        },
    );
    assert!(
        matches!(result, Err(ilang_wasm::Error::Diagnostics(ds)) if ds.iter().any(|d| d.code == "E0505"))
    );
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
