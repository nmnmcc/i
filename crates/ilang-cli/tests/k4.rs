//! Review programs exercise the checker and the actual runtime boundary.
use std::process::Command;

fn project(source: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    dir
}

fn check_and_run(name: &str, source: &str, error: Option<&str>, stdout: &str) {
    let dir = project(source);
    for args in [
        vec!["check", "--types"],
        vec!["run", "--opt", "0"],
        vec!["run", "--opt", "2"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_i"))
            .args(&args)
            .arg("--project")
            .arg(dir.path())
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !err.contains("trap") && !err.contains("panicked"),
            "{name} {args:?}: {err}"
        );
        match error {
            Some(code) => {
                assert!(
                    !out.status.success() && err.contains(code),
                    "{name} {args:?}: {text}\n{err}"
                );
            }
            None => {
                assert!(out.status.success(), "{name} {args:?}: {text}\n{err}");
                if args[0] == "run" {
                    assert_eq!(text, stdout, "{name} {args:?}");
                }
            }
        }
    }
}

#[test]
fn same_service_obligations_survive_annotations() {
    // §4.8 rule 9 and §9.2: replacing S(Dep) cannot handle S(Never),
    // nor can replacing Svc(String) handle a body that requires Svc(Int).
    for name in ["x1", "x2", "x3", "x4", "x6", "x7", "x8", "z1", "z4"] {
        let source =
            std::fs::read_to_string(format!("{}/tests/k4/{name}.i", env!("CARGO_MANIFEST_DIR")))
                .unwrap();
        check_and_run(name, &source, Some("E0515"), "");
    }
}

#[test]
fn rows_inside_type_arguments_keep_service_arguments_equal() {
    // §4.5's row ruling applies inside T's argument. These callbacks need
    // a different S form or an unprovided Dep; neither may reach runtime.
    for name in [
        "g_ann_SDep",
        "g_inf_SDep",
        "m3",
        "m4",
        "r6",
        "b4",
        "b7",
        "r2",
        "r3",
    ] {
        let source =
            std::fs::read_to_string(format!("{}/tests/k4/{name}.i", env!("CARGO_MANIFEST_DIR")))
                .unwrap();
        check_and_run(
            name,
            &source,
            Some(if ["b4", "b7", "r2", "r3"].contains(&name) {
                "E0522"
            } else {
                "E0515"
            }),
            "",
        );
    }
}

#[test]
fn field_access_has_the_same_finite_type_as_a_pattern() {
    // §4.8 rules 4, 5, 7, 13: reading the field retains N | S(N).
    // Install S(Never), whose callback returns 1, and invoke the helper.
    for n in 21..=40 {
        let path = format!("{}/tests/k4/q{n}.i", env!("CARGO_MANIFEST_DIR"));
        let Ok(mut source) = std::fs::read_to_string(path) else {
            continue;
        };
        source.push_str("\n@main\nmain = () { ! S(Never) = S(() { 1 }) assert(fw() == 1) }\n");
        check_and_run(&format!("q{n}"), &source, None, "");
    }
    let source = include_str!("k4/r4.i");
    check_and_run("r4", source, None, "four\nthree\n");
}

#[test]
fn generic_handlers_have_finite_types_with_or_without_annotations() {
    // §4.8 rule 9 removes S(N), retaining N and its same-form obligation.
    // The body and the supplied S(Never) both return 1.
    for n in 1..=16 {
        let mut source =
            std::fs::read_to_string(format!("{}/tests/k4/w{n}.i", env!("CARGO_MANIFEST_DIR")))
                .unwrap();
        let service = if n >= 9 {
            "provided = S(() { 1 }, (_: Never) { Nil })"
        } else {
            "provided = S(() { 1 })"
        };
        let service = service.strip_prefix("provided = ").unwrap();
        let call = match n {
            1 | 2 | 9 | 10 => "wrap(() { use() })".to_string(),
            3 | 4 | 5 | 11 | 12 | 13 => "wrap(() { use() }, VALUE)".replace("VALUE", service),
            _ => "wrap(VALUE)".replace("VALUE", service),
        };
        source.push_str(&"\n@main\nmain = () { assert(CALL == 1) }\n".replace("CALL", &call));
        check_and_run(&format!("w{n}"), &source, None, "");
    }
}
