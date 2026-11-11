//! Finite schemes must survive annotations and retain entry requirements.
mod common;

fn entry(source: &str) {
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::query::check_for_queries(&graph);
    let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(
        !errors.is_empty() && errors.iter().all(|d| d.code == "E0522"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let types = checked.types.unwrap();
    let lines = types.lines(&program, common::module(&program, "app/main"));
    assert!(
        lines.iter().any(|s| s.starts_with("app/main.main : ")),
        "{lines:?}"
    );
}

#[test]
fn annotated_local_keeps_the_missing_service() {
    // §4.8 rules 4–5 retain S(N); §10.2 requires it before entry.
    let source = "S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }

main(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }";
    entry(source);
    entry(&source.replace("local: () -> Nil ! N | S(N)", "local"));
}

#[test]
fn local_scheme_is_shown_without_an_entry() {
    // §4.7: the finite principal scheme may not disappear from --types.
    let source = "S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
main(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }";
    let (graph, _, _) = common::check_files(&[("src/other.i", source)]);
    let checked = ilang_check::query::check_for_queries(&graph);
    common::expect_unused(&checked.diags, &["main"]);
    let program = checked.program.unwrap();
    let types = checked.types.unwrap();
    assert!(
        types
            .lines(&program, common::module(&program, "app/other"))
            .contains(&"app/other.main : () -> Nil ! N | S(N)".to_string())
    );
    let query = ilang_check::query::Query::new(&graph, &program, Some(&types));
    let module = common::module(&program, "app/other");
    let local = query
        .type_at(module, source.find("local:").unwrap() as u32)
        .unwrap();
    assert_eq!(local.code, "local: () -> Nil ! N | S(N)");
}

#[test]
fn annotation_positions_retain_the_service() {
    let base = "S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
run(N) = (body: () -> Nil ! N) { body() }";
    // §4.7 and §4.8: annotations, local generalization and calls cannot
    // remove a requirement. Every case is a finite scheme needing S(N).
    for body in [
        "helper(N): () -> Nil ! N | S(N) = () { use() }

main = () { helper() }",
        "helper(N) = () -> Nil ! N | S(N) { use() }

main = () { helper() }",
        "
main(N) = () { run(() -> Nil ! N | S(N) { use() }) }",
        "
main(N) = () { local: () -> Nil ! N | S(N) = () { use() } run(local) }",
        "
main(N) = () { run(() { local: () -> Nil ! N | S(N) = () { use() } local() }) }",
    ] {
        entry(&format!("{base}\n{body}"));
        let inferred = body
            .replace(": () -> Nil ! N | S(N)", "")
            .replace("() -> Nil ! N | S(N) {", "() {");
        entry(&format!("{base}\n{inferred}"));
    }
}

#[test]
fn two_parameters_retain_the_service() {
    let base = "S(E, N) { S(() -> Nil ^ E ! N) }
use(E, N) = (s ! S(E, N)) { S(f) = s f() }
run(E, N) = (body: () -> Nil ^ E ! N) { body() }";
    for body in [
        "
main(E, N) = () { local: () -> Nil ^ E ! N | S(E, N) = () { use() } local() }",
        "
main(E, N) = () { local = () { use() } local() }",
        "helper(E, N): () -> Nil ^ E ! N | S(E, N) = () { use() }

main = () { helper() }",
        "
main(E, N) = () { run(() -> Nil ^ E ! N | S(E, N) { use() }) }",
    ] {
        entry(&format!("{base}\n{body}"));
        let inferred = body
            .replace(": () -> Nil ? E ! N | S(E, N)", "")
            .replace("() -> Nil ? E ! N | S(E, N) {", "() {");
        entry(&format!("{base}\n{inferred}"));
    }
}

#[test]
fn two_environment_parameters_retain_the_service() {
    let source = "S(A, B) { S(() -> Nil ! A, () -> Nil ! B) }
use(A, B) = (s ! S(A, B)) { S(f, g) = s f() g() }

main(A, B) = () { local: () -> Nil ! A | B | S(A, B) = () { use() } local() }";
    entry(source);
    entry(&source.replace("local: () -> Nil ! A | B | S(A, B)", "local"));
}

#[test]
fn a_captured_row_annotation_can_contain_the_service() {
    // §4.7 allows N to become the more exact finite row A | S(A).
    // Merely making an unused function needs nothing (§4.8 rule 4).
    let source = "S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }

main(N) = () { local: () -> Nil ! N = () { use() } local() }";
    entry(source);
    let unused = source.replace("local() }", "Nil }");
    let (graph, _, _) = common::check_files(&[("src/main.i", &unused)]);
    for checked in [
        ilang_check::check(&graph),
        ilang_check::query::check_for_queries(&graph),
    ] {
        assert!(
            !checked.diags.iter().any(|d| d.is_error()),
            "{:?}",
            graph.sources.render_all(&checked.diags)
        );
    }
    // §9.2 compares service arguments equally even when S is covariant.
    // Supplying S(Dependency) does not supply its callback's Dependency.
    let dependency = "S(N) { S(() -> Nil ! N) }
Dependency { Dependency }
read = (d ! Dependency) { Nil }
use(N) = (s ! S(N)) { S(f) = s f() }
apply(N) = () { local: () -> Nil ! N = () { use() } local() }

main = () { ! S(Dependency) = S(() { read() }) apply() }";
    entry(dependency);
    let supplied = dependency.replace(
        "! S(Dependency)",
        "! Dependency = Dependency\n! S(Dependency)",
    );
    let (graph, _, _) = common::check_files(&[("src/main.i", &supplied)]);
    let checked = ilang_check::check(&graph);
    assert!(
        checked.diags.is_empty(),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    assert!(
        checked
            .types
            .unwrap()
            .lines(&program, common::module(&program, "app/main"))
            .contains(&"app/main.main : () -> Nil".to_string())
    );
}

#[test]
fn finite_exported_schemes_survive_module_boundaries() {
    // §2.4 and §4.7: the text and interface tree must both contain the
    // complete scheme. Reading it in another module must retain S(N).
    let source = "(S(..), helper)
S(N) { S(() -> Nil ! N) }
use(N) = (s ! S(N)) { S(f) = s f() }
helper(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }";
    let (graph, _, _) = common::check_files(&[
        ("src/service.i", source),
        (
            "src/main.i",
            "(app/service)\n\nmain = () { service.helper() }",
        ),
    ]);
    let checked = ilang_check::check(&graph);
    let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(
        !errors.is_empty() && errors.iter().all(|d| d.code == "E0522"),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let types = checked.types.unwrap();
    let module = common::module(&program, "app/service");
    let index = program
        .module(module)
        .defs
        .iter()
        .position(|d| d.name == "helper")
        .unwrap();
    let id = ilang_check::hir::DefId {
        module,
        index: index as u32,
    };
    let names = ilang_check::types::type_names(&program, module);
    assert_eq!(types.scheme_types[&id].text(&names), "() -> Nil ! N | S(N)");
    assert_eq!(types.schemes[&id], "() -> Nil ! N | S(N)");
}
