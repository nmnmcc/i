//! Expanded syntax must resolve through the ordinary checker without capture.
mod common;
use ilang_syntax::ast::{ExprKind, ItemKind, Stmt};
use ilang_syntax::lisp::{
    parse,
    scoped::ScopedSource,
    syntax::{Node, Scope, Syntax},
};
use std::collections::BTreeSet;

fn syntax(code: &str, module: u32, marks: &[u64]) -> Syntax {
    Syntax::from_datum(
        &parse(code).unwrap()[0],
        &Scope {
            module,
            marks: marks.iter().copied().collect::<BTreeSet<_>>(),
        },
    )
}
fn setup() -> (
    ilang_package::PackageGraph,
    Vec<ilang_check::Parsed>,
    u32,
    u32,
) {
    let (graph, _, _) = common::check_files(&[
        ("src/main.i", "f = (x : String) { 0 }\n"),
        ("src/template.i", "helper = 11\n"),
    ]);
    let (parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty(), "{errors:?}");
    let caller = parsed
        .iter()
        .find(|p| p.path == "app/main")
        .unwrap()
        .source
        .0;
    let template = parsed
        .iter()
        .find(|p| p.path == "app/template")
        .unwrap()
        .source
        .0;
    (graph, parsed, caller, template)
}
fn insert_expression(
    graph: &mut ilang_package::PackageGraph,
    parsed: &mut [ilang_check::Parsed],
    caller: u32,
    expression: &Syntax,
) {
    let module = parsed.iter_mut().find(|p| p.source.0 == caller).unwrap();
    let text = graph.sources.get(module.source).text.clone();
    let mut scoped = ScopedSource::new(text);
    let call = syntax("x", caller, &[]);
    let expr = scoped.expression(expression, &call).unwrap();
    let ItemKind::Binding(binding) = &mut module.ast.items[0].kind else {
        panic!("binding")
    };
    let ExprKind::Fn(function) = &mut binding.value.kind else {
        panic!("function")
    };
    function.body.stmts[0] = Stmt::Expr(expr);
    graph
        .sources
        .set_text(module.source, scoped.text().to_owned());
    module.scoped = Some(scoped);
}
#[test]
fn template_binder_does_not_capture_caller_name() {
    let (mut graph, mut parsed, caller, template) = setup();
    let result = syntax("{ (= x 3) (, x x) }", template, &[7]);
    let Node::Curly(statements) = result.node() else {
        panic!("block")
    };
    let Node::Round(tuple) = statements[1].node() else {
        panic!("tuple")
    };
    let tuple = statements[1].replace_children(vec![
        tuple[0].clone(),
        tuple[1].clone(),
        syntax("x", caller, &[]),
    ]);
    let result = result.replace_children(vec![statements[0].clone(), tuple]);
    insert_expression(&mut graph, &mut parsed, caller, &result);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let lines = checked
        .types
        .unwrap()
        .lines(&program, common::module(&program, "app/main"));
    assert!(
        lines.iter().any(|line| line.contains("(3, String)")),
        "{lines:?}"
    );
}
#[test]
fn template_private_is_visible_but_made_name_must_be_exported() {
    let (mut graph, mut parsed, caller, template) = setup();
    let private = syntax("helper", template, &[9]);
    insert_expression(&mut graph, &mut parsed, caller, &private);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let (mut graph, mut parsed, caller, template) = setup();
    let private = syntax("helper", template, &[9]);
    let made = Syntax::make(Node::Name("helper".into()), &private).unwrap();
    insert_expression(&mut graph, &mut parsed, caller, &made);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::NOT_EXPORTED),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

