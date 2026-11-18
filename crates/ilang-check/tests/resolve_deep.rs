//! A value nested far deeper than a normal stack goes through the production
//! resolver end to end: package load → parse → `resolve3`.
//!
//! This replaces the deep-resolution coverage removed with the legacy
//! differential tests. Unlike `hir_deep.rs`, which exercises HIR visitors on
//! hand-built trees, it drives the real pipeline from source text.
use ilang_check::hir::DefKind;
use ilang_syntax::syn3::deep::deep;

#[test]
fn resolve3_handles_a_20000_deep_value() {
    let depth = 20_000;
    let text = format!("x = {}1{}\n", "[".repeat(depth), "]".repeat(depth));
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"deep\"\n").unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), &text).unwrap();

    let (graph, _) = ilang_package::load(dir.path());
    let root = graph.root.expect("the package loads");
    let (parsed, diags) = deep(|| ilang_check::parse(&graph));
    let parse_errors: Vec<_> = diags.into_iter().filter(|d| d.is_error()).collect();
    let modules: Vec<_> = deep(|| parsed.iter().map(|p| p.ast.clone()).collect());

    let (program, diags) = ilang_check::resolve::resolve3(&graph, &parsed, &modules);
    let resolve_errors: Vec<_> = diags.into_iter().filter(|d| d.is_error()).collect();

    // Gather the facts first and drop the deep trees on a deep stack, so a
    // failing assertion reports instead of overflowing while unwinding.
    let mains: Vec<_> = program
        .modules
        .iter()
        .filter(|m| m.package == root)
        .map(|m| {
            let x: Vec<_> = m
                .defs
                .iter()
                .filter(|d| d.name == "x")
                .map(|d| matches!(d.kind, DefKind::Binding { binding: 0, .. }))
                .collect();
            (m.path.clone(), m.bindings.len(), m.defs.len(), x)
        })
        .collect();
    deep(move || {
        drop(program);
        drop(modules);
        drop(parsed);
    });

    assert!(parse_errors.is_empty(), "parse: {parse_errors:?}");
    assert!(resolve_errors.is_empty(), "resolve: {resolve_errors:?}");
    // The root package has exactly its one module, with one binding whose
    // only name is `x`, bound by that binding.
    assert_eq!(
        mains,
        vec![("deep/main".to_string(), 1, 1, vec![true])],
        "root modules: (path, bindings, defs, x is binding 0)"
    );
}
