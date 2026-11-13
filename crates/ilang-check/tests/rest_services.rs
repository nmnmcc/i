//! Syntax-three E6: captured need rows stay distinct from data.
mod common;

fn checked(source: &str) -> (Vec<String>, Vec<String>) {
    let (mut graph, _, _) = common::check_files(&[("src/main.i", source)]);
    // These are checker unit tests, independent of the task/effects port.
    // Fail is an ordinary service declared by the prelude.
    graph.packages[graph.library].modules.retain(|m| {
        matches!(
            m.path.as_str(),
            "i/prelude" | "i/control" | "i/fail" | "i/resource" | "i/core"
        )
    });
    let checked = ilang_check::check(&graph);
    let errors = checked
        .diags
        .iter()
        .filter(|d| d.is_error())
        .map(|d| graph.sources.render(d))
        .collect();
    let types = match (checked.program, checked.types) {
        (Some(p), Some(t)) => t.lines(&p, common::module(&p, "app/main")),
        _ => Vec::new(),
    };
    (types, errors)
}

#[test]
fn keep_captures_the_rest_and_leaves_fail_at_the_call() {
    let (types, errors) = checked(
        "keep = <S, A, E, N>(f: (S) <- (..N, Fail(E)) -> A) <- (..env: N) -> (S) <- (Fail(E)) -> A { (x) { f(x) <- env } }",
    );
    assert!(errors.is_empty(), "{errors:#?}");
    let ty = types
        .iter()
        .find(|s| s.starts_with("app/main.keep :"))
        .unwrap();
    assert_eq!(
        ty,
        "app/main.keep : ((A) <- (Fail(C), ..N) -> B) <- N -> (A) <- (Fail(C)) -> B"
    );
}

#[test]
fn rest_requires_a_type_parameter() {
    let (_, errors) = checked("Db = ()\nf = () <- (..env: Db) { Nil }");
    assert!(
        errors
            .iter()
            .any(|d| d.contains("a rest service pattern needs a type parameter")),
        "{errors:#?}"
    );
}

#[test]
fn whole_data_provide_uses_service_keys() {
    let (_, errors) = checked(
        "Db = (value = Int)\nread = () <- (db: Db) { db.value }\nservices = () { (Db(value = 42),) }\nmain = () { assert((read() <- services()) == 42) }",
    );
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn row_values_are_not_ordinary_data() {
    for body in [
        "env == env",
        "(env,)",
        "env |> (_) { Nil }",
        "env.0",
        "inspect(env)",
    ] {
        let source = format!("inspect = (x) {{ Nil }}\nf = <N>() <- (..env: N) {{ {body} }}");
        let (_, errors) = checked(&source);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("an environment row may only be bound")),
            "{body}: {errors:#?}"
        );
    }
}

#[test]
fn rest_needs_fit_the_written_keys() {
    let (_, errors) = checked(
        "raise = () <- (handler: Fail(String)) { handler.raise(\"wrong\") }\nf = <N>() <- (handler: Fail(Int), ..env: N) { raise() }",
    );
    assert!(
        errors.iter().any(|e| e.contains("Fail(String)") && e.contains("Fail(Int)")),
        "{errors:#?}"
    );
}

#[test]
fn duplicate_explicit_and_spread_keys_are_errors() {
    for provided in ["(Db(1), Db(2))", "(..(Db(1),), ..(Db(2),))"] {
        let source = format!(
            "Db = (Int)\nread = () <- (db: Db) {{ Db(n) = db\n n }}\nmain = () {{ read() <- {provided} }}"
        );
        let (_, errors) = checked(&source);
        assert!(
            errors.iter().any(|e| e.contains("one service key")),
            "{provided}: {errors:#?}"
        );
    }
}

#[test]
fn e6_runtime_fixtures_type_check() {
    for source in [
        include_str!("../../../tests/spec/9/rest_services.i"),
        include_str!("../../../tests/spec/9/keep_env_rest.i"),
        include_str!("../../../tests/spec/9/keep_fail_rest.i"),
    ] {
        let (_, errors) = checked(source);
        assert!(errors.is_empty(), "{errors:#?}");
    }
}

#[test]
fn a_rest_type_parameter_cannot_also_be_a_value_type() {
    let (_, errors) = checked("f = <N>(x: N) <- (..env: N) { Nil }");
    assert!(errors.iter().any(|e| e.contains("cannot also be an ordinary value type")), "{errors:#?}");
}

#[test]
fn a_rest_head_declares_its_own_row_parameter() {
    let (_, errors) = checked("outer = <N>(f: () <- N -> Nil) { inner = () <- (..env: N) { f() }; Nil }".replace(";", "\n").as_str());
    assert!(errors.iter().any(|e| e.contains("a rest service pattern needs a type parameter of the function")), "{errors:#?}");
}
