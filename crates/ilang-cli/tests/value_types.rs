//! Type-only dependencies must not introduce evaluation.
use ilang_package::Diagnostic;

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
fn completed_aliases_and_projections_do_not_change_wat() {
    // Keep executable source positions equal: call-site metadata is part of WAT.
    let base = "(\n\trun\n)\nsource : Int = 1\nparts : (Int, String) = (1, \"a\")\nrun = (n : Int) { n + source }\n\nmain = () {\n\t_ = run(1)\n\t()\n}\n";
    let typed = "(\n\trun\n)\nsource : Int = 1\nparts : (Int, String) = (1, \"a\")\nrun = (n : Num) { n + source }\n\nmain = () {\n\t_ = run(1)\n\t()\n}\nNum = source\nPair(A, B) = { (A, B) }\nPair(First, Second) = parts\n";
    let compile = |source: &str, tests, target| {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("i.toml"),
            format!(
                "name = \"app\"\nhostcall = true\n{}",
                library_dependencies()
            ),
        )
        .unwrap();
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
    // Type-only aliases may shift numeric constant pool entries and offsets
    // without changing the code that runs. Compare the instruction structure.
    fn without_numeric_constants(wat: &str) -> String {
        let mut out = String::with_capacity(wat.len());
        let mut digit = false;
        for c in wat.chars() {
            if c.is_ascii_digit() {
                if !digit {
                    out.push('0');
                    digit = true;
                }
            } else {
                digit = false;
                out.push(c);
            }
        }
        out
    }
    for tests in [false, true] {
        for target in [ilang_wasm::Target::Native, ilang_wasm::Target::Wasi] {
            assert_eq!(
                without_numeric_constants(&compile(base, tests, target)),
                without_numeric_constants(&compile(typed, tests, target))
            );
        }
    }
}
