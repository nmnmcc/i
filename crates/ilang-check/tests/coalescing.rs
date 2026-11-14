//! Finite schemes must survive annotations and retain entry requirements.
mod common;

fn entry(source: &str) {
    let (graph, _, _) = common::check_files(&[("src/main.i", source)]);
    let checked = ilang_check::query::check_for_queries(&graph);
    let errors: Vec<_> = checked.diags.iter().filter(|d| d.is_error()).collect();
    assert!(
        errors.is_empty(),
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
    let source = "S(N) = (() <- N -> Nil)\nuse = <N>() <- (s : S(N)) {\n\tS(f) = s\n\tf()\n}\n\nmain = <N>() {\n\tlocal : () <- (S(N), ..N) -> Nil = () { use() }\n\tlocal()\n}";
    entry(source);
    entry(&source.replace("local : () <- (S(N), ..N) -> Nil", "local"));
}

#[test]
fn local_scheme_is_shown_without_an_entry() {
    // §4.7: the finite principal scheme may not disappear from --types.
    let source = "S(N) = (() <- N -> Nil)\nuse = <N>() <- (s : S(N)) {\n\tS(f) = s\n\tf()\n}\nmain = <N>() {\n\tlocal : () <- (S(N), ..N) -> Nil = () { use() }\n\tlocal()\n}";
    let (graph, _, _) = common::check_files(&[("src/other.i", source)]);
    let checked = ilang_check::query::check_for_queries(&graph);
    common::expect_unused(&checked.diags, &["main"]);
    let program = checked.program.unwrap();
    let types = checked.types.unwrap();
    assert!(
        types
            .lines(&program, common::module(&program, "app/other"))
            .contains(&"app/other.main : () <- (S(N), ..N) -> Nil".to_string())
    );
    let query = ilang_check::query::Query::new(&graph, &program, Some(&types));
    let module = common::module(&program, "app/other");
    let local = query
        .type_at(module, source.find("local :").unwrap() as u32)
        .unwrap();
    assert_eq!(local.code, "local : () <- (S(N), ..N) -> Nil");
}

#[test]
fn annotation_positions_retain_the_service() {
    let base = "S(N) = (() <- N -> Nil)\nuse = <N>() <- (s : S(N)) {\n\tS(f) = s\n\tf()\n}\nrun = <N>(body : () <- N -> Nil) { body() }";
    // §4.7 and §4.8: annotations, local generalization and calls cannot
    // remove a requirement. Every case is a finite scheme needing S(N).
    for body in [
        "helper : <N>() <- (S(N), ..N) -> Nil = <N>() { use() }\n\nmain = () { helper() }",
        "helper = () -> Nil { use() }

main = () { helper() }",
        "main = <N>() { run(() -> Nil { use() }) }",
        "main = <N>() {\n\tlocal : () <- (S(N), ..N) -> Nil = () { use() }\n\trun(local)\n}",
        "main = <N>() {\n\trun(() {\n\t\tlocal : () <- (S(N), ..N) -> Nil = () { use() }\n\t\tlocal()\n\t})\n}",
    ] {
        entry(&format!("{base}\n{body}"));
        let inferred = body
            .replace(": <N>() <- (S(N), ..N) -> Nil", "")
            .replace(": () <- (S(N), ..N) -> Nil", "")
            .replace("() -> Nil {", "() {");
        entry(&format!("{base}\n{inferred}"));
    }
}

#[test]
fn two_parameters_retain_the_service() {
    let base = "S(E, N) = (() <- (Fail(E), ..N) -> Nil)\nuse = <E, N>() <- (s : S(E, N)) {\n\tS(f) = s\n\tf()\n}\nrun = <N>(body : () <- N -> Nil) { body() }";
    for body in [
        "main = <E, N>() {\n\tlocal : () <- (Fail(E), S(E, N), ..N) -> Nil = () { use() }\n\tlocal()\n}",
        "main = <E, N>() {
	local = () { use() }
	local()
}",
        "helper : <E, N>() <- (Fail(E), S(E, N), ..N) -> Nil = <E, N>() { use() }\n\nmain = () { helper() }",
        "main = <E, N>() { run(() -> Nil { use() }) }",
    ] {
        entry(&format!("{base}\n{body}"));
        let inferred = body
            .replace(": <E, N>() <- (Fail(E), S(E, N), ..N) -> Nil", "")
            .replace(": () <- (Fail(E), S(E, N), ..N) -> Nil", "")
            .replace("() -> Nil {", "() {");
        entry(&format!("{base}\n{inferred}"));
    }
}

#[test]
fn two_environment_parameters_retain_the_service() {
    let source = "S(A, B) = (() <- A -> Nil, () <- B -> Nil)\nuse = <A, B>() <- (s : S(A, B)) {\n\tS(f, g) = s\n\tf()\n\tg()\n}\n\nmain = <A, B>() {\n\tlocal = () { use() }\n\tlocal()\n}";
    // The old annotation `() -> Nil ! A | B | S(A, B)` has two row variables,
    // which syntax three cannot write (one `..N` per need list); only the
    // inferred form is checked.
    entry(source);
}

#[test]
fn a_captured_row_annotation_can_contain_the_service() {
    // §4.7 allows N to become the more exact finite row A | S(A).
    // Merely making an unused function needs nothing (§4.8 rule 4).
    let source = "S(N) = (() <- N -> Nil)\nuse = <N>() <- (s : S(N)) {\n\tS(f) = s\n\tf()\n}\n\nmain = <N>() {\n\tlocal : () <- (Fail(Never), ..N) -> Nil = () { use() }\n\tlocal()\n}";
    entry(source);
    let unused = source.replace("\tlocal()\n}", "\tNil\n}");
    assert_ne!(source, unused);
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
    // §9.2 fits the provided service by ordinary value subtyping.
    // Supplying S(Dependency) does not supply its callback's Dependency.
    let dependency = "S(N) = (() <- N -> Nil)\nDependency = ()\nread = () <- (d : Dependency) { Nil }\nuse = <N>() <- (s : S(N)) {\n\tS(f) = s\n\tf()\n}\napply = <N>() {\n\tlocal : () <- (Fail(Never), ..N) -> Nil = () { use() }\n\tlocal()\n}\n\nmain = () { provided : S(Dependency) = S(() { read() }) apply() <- (provided) }";
    entry(dependency);
    // An empty `Dependency = ()` is not a keyed service type in syntax three
    // (E0225), so the supplied variant gives it a field.
    let supplied = dependency
        .replace("Dependency = ()", "Dependency = (value = Int)")
        .replace(
            "apply() <- (provided)",
            "apply() <- (Dependency(value = 1), provided)",
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
    let source = "(\n\tS(..)\n\thelper\n)\nS(N) = (() <- N -> Nil)\nuse = <N>() <- (s : S(N)) {\n\tS(f) = s\n\tf()\n}\nhelper = <N>() {\n\tlocal : () <- (S(N), ..N) -> Nil = () { use() }\n\tlocal()\n}";
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
        errors.is_empty(),
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
    assert_eq!(
        types.scheme_types[&id].text(&names),
        "() <- (S(N), ..N) -> Nil"
    );
    assert_eq!(types.schemes[&id], "() <- (S(N), ..N) -> Nil");
}
