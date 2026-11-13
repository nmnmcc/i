//! E6 runtime regressions independent of the concurrent task-library port.
use ilang_host::{HostImports, HostOptions, Outcome, Program};
use ilang_wasm::{Numeric, Options};

fn run(source: &str) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"rest\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    let (mut graph, diagnostics) = ilang_package::load(dir.path());
    assert!(!diagnostics.iter().any(|d| d.is_error()), "{diagnostics:?}");
    graph.packages[graph.library].modules.retain(|m| {
        matches!(
            m.path.as_str(),
            "i/prelude" | "i/control" | "i/fail" | "i/resource" | "i/core"
        )
    });
    let checked = ilang_check::check(&graph);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    for (opt, numeric) in [(0, Numeric::Reference), (2, Numeric::Scalars)] {
        let (wasm, _) = ilang_wasm::compile_with_bindings_numeric(
            &graph,
            checked.program.as_ref().unwrap(),
            checked.types.as_ref().unwrap(),
            Options::default().numeric(numeric),
        )
        .unwrap();
        let wasm = ilang_cli::optimize(&wasm, ilang_cli::level(opt).unwrap()).unwrap();
        let program = Program::from_bytes(&wasm).unwrap();
        let outcome = program
            .run_entry(HostOptions::new(HostImports::default()))
            .unwrap();
        assert!(matches!(outcome, Outcome::Ok(_)), "O{opt}: {outcome:?}");
    }
}

#[test]
fn captured_rows_and_explicit_overrides() {
    run(include_str!("../../../tests/spec/9/rest_services.i"));
}

#[test]
fn retained_rows_survive_repeated_resume_and_cancel() {
    run(include_str!("../../../tests/spec/9/keep_env_rest.i"));
}

#[test]
fn fail_comes_from_the_caller_on_call_resume_and_cancel() {
    run(include_str!("../../../tests/spec/9/keep_fail_rest.i"));
}
