//! Syntax-three E6: captured need rows stay distinct from data.
mod common;

fn checked(source: &str) -> (Vec<String>, Vec<String>) {
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::check(&graph);
    let errors = checked.diags.iter().filter(|d| d.is_error())
        .map(|d| graph.sources.render(d)).collect();
    let types = match (checked.program, checked.types) {
        (Some(p), Some(t)) => t.lines(&p, common::module(&p, "app/main")),
        _ => Vec::new(),
    };
    (types, errors)
}

#[test]
fn keep_captures_the_rest_and_leaves_fail_at_the_call() {
    let (types, errors) = checked("keep = <S, A, E, N>(f: (S) <- (..N, Fail(E)) -> A) <- (..env: N) -> (S) <- (Fail(E)) -> A { (x) { f(x) <- env } }");
    assert!(errors.is_empty(), "{errors:#?}");
    let ty = types.iter().find(|s| s.starts_with("keep :")).unwrap();
    assert_eq!(ty, "keep : ((A) <- (Fail(E), ..N) -> B) <- N -> (A) <- (Fail(E)) -> B");
}

#[test]
fn rest_requires_a_type_parameter() {
    let (_, errors) = checked("Db = ()\nf = () <- (..env: Db) { Nil }");
    assert!(errors.iter().any(|d| d.contains("a rest service pattern needs a type parameter")), "{errors:#?}");
}

#[test]
fn whole_data_provide_uses_service_keys() {
    let (_, errors) = checked("Db = (value = Int)\nread = () <- (db: Db) { db.value }\nservices = () { (Db(value = 42),) }\nmain = () { assert((read() <- services()) == 42) }");
    assert!(errors.is_empty(), "{errors:#?}");
}
