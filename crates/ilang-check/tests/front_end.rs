//! Interpretation checks for A1 §3.9-§3.12, separate from generated output.
mod common;

use ilang_check::hir::*;

fn checked(src: &str, unused: &[&str]) -> (ilang_package::PackageGraph, ilang_check::Checked) {
    let (graph, _, _) = common::check_files(&[("src/main.i", src)]);
    let checked = ilang_check::check(&graph);
    common::expect_unused(&checked.diags, unused);
    (graph, checked)
}

#[test]
fn declared_frames_hide_outer_variables_and_generalize() {
    // §3.10: every declared frame makes fresh variables, while a variable
    // used from an outer frame remains that variable.
    let (_, c) = checked(
        "outer = <A>(x: A) {
	inner = <A>(y: A) { y }
	(inner(1), inner(True), x)
}
nested = <A>(x: A) {
	inner = <B>(y: B, z: A) { (y, z) }
	inner(1, x)
}
id = <A>(x: A) {
	y: A = x
	y
}
uses = () { (id(1), id(True)) }",
        &["outer", "nested", "uses"],
    );
    let p = c.program.as_ref().unwrap();
    let m = p.module(common::module(p, "app/main"));
    let outer = &m.bindings[0].binding;
    let ExprKind::Fn(f) = &outer.value.kind else {
        panic!()
    };
    let Stmt::Bind(inner) = &f.body.stmts[0] else {
        panic!()
    };
    assert_eq!(outer.ty_vars.len(), 1);
    assert_eq!(inner.ty_vars.len(), 1);
    assert_ne!(outer.ty_vars[0], inner.ty_vars[0]);
}

#[test]
fn declared_external_variables_are_general_in_library_declarations() {
    // §3.10 values(): declaration frames must use their declared UpNames.
    // §10.3 forbids general host imports, so use the existing library
    // operation, whose declarations are not restricted to ABI types.
    let (mut graph, _, _) = common::check_files(&[(
        "src/main.i",
        "(i/core)\nf = () { (core.inspect(1), core.inspect(True)) }",
    )]);
    let source = graph.packages[graph.library]
        .modules
        .iter()
        .find(|m| m.path == "i/core")
        .unwrap()
        .source;
    let text = graph
        .sources
        .get(source)
        .text
        .replace("inspect: (a) -> String", "inspect(A): (A) -> String");
    assert!(text.contains("inspect(A)"));
    graph.sources.set_text(source, text);
    let c = ilang_check::check(&graph);
    common::expect_unused(&c.diags, &["f"]);
    let p = c.program.as_ref().unwrap();
    let DefKind::External { ty_vars, .. } = &common::def(p, "i/core", "inspect").kind else {
        panic!()
    };
    assert_eq!(ty_vars.len(), 1);
}

#[test]
fn new_match_is_a_non_generalized_binding_at_the_pattern() {
    // §3.9(a): resolve subject outside p's scope and keep p.span as the
    // binding span. The subject function must still call the top-level f.
    let (_, c) = checked(
        "f = (x) { x + 1 }\ng = () { (x) { f(x) * 2 } |> (f) { f(1) } }",
        &["g"],
    );
    let p = c.program.as_ref().unwrap();
    let m = p.module(common::module(p, "app/main"));
    let ExprKind::Fn(g) = &m.bindings[1].binding.value.kind else {
        panic!()
    };
    let Stmt::Expr(Expr {
        kind: ExprKind::Block(b),
        ..
    }) = &g.body.stmts[0]
    else {
        panic!()
    };
    let Stmt::Bind(binding) = &b.stmts[0] else {
        panic!()
    };
    assert!(!binding.direct);
    assert!(binding.ty_vars.is_empty());
    assert_eq!(binding.span, binding.pattern.span);
    assert!(binding.value.span.end <= binding.span.start);
    let ExprKind::Fn(subject) = &binding.value.kind else {
        panic!()
    };
    let Stmt::Expr(Expr {
        kind: ExprKind::Binary { lhs, .. },
        ..
    }) = &subject.body.stmts[0]
    else {
        panic!()
    };
    let ExprKind::Call { callee, .. } = &lhs.kind else {
        panic!()
    };
    assert!(matches!(callee.kind, ExprKind::Name(ValueRef::Def(_))));
}

#[test]
fn clause_function_labels_and_synthetic_spans_follow_the_rule() {
    // §3.9(c): no label on the artificial parameter, zero-width at `{`,
    // and the Case covers clauses only. One clause keeps label_of(p).
    // Spec §4.8 rule 11 permits values outside Int through the catch-all.
    let (_, c) = checked(
        "one = (n) { n }\nmany = (0) { 1 } | (_) { 2 }\nstring_argument = () { many(\"s\") }",
        &["one", "string_argument"],
    );
    let p = c.program.as_ref().unwrap();
    let m = p.module(common::module(p, "app/main"));
    for (index, label) in [(0, Some("n")), (1, None)] {
        let binding = &m.bindings[index].binding;
        assert!(binding.direct);
        let ExprKind::Fn(f) = &binding.value.kind else {
            panic!()
        };
        let Param::Pattern {
            pattern,
            label: actual,
        } = &f.params[0]
        else {
            panic!()
        };
        assert_eq!(actual.as_deref(), label);
        if index == 1 {
            assert_eq!(pattern.span.start, f.span.start);
            assert_eq!(pattern.span.start, pattern.span.end);
            let Stmt::Expr(case) = &f.body.stmts[0] else {
                panic!()
            };
            assert!(case.span.start > f.span.start && case.span.end < f.span.end);
        }
    }
    let lines = c
        .types
        .as_ref()
        .unwrap()
        .lines(p, common::module(p, "app/main"));
    assert!(
        lines.iter().any(|s| s == "app/main.many : (A) -> 1 | 2"),
        "{lines:?}"
    );
    assert!(!lines.iter().any(|s| s.contains("%arg")));
}

#[test]
fn kept_form_messages_are_independent_checks() {
    // D7 and §3.11 supply these codes and message fragments; no generated
    // expected output defines them.
    for (src, code, message) in [
        ("f = <A>(x: A(Int)) { x }", "E0230", "takes no arguments"),
        ("f = 1", "E0226", "declared more than once"),
        ("f = 1", "E0231", "name of a type"),
        ("f = <A>(x: a) { x }", "E0201", "not defined"),
        (
            "f = (x: Missing) { x }",
            "E0202",
            "declare it after the name",
        ),
        ("T = { (Missing) }", "E0202", "declare it in the head"),
        ("Box(Int) = { (Int) }", "E0231", "name of a type"),
        ("Box(a, B) = ()", "E0232", "an `UpName`"),
        ("Schema(Int) = Int", "E0231", "name of a type"),
        ("f = (x: p.name) { x }", "E0201", "not defined"),
        ("f = (x: m(Int)) { x }", "E0234", "kept for macros"),
        (
            /* old-syntax-check: E0301 */ "@main()\nf = () { Nil }",
            "E0301",
            "kept for macros",
        ),
        ("@a.main\nf = () { Nil }", "E0301", "kept for macros"),
        ("f(A): (A) -> String", "E0308", "kept for macros"),
    ] {
        let (g, _, diags) = common::check_files(&[("src/main.i", src)]);
        assert!(
            diags
                .iter()
                .any(|d| d.code == code && d.message.contains(message)),
            "{src}\n{:?}",
            g.sources.render_all(&diags)
        );
    }
}
