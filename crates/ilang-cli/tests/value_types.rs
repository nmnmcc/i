//! Type-only dependencies must not introduce evaluation (T2 O2/joint §8).
use ilang_package::Diagnostic;

#[test]
fn completed_aliases_and_projections_do_not_change_wat() {
    let base = "(run)\nsource: Int = 1\nparts: (Int, String) = (1, \"a\")\nrun = (n: Int) { n + source }\n\nmain = () { _ = run(1) Nil }\n";
    let typed = "(run)\nsource: Int = 1\nparts: (Int, String) = (1, \"a\")\nrun = (n: Num) { n + source }\n\nmain = () { _ = run(1) Nil }\nNum = source\nPair(A,B) = (A,B)\nPair(First,Second) = parts\n";
    let compile = |source: &str, tests, target| {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.i"), source).unwrap();
        let (g, ds) = ilang_cli::load(dir.path(), tests, true);
        assert!(!ds.iter().any(Diagnostic::is_error));
        let c = ilang_check::check(&g);
        assert!(!c.diags.iter().any(Diagnostic::is_error), "{:?}", c.diags);
        ilang_wasm::compile_wat(
            &g,
            c.program.as_ref().unwrap(),
            c.types.as_ref().unwrap(),
            ilang_wasm::Options { tests, target },
        )
        .unwrap()
    };
    for tests in [false, true] {
        for target in [ilang_wasm::Target::Ilang, ilang_wasm::Target::Wasi] {
            assert_eq!(compile(base, tests, target), compile(typed, tests, target));
        }
    }
}
