//! Resource accounting is a property of the macro language, not Wasm fuel.
use ilang_host::macro_budget::{Limit, Limits, MacroBudget, Usage};
use ilang_host::{HostImports, HostOptions, Outcome, Program};

fn compile(source: &str, optimize: bool) -> Vec<u8> {
    compile_project(
        source,
        "name = \"app\"\n",
        optimize,
        ilang_wasm::Target::Ilang,
    )
}

fn compile_project(
    source: &str,
    manifest: &str,
    optimize: bool,
    target: ilang_wasm::Target,
) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), manifest).unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), source).unwrap();
    let (graph, diags) = ilang_package::load(dir.path());
    assert!(diags.is_empty(), "{diags:?}");
    let checked = ilang_check::check(&graph);
    let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let (wasm, _) = ilang_wasm::compile_macro(
        &graph,
        checked.program.as_ref().unwrap(),
        checked.types.as_ref().unwrap(),
        ilang_wasm::Options {
            target,
            ..Default::default()
        },
    )
    .unwrap();
    if optimize {
        ilang_cli::optimize(&wasm, ilang_binaryen::Level::O2).unwrap()
    } else {
        wasm
    }
}

fn call(wasm: &[u8], n: i64) -> (Outcome, Usage) {
    let budget = MacroBudget::default();
    let mut imports = HostImports::default();
    imports.set_macro_budget(budget.clone());
    let program = Program::from_bytes(wasm).unwrap();
    let mut instance = program.instantiate(HostOptions::new(imports)).unwrap();
    let outcome = instance
        .call("app/main.measure", &[ilang_host::Value::Int(n.into())])
        .unwrap();
    (outcome, budget.usage())
}

fn run(wasm: &[u8], limits: Limits) -> (Result<Outcome, anyhow::Error>, MacroBudget) {
    let budget = MacroBudget::new(limits);
    let mut imports = HostImports::default();
    imports.set_macro_budget(budget.clone());
    let outcome = Program::from_bytes(wasm)
        .unwrap()
        .run_entry(HostOptions::new(imports));
    (outcome, budget)
}

#[test]
fn charges_are_identical_before_and_after_optimization() {
    let source = "f = (n) { n |> { 0 -> [\"end\"] _ -> [\"x\", ..f(n - 1)] } }\n@main\nmain = () { f(5) Nil }\n";
    let a = compile(source, false);
    let b = compile(source, true);
    let (out_a, count_a) = run(&a, Limits::default());
    let (out_b, count_b) = run(&b, Limits::default());
    assert!(matches!(out_a.unwrap(), Outcome::Ok(_)));
    assert!(matches!(out_b.unwrap(), Outcome::Ok(_)));
    assert_eq!(count_a.usage(), count_b.usage());
    assert!(count_a.usage().work > 0 && count_a.usage().allocation > 0);
    assert_eq!(count_a.usage().frames, 0);
}

#[test]
fn the_same_export_counts_longer_paths_and_ignores_the_program_target() {
    let source = "(measure)\nmeasure = (n: Int) -> Int { n |> { 0 -> 0 _ -> measure(n - 1) + 1 } }\n";
    let manifest = "name = \"app\"\n[wasm]\nexports = [\"app/main.measure\"]\n";
    let native = compile_project(source, manifest, false, ilang_wasm::Target::Ilang);
    let wasi = compile_project(source, manifest, false, ilang_wasm::Target::Wasi);
    assert_eq!(native, wasi);
    let optimized = compile_project(source, manifest, true, ilang_wasm::Target::Wasi);
    let (short, short_usage) = call(&native, 1);
    let (long, long_usage) = call(&native, 30);
    let (long_optimized, optimized_usage) = call(&optimized, 30);
    assert!(matches!(short, Outcome::Ok(_)));
    assert!(matches!(long, Outcome::Ok(_)));
    assert!(matches!(long_optimized, Outcome::Ok(_)));
    assert!(long_usage.work > short_usage.work);
    assert_eq!(long_usage, optimized_usage);
    assert_eq!(long_usage.frames, 0);
}

#[test]
fn work_and_allocation_stop_before_crossing_the_limit() {
    let wasm = compile("@main\nmain = () { [1, 2, 3] Nil }\n", false);
    let (_, work) = run(
        &wasm,
        Limits {
            work: 0,
            ..Limits::default()
        },
    );
    assert_eq!(work.exceeded(), Some(Limit::Work));
    assert_eq!(work.usage(), Usage::default());
    let (_, memory) = run(
        &wasm,
        Limits {
            allocation: 0,
            ..Limits::default()
        },
    );
    assert_eq!(memory.exceeded(), Some(Limit::Allocation));
    assert_eq!(memory.usage().allocation, 0);
}

#[test]
fn tail_frames_leave_before_call_and_non_tail_frames_accumulate() {
    let tail = compile(
        "f = (n) { n |> { 0 -> Nil _ -> f(n - 1) } }\n@main\nmain = () { f(40) }\n",
        false,
    );
    let non_tail = compile(
        "f = (n) { n |> { 0 -> 0 _ -> f(n - 1) + 1 } }\n@main\nmain = () { f(40) Nil }\n",
        false,
    );
    let limits = Limits {
        frames: 20,
        ..Limits::default()
    };
    let (ok, tail_budget) = run(&tail, limits);
    assert!(matches!(ok.unwrap(), Outcome::Ok(_)));
    assert_eq!(tail_budget.usage().frames, 0);
    let (_, nested) = run(&non_tail, limits);
    assert_eq!(nested.exceeded(), Some(Limit::Frames));
}

#[test]
fn data_dependent_work_and_allocation_grow_with_values() {
    let program = |content: &str| {
        format!("f = (s) {{ s <> s }}\n@main\nmain = () {{ f(\"{content}\") Nil }}\n")
    };
    let small = compile(&program("a"), false);
    let large = compile(&program(&"a".repeat(512)), false);
    let (a, small_budget) = run(&small, Limits::default());
    let (b, large_budget) = run(&large, Limits::default());
    assert!(matches!(a.unwrap(), Outcome::Ok(_)));
    assert!(matches!(b.unwrap(), Outcome::Ok(_)));
    assert!(large_budget.usage().work > small_budget.usage().work);
    assert!(large_budget.usage().allocation > small_budget.usage().allocation);
}

#[test]
fn frame_accounting_restores_after_errors_and_continuation_resume() {
    let source = "(i/fail\n i/control)\ninner = (n) { n |> { 0 -> fail.raise(\"err\") _ -> inner(n - 1) + 1 } }\n@main\nmain = () { fail.catch(() { inner(5) }) control.boundary((b) { control.suspend(b, (k) { control.resume(k, 1) }) }) Nil }\n";
    for optimized in [false, true] {
        let wasm = compile(source, optimized);
        let (outcome, budget) = run(&wasm, Limits::default());
        assert!(matches!(outcome.unwrap(), Outcome::Ok(_)));
        assert_eq!(budget.exceeded(), None);
        assert_eq!(budget.usage().frames, 0);
    }
}
