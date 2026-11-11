//! Interpretation checks name syntax-2 rules; tree and formatting checks are
//! separate implementation checks, not an oracle for the language rules.
use ilang_syntax::{
    ast::*, format::format, parse_module, parser::parse_module_legacy, print::print_module,
    shape::canon,
};

fn binding(source: &str) -> Expr {
    let (module, diagnostics) = parse_module(source);
    assert!(diagnostics.is_empty(), "{source}: {diagnostics:?}");
    let ItemKind::Binding(binding) = &module.items.last().unwrap().kind else {
        panic!()
    };
    binding.value.clone()
}

#[test]
fn match_suffix_binds_above_unary_and_every_binary_level() {
    // M1/M2: suffix is a Case node, and is stronger than unary and multiplication.
    let binary = binding("answer = n.{ 0 -> 1 _ -> 2 } * 10");
    let ExprKind::Binary { lhs, .. } = &binary.kind else {
        panic!()
    };
    assert!(matches!(lhs.kind, ExprKind::Case { .. }));
    let unary = binding("answer = -n.{ 0 -> 1 _ -> 2 }");
    let ExprKind::Unary { expr, .. } = &unary.kind else {
        panic!()
    };
    assert!(matches!(expr.kind, ExprKind::Case { .. }));
    for subject in [
        "a || b", "a && b", "a == b", "a < b", "a <> b", "a | f", "a + b", "a * b", "-a",
    ] {
        let source = format!("answer = {{ {subject} }}.{{ _ -> 1 }}");
        assert!(matches!(binding(&source).kind, ExprKind::Case { .. }));
    }
}

#[test]
fn bare_pipe_does_not_make_a_clause_head() {
    // Coordinator a / P3: a bar selects clauses only after a complete head.
    let value = binding("answer = { x | f }");
    let ExprKind::Block(block) = &value.kind else {
        panic!()
    };
    assert!(matches!(
        &block.stmts[0],
        Stmt::Expr(Expr {
            kind: ExprKind::Binary {
                op: BinOp::Pipe,
                ..
            },
            ..
        })
    ));
    assert!(matches!(
        binding("answer = { 0 | 1 -> 2 _ -> 3 }").kind,
        ExprKind::Clauses(_)
    ));
    // Coordinator b: piping to a clause value has existing direct-match meaning.
    assert!(matches!(
        binding("answer = x | { 0 -> 1 _ -> 2 }").kind,
        ExprKind::Case { .. }
    ));
}

#[test]
fn caret_and_guard_have_separate_slots() {
    // E1/E2 and P2: caret is an error slot, question is a guard.
    binding("answer = (x) -> Int ^ String ! Log { x }");
    binding("answer = x.{ f: (Int) -> Int ? ready -> f(1) _ -> 0 }");
    binding("answer = x.{ n ? n | ready -> 1 _ -> 0 }");
}

#[test]
fn contexts_and_records_do_not_use_commas_to_choose() {
    // D1–D4: function suffix wins; mixed calls/constructors stay valid.
    binding("answer = (a: Int) { a }");
    binding("answer = f(1, a: 2)");
    let (_, diagnostics) = parse_module("User { User(Int, age: Int) }");
    assert!(diagnostics.is_empty());
    for rhs in ["(a: 1)", "(..base, a: 1)", "(f: (x) -> Int { x })", "()"] {
        assert!(matches!(
            binding(&format!("answer = {rhs}")).kind,
            ExprKind::Record { .. }
        ));
    }
    for rhs in ["(a: 1, 2)", "(1, a: 2)"] {
        let (_, diagnostics) = parse_module(&format!("answer = {rhs}"));
        assert!(
            diagnostics.iter().any(|d| d.message.contains("cannot mix")),
            "{diagnostics:?}"
        );
    }
    // D3/N2: function parameter lists may be empty; tuple types/patterns may not.
    for source in ["Unit = ()", "take = (()) { Nil }"] {
        let (_, diagnostics) = parse_module(source);
        assert!(
            diagnostics.iter().any(|d| d.message.contains("Nil")),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn numeric_subject_diagnostic_keeps_number_lexing() {
    // Coordinator d / M3: the formatter prefers a brace group.
    let source = format!("answer = {}", "1.{ _ -> 0 }");
    assert!(
        parse_module(&source)
            .1
            .iter()
            .any(|d| d.message.contains("lexed as a float"))
    );
    let out = format("answer = 1 |> { _ -> 0 }").unwrap();
    assert!(out.contains("{ 1 }.{"), "{out}");
}

#[test]
fn both_printers_preserve_trees_and_comments_with_new_spelling() {
    for source in [
        "answer = { a == b } |> { True -> 1 False -> 0 }",
        "answer = 1 + n |> { 0 -> 1 _ -> 2 }",
        "answer = -n |> { 0 -> 1 _ -> 2 }",
        "answer = x // subject\n |> { n ? n > 0 -> n _ -> 0 }",
        "answer = (x) -> Int ? String { x }",
    ] {
        let (before, diagnostics) = parse_module_legacy(source);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        for out in [format(source).unwrap(), print_module(&before)] {
            let (after, diagnostics) = parse_module(&out);
            assert!(diagnostics.is_empty(), "{out}: {diagnostics:?}");
            assert_eq!(canon(&before), canon(&after), "{out}");
            assert!(!out.contains("|>"), "{out}");
        }
        let once = format(source).unwrap();
        assert_eq!(format(&once).unwrap(), once);
        if source.contains("// subject") {
            assert!(once.contains("// subject"));
        }
    }
}
