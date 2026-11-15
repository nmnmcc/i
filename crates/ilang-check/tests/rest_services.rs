//! Syntax-three E6: captured need rows stay distinct from data.
mod common;

fn checked(source: &str) -> (Vec<String>, Vec<String>) {
    let (mut graph, _, _) = common::check_files(&[("src/main.i", source)]);
    // These are checker unit tests, independent of the task/effects port.
    // Fail is an ordinary service declared by the prelude.
    graph.packages[graph.library].modules.retain(|m| {
        matches!(
            m.path.as_str(),
            "i/prelude" | "i/control" | "i/resource" | "i/core"
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
        "Db = (value = Int)\nread = () <- (db: Db) { db.value }\nservices = () { (Db(value = 42),) }\nmain = () { assert({ read() <- services() } == 42) }",
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
        errors
            .iter()
            .any(|e| e.contains("Fail(String)") && e.contains("Fail(Int)")),
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
fn a_need_parameter_can_also_appear_in_a_value_type() {
    // Decision 1 (EF01/EF09) removes §4.7's exclusive classification.
    // Retain the original mixed-use source as an acceptance regression;
    // known keys are still required for extracting/providing services.
    let (_, errors) = checked("f = <N>(x: N) <- (..env: N) { Nil }");
    assert!(errors.is_empty(), "{errors:#?}");
    let (_, errors) = checked("f = <N>(x: N) { Nil <- (x) }");
    assert!(
        errors
            .iter()
            .any(|e| e.contains("not known to be one service type")),
        "{errors:#?}"
    );
}

#[test]
fn a_rest_head_declares_its_own_row_parameter() {
    let (_, errors) = checked(
        "outer = <N>(f: () <- N -> Nil) { inner = () <- (..env: N) { f() }; Nil }"
            .replace(";", "\n")
            .as_str(),
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("a rest service pattern needs a type parameter of the function")),
        "{errors:#?}"
    );
}

#[test]
fn an_unused_fail_parameter_keeps_its_service_key() {
    // Decision 2: the declared Fail(E) need is real even at E = Never.
    // Keep both original definitions as rejection cases; retain positive
    // callback coverage by passing the need on and by supplying its value.
    let original = "constrain = <E>(f: () <- (Fail(E)) -> Int) -> () -> Int { () { f() } }\npure = () { constrain(() { 42 })() }";
    for source in [
        original,
        "raises = () <- (handler: Fail(String)) { handler.raise(\"error\") }\nconstrain = (f: () <- (Fail(String)) -> Int) -> () -> Int { () { f() } }",
    ] {
        let (_, errors) = checked(source);
        assert!(
            errors.iter().any(|e| e.contains("not a subtype")),
            "{errors:#?}"
        );
    }
    let forwarding = original.replace("-> () -> Int", "-> () <- (Fail(E)) -> Int");
    let source = format!(
        "{forwarding}\nprovided = () {{ constrain(() {{ 42 }})() <- (Fail(raise = (_: Never) {{ panic(\"unreachable\") }})) }}"
    );
    let (shown, errors) = checked(&source);
    assert!(errors.is_empty(), "{errors:#?}");
    assert!(
        shown
            .iter()
            .any(|t| t.starts_with("app/main.pure :") && t.contains("Fail(")),
        "{shown:#?}"
    );
    assert!(
        shown.iter().any(|t| t == "app/main.provided : () -> Int"),
        "{shown:#?}"
    );
}

#[test]
fn a_full_function_type_supplies_the_rest_heads_callback_type() {
    let (_, errors) = checked(
        "keep: <S, A, E, N>((S) <- (..N, Fail(E)) -> A) <- N -> (S) <- (Fail(E)) -> A = (f) <- (..env: N) { (x) { f(x) <- env } }\nName = (text = String)\nread = (_: Nil) <- (name: Name) { name.text }\nmain = () { f = keep(read) <- (Name(text = \"saved\"))\n assert(f(Nil) == \"saved\") }",
    );
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn a_named_client_beside_a_rest_row_keeps_its_known_data_type() {
    let (_, errors) = checked(
        "Client = (make = () -> (send = () -> Int, close = () -> Nil))\n\
         read = <N>() <- (client: Client, ..env: N) { functions = client.make()\n functions.close()\n functions.send() }\n\
         saved = <N>() <- (client: Client, ..env: N) { value = client\n () { value.make().send() } }\n\
         forward = <A, N>(body: () <- (..N, Client) -> A) <- (client: Client, ..env: N) { body() <- (..env, client) }\n\
         main = () { client = Client(make = () { (send = () { 42 }, close = () { Nil }) })\n\
           assert({ forward(() { read() }) <- (client) } == 42)\n\
           callback = saved() <- (client)\n assert(callback() == 42) }",
    );
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn a_whole_provided_data_value_keeps_its_written_type() {
    let source = "Db(A) = (value = A)\nread = () <- (db: Db(Int)) { db.value }\nmain = () { given: (Db(Int),) = (Db(value = 1),)\n read() <- given }";
    let (_, errors) = checked(source);
    assert!(errors.is_empty(), "{errors:#?}");
    let (_, errors) = checked(&source.replace("(Db(Int),)", "(Db(String),)"));
    assert!(
        errors
            .iter()
            .any(|e| e.contains("E0501") && e.contains("String")),
        "the written data annotation must still be checked: {errors:#?}"
    );
}
