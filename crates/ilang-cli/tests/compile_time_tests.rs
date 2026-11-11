//! Compile-time bindings are selected by ordinary dependency reachability.
use std::path::Path;

fn write(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

fn build(
    root: &Path,
    tests: bool,
    opt: ilang_cli::Level,
) -> Result<ilang_cli::Built, ilang_cli::BuildError> {
    ilang_cli::build(
        root,
        ilang_cli::BuildOptions {
            tests,
            opt,
            offline: true,
            target: ilang_cli::Target::Ilang,
        },
    )
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "i.toml", "name = \"app\"\n");
    write(
        dir.path(),
        "src/helpers.is",
        "(, quote)\n(= quote (-> () _ `(f x)))\n",
    );
    write(
        dir.path(),
        "src/forward.i",
        "(app/helpers)\n(forward)\nforward = () { helpers.quote() }\n",
    );
    dir
}

#[test]
fn ordinary_roots_and_initializers_cannot_reach_compile_time_bindings() {
    let dir = fixture();
    let root = dir.path();
    write(root, "src/main.i", "main = () { Nil }\n");
    let built = build(root, false, ilang_cli::Level::O0).unwrap();
    assert!(matches!(
        built
            .host_program()
            .unwrap()
            .run_entry(built.host_options())
            .unwrap(),
        ilang_host::Outcome::Ok(_)
    ));

    write(
        root,
        "src/main.i",
        "(app/forward)\nmain = () { forward.forward() Nil }\n",
    );
    let error = build(root, false, ilang_cli::Level::O0)
        .err()
        .expect("ordinary entry must be rejected");
    assert!(error.to_string().contains("compiler-owned"), "{error}");

    write(
        root,
        "src/main.i",
        "(app/forward)\nvalue = forward.forward()\nmain = () { Nil }\n",
    );
    let error = build(root, false, ilang_cli::Level::O0)
        .err()
        .expect("ordinary entry must be rejected");
    assert!(error.to_string().contains("initialization"), "{error}");

    // The service functions themselves also make a binding compile-time, even
    // when it contains no quote.
    write(
        root,
        "src/syntax_helper.i",
        "(syntax = i/syntax)\n(helper)\nhelper = () { syntax.show }\n",
    );
    write(
        root,
        "src/main.i",
        "(app/syntax_helper)\nmain = () { syntax_helper.helper() Nil }\n",
    );
    let error = build(root, false, ilang_cli::Level::O0)
        .err()
        .expect("syntax helper must be rejected");
    assert!(error.to_string().contains("compiler-owned"), "{error}");

    write(
        root,
        "i.toml",
        "name = \"app\"\n[wasm]\nexports = [\"app/main.entry\"]\n",
    );
    write(
        root,
        "src/main.i",
        "(app/forward)\n(entry)\nentry = () { forward.forward() Nil }\n",
    );
    let error = build(root, false, ilang_cli::Level::O0)
        .err()
        .expect("ordinary entry must be rejected");
    assert!(error.to_string().contains("compiler-owned"), "{error}");
}

#[test]
fn tests_use_each_builds_unmarked_quote_templates() {
    for opt in [ilang_cli::Level::O0, ilang_cli::Level::O2] {
        let dir = fixture();
        let root = dir.path();
        // The ordinary entry is not executed by i test and may itself use the
        // same helper while the test instance has the syntax service.
        write(
            root,
            "src/main.i",
            "(app/forward)\nmain = () { forward.forward() Nil }\n",
        );
        write(
            root,
            "test/check.i",
            "(app/forward, syntax = i/syntax)\n@test\ncheck = () { assert(syntax.show(forward.forward()) == \"(f x)\") Nil }\n",
        );
        write(
            root,
            "test/quote.is",
            "(, (= syntax (/ i syntax)))\n(= quoted `(a b))\n(@ test (= quote_test (-> () _ { (assert (== ((. syntax show) quoted) \"(a b)\")) Nil })))\n",
        );
        let built = build(root, true, opt).unwrap();
        assert!(built.templates.len() >= 2);
        assert!(built.templates.iter().all(|s| s.scope().marks.is_empty()));
        let modules: std::collections::HashSet<_> =
            built.templates.iter().map(|s| s.scope().module).collect();
        assert_eq!(modules.len(), 2, "templates must keep their source module");
        let program = built.host_program().unwrap();
        let names: Vec<_> = program.abi().tests.iter().map(|t| t.name.clone()).collect();
        assert_eq!(names.len(), 2, "{names:?}");
        for name in names {
            let result = ilang_cli::run_test_with(&program, &name, built.host_options());
            assert!(
                matches!(result, ilang_host::TestResult::Passed),
                "{name}: {result:?}"
            );
        }
        if opt == ilang_cli::Level::O0 {
            let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
                .args(["test", "--offline", "--project"])
                .arg(root)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("2 passed, 0 failed"));
        }
    }
}