fn append_items(
    graph: &mut ilang_package::PackageGraph,
    parsed: &mut [ilang_check::Parsed],
    caller: u32,
    sets: &[Syntax],
) {
    let module = parsed.iter_mut().find(|p| p.source.0 == caller).unwrap();
    let mut scoped = ScopedSource::new(graph.sources.get(module.source).text.clone());
    let call = syntax("x", caller, &[]);
    for set in sets {
        module.ast.items.extend(scoped.items(set, &call).unwrap());
    }
    graph
        .sources
        .set_text(module.source, scoped.text().to_owned());
    module.scoped = Some(scoped);
}
#[test]
fn generated_top_levels_with_distinct_marks_coexist_and_link_by_identity() {
    let (mut graph, mut parsed, caller, template) = setup();
    let first = syntax("{ (= temp 1) (= out temp) }", template, &[11]);
    let second = syntax("{ (= temp 2) (= out temp) }", template, &[12]);
    append_items(&mut graph, &mut parsed, caller, &[first, second]);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let lines = checked
        .types
        .unwrap()
        .lines(&program, common::module(&program, "app/main"));
    assert!(
        lines.iter().any(|line| line == "app/main.out : 1"),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|line| line == "app/main.out : 2"),
        "{lines:?}"
    );
}
#[test]
fn made_answer_name_does_not_reach_marked_definition() {
    let (mut graph, mut parsed, caller, template) = setup();
    let first = syntax("(= temp 1)", template, &[11]);
    let binder_and_use = syntax("(= out temp)", template, &[11]);
    let Node::Round(parts) = binder_and_use.node() else {
        panic!("binding")
    };
    let from_answer = parts[2].as_answer();
    let made = Syntax::make(Node::Name("temp".into()), &from_answer).unwrap();
    let second = binder_and_use.replace_children(vec![parts[0].clone(), parts[1].clone(), made]);
    append_items(&mut graph, &mut parsed, caller, &[first, second]);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::UNKNOWN_NAME),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn builtin_identity_bypasses_caller_type_shadow() {
    use ilang_syntax::lisp::syntax::ScopeIdentity;
    let (mut graph, _, _) = common::check_files(&[("src/main.i", "Int = ()\nf = () { 0 }\n")]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty());
    let caller = parsed
        .iter()
        .find(|p| p.path == "app/main")
        .unwrap()
        .source
        .0;
    let root = syntax("((-> ((: z Int)) _ z) 1)", caller, &[]);
    let Node::Round(call) = root.node() else {
        panic!("call")
    };
    let Node::Round(fun) = call[0].node() else {
        panic!("function")
    };
    let Node::Round(params) = fun[1].node() else {
        panic!("parameters")
    };
    let Node::Round(typed) = params[0].node() else {
        panic!("typed parameter")
    };
    let builtin = typed[2].with_scope_identity(ScopeIdentity::Builtin { name: "Int".into() });
    let typed = params[0].replace_children(vec![typed[0].clone(), typed[1].clone(), builtin]);
    let params = fun[1].replace_children(vec![typed]);
    let fun =
        call[0].replace_children(vec![fun[0].clone(), params, fun[2].clone(), fun[3].clone()]);
    let root = root.replace_children(vec![fun, call[1].clone()]);
    let module = parsed.iter_mut().find(|p| p.source.0 == caller).unwrap();
    let mut scoped = ScopedSource::new(graph.sources.get(module.source).text.clone());
    let expr = scoped.expression(&root, &syntax("f", caller, &[])).unwrap();
    let ItemKind::Binding(binding) = &mut module.ast.items[1].kind else {
        panic!("binding")
    };
    let ExprKind::Fn(function) = &mut binding.value.kind else {
        panic!("function")
    };
    function.body.stmts[0] = Stmt::Expr(expr);
    graph
        .sources
        .set_text(module.source, scoped.text().to_owned());
    module.scoped = Some(scoped);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn answer_names_cannot_be_used_as_binders() {
    let (mut graph, mut parsed, caller, template) = setup();
    let answer = syntax("(= temp 1)", template, &[11]);
    let Node::Round(parts) = answer.node() else {
        panic!("binding")
    };
    let binder = parts[1].as_answer();
    let answer = answer.replace_children(vec![parts[0].clone(), binder, parts[2].clone()]);
    append_items(&mut graph, &mut parsed, caller, &[answer]);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::MACRO_BINDER),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn definition_parameter_keeps_identity_across_mark_sets() {
    use ilang_syntax::lisp::definitions::source_definition;
    let (mut graph, mut parsed, caller, template) = setup();
    let original = "Id(A) = A";
    let (module, errors) = ilang_syntax::parse_module(original);
    assert!(errors.is_empty());
    let answer = source_definition(
        &module.items[0],
        original,
        &Scope {
            module: template,
            marks: BTreeSet::new(),
        },
        5,
    )
    .flip_mark(11);
    let Node::Round(parts) = answer.node() else {
        panic!("definition")
    };
    let Node::Round(head) = parts[1].node() else {
        panic!("head")
    };
    let head = parts[1].replace_children(vec![syntax("Box", template, &[11]), head[1].clone()]);
    let definition = answer.replace_children(vec![parts[0].clone(), head, parts[2].flip_mark(99)]);
    append_items(&mut graph, &mut parsed, caller, &[definition]);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn definition_parameter_moved_to_another_head_is_rejected() {
    use ilang_syntax::lisp::definitions::source_definition;
    let (mut graph, mut parsed, caller, template) = setup();
    let original = "Id(A) = A";
    let (module, errors) = ilang_syntax::parse_module(original);
    assert!(errors.is_empty());
    let answer = source_definition(
        &module.items[0],
        original,
        &Scope {
            module: template,
            marks: BTreeSet::new(),
        },
        5,
    )
    .flip_mark(11);
    let Node::Round(parts) = answer.node() else {
        panic!("definition")
    };
    let Node::Round(answer_head) = parts[1].node() else {
        panic!("head")
    };
    let unrelated = syntax("(= (Box A) A)", template, &[11]);
    let Node::Round(other) = unrelated.node() else {
        panic!("definition")
    };
    let Node::Round(other_head) = other[1].node() else {
        panic!("head")
    };
    let head = other[1].replace_children(vec![other_head[0].clone(), answer_head[1].clone()]);
    let stolen = unrelated.replace_children(vec![other[0].clone(), head, parts[2].clone()]);
    append_items(&mut graph, &mut parsed, caller, &[stolen]);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::MACRO_BINDER),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn an_exported_inferred_type_cannot_leak_a_foreign_private_type() {
    let (mut graph, _, _) = common::check_files(&[
        ("src/main.i", "(f)\nf = () { 0 }\n"),
        ("src/template.i", "Secret = ()\n"),
    ]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty());
    let caller = parsed
        .iter()
        .find(|p| p.path == "app/main")
        .unwrap()
        .source
        .0;
    let template = parsed
        .iter()
        .find(|p| p.path == "app/template")
        .unwrap()
        .source
        .0;
    let module = parsed.iter_mut().find(|p| p.source.0 == caller).unwrap();
    let mut scoped = ScopedSource::new(graph.sources.get(module.source).text.clone());
    let expr = scoped
        .expression(&syntax("Secret", template, &[3]), &syntax("f", caller, &[]))
        .unwrap();
    let ItemKind::Binding(binding) = &mut module.ast.items[0].kind else {
        panic!("binding")
    };
    let ExprKind::Fn(function) = &mut binding.value.kind else {
        panic!("function")
    };
    function.body.stmts[0] = Stmt::Expr(expr);
    graph
        .sources
        .set_text(module.source, scoped.text().to_owned());
    module.scoped = Some(scoped);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::EXPORT_HIDDEN_TYPE),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn compiler_generated_module_can_call_private_members_but_source_cannot() {
    let (graph, _, _) = common::check_files(&[
        ("src/main.i", "(app/template)\nf = () { template.helper }\n"),
        ("src/template.i", "helper = 11\n"),
    ]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty());
    let plain = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        plain
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::NOT_EXPORTED)
    );
    parsed
        .iter_mut()
        .find(|m| m.path == "app/main")
        .unwrap()
        .compiler_generated = true;
    let generated = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !generated.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&generated.diags)
    );
}

