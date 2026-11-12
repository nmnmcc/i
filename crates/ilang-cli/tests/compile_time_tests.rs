//! Compile-time bindings are selected by ordinary dependency reachability.
#[path = "common/test_project.rs"]
mod test_project;

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
    let options = ilang_cli::BuildOptions {
        tests,
        opt,
        offline: true,
        target: ilang_cli::Target::Native,
    };
    if tests {
        return ilang_cli::build_tests(root, options, Some("std/test.tests"))
            .map(|(built, _)| built);
    }
    ilang_cli::build(
        root,
        ilang_cli::BuildOptions {
            tests,
            opt,
            offline: true,
            target: ilang_cli::Target::Native,
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
    let program = built.host_program().unwrap();
    assert!(
        program
            .abi()
            .externals
            .keys()
            .all(|namespace| !namespace.starts_with("i:compiler/"))
    );
    assert!(matches!(
        program.run_entry(built.host_options()).unwrap(),
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
        test_project::configure(root);
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
            "(\n\ttestlib = (test) = std/test\n\tapp/forward, syntax = i/syntax)\n\ntest(\"check\", () { assert(syntax.show(forward.forward()) == \"(f x)\") Nil })\n",
        );
        write(
            root,
            "test/quote.is",
            "(, (= syntax (/ i syntax)) (= testlib (, test) (/ std test)))\n(= quoted `(a b))\n(test \"quote_test\" (-> () _ { (assert (== ((. syntax show) quoted) \"(a b)\")) Nil }))\n",
        );
        let built = build(root, true, opt).unwrap();
        let templates = built
            .templates
            .iter()
            .filter(|s| matches!(s.show().as_str(), "(f x)" | "(a b)"))
            .collect::<Vec<_>>();
        assert_eq!(templates.len(), 2, "both fixture quotes must survive");
        assert!(templates.iter().all(|s| s.scope().marks.is_empty()));
        let modules: std::collections::HashSet<_> =
            templates.iter().map(|s| s.scope().module).collect();
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

#[test]
fn selected_test_collection_installs_its_quote_templates() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "i.toml", "name = \"app\"\n");
    write(
        root,
        "src/macros.is",
        "(, (= syntax (/ i syntax)))\n(, emit)\n(@ macro (= emit (-> (c x body (! it (. syntax Items))) _ { ((. it add) c x [] x) `(= ~x (-> () _ { ~body })) })))\n",
    );
    write(
        root,
        "src/main.i",
        "(m = app/macros)\n(anchor)\nanchor = Nil\n",
    );
    write(
        root,
        "src/quote.is",
        "(, quoted)\n(= quoted (-> () _ `(a b)))\n",
    );
    write(
        root,
        "test/check.is",
        "(, (= m (/ app macros)) (= syntax (/ i syntax)) (= main (/ app main)) (= q (/ app quote)))\n((. m emit) (. main anchor) check (assert (== ((. syntax show) ((. q quoted))) \"(a b)\")))\n",
    );
    let manifest = ilang_cli::collections(root, true, true).unwrap();
    assert_eq!(manifest.entries.len(), 1);
    for opt in [ilang_cli::Level::O0, ilang_cli::Level::O2] {
        let options = ilang_cli::BuildOptions {
            tests: true,
            opt,
            offline: true,
            target: ilang_cli::Target::Native,
        };
        let built = ilang_cli::build_collection(
            root,
            options,
            &manifest.entries[0].handle,
            ilang_cli::CollectionResult::Nil,
        )
        .unwrap();
        assert!(matches!(
            built.run_with(built.built.host_options()).unwrap(),
            ilang_host::Outcome::Ok(_)
        ));
    }
}
