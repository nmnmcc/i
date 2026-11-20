//! Runtime regressions against ordinary library services and task.keep.
use ilang_host::{HostImports, HostOptions, Outcome, Program};
use ilang_wasm::{Numeric, Options};

/// A manifest for `name` that depends on the standard library of this
/// repository by path.
fn manifest(name: &str) -> String {
    let library = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../stdlib")
        .canonicalize()
        .unwrap();
    format!(
        "name = {name:?}\n[dependencies]\ni = {{ path = {:?} }}\n",
        library.to_str().unwrap()
    )
}

fn run(source: &str) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), manifest("rest")).unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    let (mut graph, diagnostics) = ilang_package::load(dir.path());
    assert!(!diagnostics.iter().any(|d| d.is_error()), "{diagnostics:?}");
    let needs_task = source.contains("i/task");
    // The library modules are in the package `i`; the built-in package has
    // only the prelude.
    let library = graph
        .standard_library()
        .expect("the package depends on the standard library");
    graph.packages[library].modules.retain(|m| {
        needs_task
            || matches!(
                m.path.as_str(),
                "$prelude" | "i/control" | "i/fail" | "i/resource" | "i/core"
            )
            || ilang_package::library::RUNTIME.contains(&m.path.as_str())
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

#[test]
fn task_keep_uses_the_invoking_catch_after_yield() {
    run(include_str!("../../../tests/spec/9/task_keep_error.i"));
}

#[test]
fn external_host_errors_raise_through_the_callers_fail() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), manifest("app")).unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/main.i"),
        include_str!("../../../tests/browser/fixtures/external-fail/src/main.i"),
    )
    .unwrap();
    for opt in [ilang_cli::Level::O0, ilang_cli::Level::O2] {
        let built = ilang_cli::build(
            dir.path(),
            ilang_cli::BuildOptions {
                opt,
                offline: true,
                tests: false,
                target: Default::default(),
            },
        )
        .unwrap();
        let mut imports = HostImports::default();
        imports
            .fallible_external("app:fallible/v1", "value", |args| {
                let [ilang_host::Value::String(text)] = args else {
                    panic!("one string")
                };
                Ok(if text == "bad" {
                    Err(ilang_host::Value::String("from-host".into()))
                } else {
                    Ok(ilang_host::Value::Int(42.into()))
                })
            })
            .unwrap();
        let outcome = Program::from_bytes(&built.wasm)
            .unwrap()
            .run_entry(HostOptions::new(imports))
            .unwrap();
        assert!(matches!(outcome, Outcome::Ok(_)), "{opt:?}: {outcome:?}");
    }
}
