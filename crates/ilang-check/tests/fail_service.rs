//! Fail uses ordinary service binding and provision (§9.2–§9.3).
mod common;

#[test]
fn fail_patterns_bind_and_explicit_providers_discharge_the_need() {
    for source in [
        include_str!("../../../tests/spec/9/fail_service_provide.i"),
        include_str!("../../../tests/spec/9/fail_service_binding.i"),
        include_str!("../../../tests/spec/9/fail_service_nested.i"),
    ] {
        let (graph, _, diags) = common::check_files(&[("src/main.i", source)]);
        assert!(
            !diags.iter().any(|d| d.is_error()),
            "{:?}",
            graph.sources.render_all(&diags)
        );
        let checked = ilang_check::check(&graph);
        let program = checked.program.unwrap();
        let types = checked.types.unwrap();
        let lines = types.lines(&program, common::module(&program, "app/main"));
        let main = lines
            .iter()
            .find(|line| line.starts_with("app/main.main : "))
            .unwrap();
        assert!(!main.contains("Fail("), "{lines:?}");
    }
}