#[test]
fn unspellable_compiler_alias_can_call_private_member() {
    let (graph, _, _) = common::check_files(&[
        ("src/main.i", "(app/template)\nf = () { template.helper }\n"),
        ("src/template.i", "helper = 11\n"),
    ]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty());
    let module = parsed.iter_mut().find(|m| m.path == "app/main").unwrap();
    let import = &mut module.ast.imports[0];
    import.alias = Some(ilang_syntax::ast::Ident {
        name: "\0compiler".into(),
        span: import.span,
    });
    let ItemKind::Binding(binding) = &mut module.ast.items[0].kind else {
        panic!("binding")
    };
    let ExprKind::Fn(function) = &mut binding.value.kind else {
        panic!("function")
    };
    let Stmt::Expr(expr) = &mut function.body.stmts[0] else {
        panic!("expression")
    };
    let ExprKind::Field { target, .. } = &mut expr.kind else {
        panic!("field")
    };
    let ExprKind::Name { name, .. } = &mut target.kind else {
        panic!("module")
    };
    name.name = "\0compiler".into();
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn same_text_in_one_tuple_pattern_binds_distinct_scopes() {
    let (mut graph, mut parsed, caller, template) = setup();
    let root = syntax("((-> ((, x x)) _ (, x x)) (, 1 \"s\"))", template, &[7]);
    let Node::Round(call) = root.node() else {
        panic!("call")
    };
    let Node::Round(fun) = call[0].node() else {
        panic!("function")
    };
    let Node::Round(params) = fun[1].node() else {
        panic!("parameters")
    };
    let Node::Round(pattern) = params[0].node() else {
        panic!("pattern")
    };
    let pattern = params[0].replace_children(vec![
        pattern[0].clone(),
        pattern[1].clone(),
        syntax("x", caller, &[]),
    ]);
    let params = fun[1].replace_children(vec![pattern]);
    let Node::Round(body) = fun[3].node() else {
        panic!("tuple")
    };
    let body = fun[3].replace_children(vec![
        body[0].clone(),
        body[1].clone(),
        syntax("x", caller, &[]),
    ]);
    let fun = call[0].replace_children(vec![fun[0].clone(), params, fun[2].clone(), body]);
    let root = root.replace_children(vec![fun, call[1].clone()]);
    insert_expression(&mut graph, &mut parsed, caller, &root);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let lines = checked
        .types
        .unwrap()
        .lines(&program, common::module(&program, "app/main"));
    assert!(lines.iter().any(|s| s.contains("(1, \"s\")")), "{lines:?}");
}

#[test]
fn parameter_labels_still_compare_written_text() {
    let (mut graph, mut parsed, caller, template) = setup();
    let root = syntax("(-> (x x) _ x)", template, &[7]);
    let Node::Round(fun) = root.node() else {
        panic!("function")
    };
    let Node::Round(params) = fun[1].node() else {
        panic!("parameters")
    };
    let params = fun[1].replace_children(vec![params[0].clone(), syntax("x", caller, &[])]);
    let root = root.replace_children(vec![fun[0].clone(), params, fun[2].clone(), fun[3].clone()]);
    insert_expression(&mut graph, &mut parsed, caller, &root);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::PARAMS_SAME_LABEL),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    assert!(
        !checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::PARAMS_BIND_TWICE),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn nested_expansions_keep_both_mark_sets_distinct() {
    let (mut graph, mut parsed, caller, template) = setup();
    let outer = syntax(
        "{ (= temp 1) { (= temp \"inner\") (, temp temp) } }",
        template,
        &[7],
    );
    let Node::Curly(outer_parts) = outer.node() else {
        panic!("outer")
    };
    let inner = outer_parts[1].flip_mark(8);
    let Node::Curly(inner_parts) = inner.node() else {
        panic!("inner")
    };
    let Node::Round(tuple) = inner_parts[1].node() else {
        panic!("tuple")
    };
    let result = inner_parts[1].replace_children(vec![
        tuple[0].clone(),
        tuple[1].clone(),
        syntax("temp", template, &[7]),
    ]);
    let inner = inner.replace_children(vec![inner_parts[0].clone(), result]);
    let outer = outer.replace_children(vec![outer_parts[0].clone(), inner]);
    insert_expression(&mut graph, &mut parsed, caller, &outer);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let lines = checked
        .types
        .unwrap()
        .lines(&program, common::module(&program, "app/main"));
    assert!(
        lines.iter().any(|s| s.contains("(\"inner\", 1)")),
        "{lines:?}"
    );
}

#[test]
fn marked_top_level_cannot_compete_with_its_scope_module_name() {
    let (mut graph, mut parsed, caller, template) = setup();
    let definition = syntax("(= helper 1)", template, &[7]);
    append_items(&mut graph, &mut parsed, caller, &[definition]);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::DEFINED_TWICE),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn answer_head_owner_survives_marks_and_freshens_but_make_cannot_copy_it() {
    use ilang_syntax::lisp::definitions::{freshen_parameters, source_definition};
    let original = "Id(A) = A";
    let (module, errors) = ilang_syntax::parse_module(original);
    assert!(errors.is_empty());
    let scope = Scope {
        module: 17,
        marks: BTreeSet::new(),
    };
    let answer = source_definition(&module.items[0], original, &scope, 5);
    let Node::Round(parts) = answer.node() else {
        panic!("definition")
    };
    assert_eq!(parts[1].definition_head(), Some(5));
    assert_eq!(parts[1].flip_mark(7).definition_head(), Some(5));
    let Node::Round(head) = parts[1].node() else {
        panic!("head")
    };
    let made = Syntax::make(Node::Round(head.clone()), &parts[1]).unwrap();
    assert_eq!(made.definition_head(), None);
    let fresh = freshen_parameters(&answer, 5, 9);
    let Node::Round(parts) = fresh.node() else {
        panic!("definition")
    };
    assert_eq!(parts[1].definition_head(), Some(9));
}

#[test]
fn an_exported_written_type_cannot_leak_a_foreign_private_type() {
    let (mut graph, _, _) = common::check_files(&[
        ("src/main.i", "(f)\nf : Int = 1\n"),
        ("src/template.i", "Secret = ()\n"),
    ]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty());
    let caller = parsed
        .iter()
        .find(|p| p.path == "app/main")
        .unwrap()
        .source
        .0;
    let template = parsed
        .iter()
        .find(|p| p.path == "app/template")
        .unwrap()
        .source
        .0;
    let module = parsed.iter_mut().find(|p| p.source.0 == caller).unwrap();
    let mut scoped = ScopedSource::new(graph.sources.get(module.source).text.clone());
    let ty = scoped
        .ty(&syntax("Secret", template, &[3]), &syntax("f", caller, &[]))
        .unwrap();
    let ItemKind::Binding(binding) = &mut module.ast.items[0].kind else {
        panic!("binding")
    };
    let ilang_syntax::ast::PatternKind::Typed { ty: written, .. } = &mut binding.pattern.kind
    else {
        panic!("typed binding")
    };
    *written = ty;
    graph
        .sources
        .set_text(module.source, scoped.text().to_owned());
    module.scoped = Some(scoped);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        checked
            .diags
            .iter()
            .any(|d| d.code == ilang_check::code::EXPORT_HIDDEN_TYPE),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn fixed_type_identity_never_falls_back_to_a_same_named_alias() {
    use ilang_syntax::lisp::syntax::ScopeIdentity;
    let (mut graph, _, _) = common::check_files(&[("src/main.i", "A = Int\nf : A = 1\n")]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty());
    let caller = parsed
        .iter()
        .find(|p| p.path == "app/main")
        .unwrap()
        .source
        .0;
    let fixed = syntax("A", caller, &[]).with_scope_identity(ScopeIdentity::TypeVariable {
        variable: 42,
        name: "A".into(),
    });
    let module = parsed.iter_mut().find(|p| p.source.0 == caller).unwrap();
    let mut scoped = ScopedSource::new(graph.sources.get(module.source).text.clone());
    let ty = scoped.ty(&fixed, &syntax("f", caller, &[])).unwrap();
    let ItemKind::Binding(binding) = &mut module.ast.items[1].kind else {
        panic!("binding")
    };
    let ilang_syntax::ast::PatternKind::Typed { ty: written, .. } = &mut binding.pattern.kind
    else {
        panic!("typed binding")
    };
    *written = ty;
    graph
        .sources
        .set_text(module.source, scoped.text().to_owned());
    module.scoped = Some(scoped);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn builtin_constructor_identity_bypasses_caller_constructor_shadow() {
    use ilang_syntax::lisp::syntax::ScopeIdentity;
    let (mut graph, _, _) =
        common::check_files(&[("src/main.i", "Ok = ()\nOption = Ok\nf = () { 0 }\n")]);
    let (mut parsed, errors) = ilang_check::parse(&graph);
    assert!(errors.is_empty());
    let caller = parsed
        .iter()
        .find(|p| p.path == "app/main")
        .unwrap()
        .source
        .0;
    let root = syntax("(Ok 1)", caller, &[]);
    let Node::Round(parts) = root.node() else {
        panic!("call")
    };
    let builtin = parts[0].with_scope_identity(ScopeIdentity::Builtin { name: "Ok".into() });
    let root = root.replace_children(vec![builtin, parts[1].clone()]);
    let module = parsed.iter_mut().find(|p| p.source.0 == caller).unwrap();
    let mut scoped = ScopedSource::new(graph.sources.get(module.source).text.clone());
    let expr = scoped.expression(&root, &syntax("f", caller, &[])).unwrap();
    // `Ok`, `Option` and `f`: `Ok = ()` is a type of its own (stage 3, §2.8).
    let ItemKind::Binding(binding) = &mut module.ast.items[2].kind else {
        panic!("binding")
    };
    let ExprKind::Fn(function) = &mut binding.value.kind else {
        panic!("function")
    };
    function.body.stmts[0] = Stmt::Expr(expr);
    graph
        .sources
        .set_text(module.source, scoped.text().to_owned());
    module.scoped = Some(scoped);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
}

#[test]
fn same_text_type_parameters_in_distinct_scopes_are_distinct() {
    let (mut graph, mut parsed, caller, template) = setup();
    let root = syntax("(= (Box A A) (, A A))", template, &[7]);
    let Node::Round(parts) = root.node() else {
        panic!("definition")
    };
    let Node::Round(head) = parts[1].node() else {
        panic!("head")
    };
    let head = parts[1].replace_children(vec![
        head[0].clone(),
        head[1].clone(),
        syntax("A", caller, &[]),
    ]);
    let Node::Round(body) = parts[2].node() else {
        panic!("body")
    };
    let body = parts[2].replace_children(vec![
        body[0].clone(),
        body[1].clone(),
        syntax("A", caller, &[]),
    ]);
    let root = root.replace_children(vec![parts[0].clone(), head, body]);
    append_items(&mut graph, &mut parsed, caller, &[root]);
    let checked = ilang_check::check_parsed(&graph, &parsed);
    assert!(
        !checked.diags.iter().any(|d| d.is_error()),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.unwrap();
    let ty = program
        .types
        .iter()
        .find(|t| t.name == "Box" && t.module == common::module(&program, "app/main"))
        .unwrap();
    assert_eq!(ty.params.len(), 2);
    assert_ne!(ty.params[0].1, ty.params[1].1);
}
