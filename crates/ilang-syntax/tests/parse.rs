//! Interpretation checks cite A1 design §3.4-§3.7 and §3.13. These values
//! come from those rules, independently of the generated goldens.
mod common;

use ilang_syntax::ast::*;
use ilang_syntax::print::{print_module, unprintable};
use ilang_syntax::{Code, parse_module, parse_stats};

fn module(src: &str) -> Module {
    let (m, d) = parse_module(src);
    assert!(d.is_empty(), "{src}\n{d:?}");
    assert_eq!(unprintable(&m), None);
    let text = print_module(&m);
    let (again, d) = parse_module(&text);
    assert!(d.is_empty(), "{text}\n{d:?}");
    assert_eq!(common::shape(&m), common::shape(&again), "{text}");
    assert_eq!(
        ilang_syntax::format::format(&text).and_then(|s| ilang_syntax::format::format(&s)),
        ilang_syntax::format::format(&text)
    );
    m
}

fn value(m: &Module) -> &Expr {
    let ItemKind::Binding(b) = &m.items[0].kind else {
        panic!("expected binding")
    };
    &b.value
}

#[test]
fn brace_decisions_follow_the_ordered_tests() {
    // Accepted syntax.md §6.1 examples: tests 1-3 and 6 make blocks.
    for expr in [
        "{ go = (n) -> Int { n + 1 }  go(1) }",
        "{ go = (n) -> Int ? String ! Log { step(n) }  go(1) }",
        "{ k: (Int) -> Int = f  k(1) }",
        "{ go(A) = (x: A) -> A { x }  go(1) }",
        "{ go = (n, log ! Log) -> Int { log.write(n)  n }  go(1) }",
        "{ k = 2  g = (x) -> Int { x * k }  g(1) }",
        "{ x }",
    ] {
        assert!(
            matches!(
                value(&module(&format!("a = {expr}"))).kind,
                ExprKind::Block(_)
            ),
            "{expr}"
        );
    }
    // Tests 0 and 2, including chain patterns, make clauses.
    for expr in [
        "{}",
        "{ x -> x }",
        "{ t = (0, _) -> t  _ -> 0 }",
        "{ r = Ok(v) -> v  _ -> 0 }",
        "{ s = \"ab\" <> rest -> s  _ -> \"\" }",
        "{ p = (a, b) -> { () { log(p) } } }",
        "{ f: { (Int) -> Int } ? ok -> f(1)  _ -> 0 }",
    ] {
        assert!(
            matches!(
                value(&module(&format!("a = {expr}"))).kind,
                ExprKind::Clauses(_)
            ),
            "{expr}"
        );
    }
    // Opening: a function body always is a block, without a brace decision.
    let m = module("make = (n) { (x) -> Int { x + n } }");
    let ExprKind::Fn(f) = &value(&m).kind else {
        panic!()
    };
    assert!(matches!(
        &f.body.stmts[0],
        Stmt::Expr(Expr {
            kind: ExprKind::Fn(_),
            ..
        })
    ));
    // T1 §6.1: a literal type starts an annotation, so this is a clause.
    assert!(matches!(
        value(&module("a = { x: 1 -> 2 }")).kind,
        ExprKind::Clauses(_)
    ));
    // §3.5 test 4 and test 5: records vs a clause with a wrong pattern.
    for (src, code) in [
        (
            /* old-syntax-check: E0114,E0101 */ "a = { p: (Int, Int) -> e }",
            Code::Unexpected,
        ),
        (
            /* old-syntax-check: E0108 */ "a = x |> { -y -> 1 }",
            Code::BadPattern,
        ),
        (
            /* old-syntax-check: E0101 */ "a = b |> { <<x: Int>> -> 1 }",
            Code::Unexpected,
        ),
    ] {
        let d = parse_module(src).1;
        assert_eq!(d.first().map(|d| d.code), Some(code), "{src}\n{d:?}");
    }
    // The tuple note is required in later clauses too (§3.6).
    for src in [
        /* old-syntax-check: E0101 */ "a = v |> { 0 -> 1  p: (Int, Int) -> e }",
        /* old-syntax-check: E0101 */ "a = { 0 -> 1  p: (Int, Int) -> e }",
    ] {
        let d = parse_module(src).1;
        assert!(!d.is_empty());
        assert!(
            d[0].message
                .contains("a tuple type in a clause pattern is put in `{ }`")
        );
    }
}

#[test]
fn header_groups_and_attributes() {
    // §3.4: content, not position, distinguishes exports and imports.
    let m = module("(a = (T, f) = pkg/mod)\n(main, T(..))\n(x, y) = (1, 2)\nmain = () { f(x) }");
    assert_eq!(m.imports[0].path[0].name, "pkg");
    assert_eq!(m.items.len(), 2);
    for src in [
        "()",
        /* old-syntax-check: E0115 */ "(main)\n(i/fail)",
        "(main)\n(main)",
        /* old-syntax-check: E0115 */ "(i/fail)\n(i/core)",
        /* old-syntax-check: E0115 */ "x = 1\n(main)",
        /* old-syntax-check: E0115 */ "@deprecated\n(\"x\")\nmain = () { Nil }",
    ] {
        let d = parse_module(src).1;
        assert_eq!(d.len(), 1, "{src}\n{d:?}");
        assert_eq!(d[0].code, Code::HeaderGroup);
    }
    let m = module("@m\n(even, odd) = (1, 2)");
    assert_eq!(m.items[0].attributes[0].name.name, "m");
    assert!(
        parse_module("@m (even, odd) = (1, 2)")
            .1
            .iter()
            .any(|d| d.code == Code::Unexpected)
    );
    let m = module("@main()\nx = 1\n@a.m(1, b: x)\ny = 2");
    assert!(m.items[0].attributes[0].call.is_some());
    assert_eq!(m.items[1].attributes[0].module.as_ref().unwrap().name, "a");
}

#[test]
fn new_forms_and_joined_parentheses() {
    for src in [
        "(i/fail)\nf = 1",
        "f = {}",
        "f = x |> { n -> n }",
        "f = { n ? c -> n }",
        "f(A) = (x: A) { x }",
        "f(A): (A) -> A",
        "Box(A) { B(A) }",
        "Schema(User, _) = user",
        "f(x)",
        "@m(1)\nf = 1",
        "f = (p: a.name) { p }",
        "f = (p: m(Int, 1, \"s\")) { p }",
    ] {
        module(src);
    }
    for src in [
        "f(A) = x",
        "main = () { wrap(A) = x  wrap }",
        "main = () { k = 1  wrap(A) = (x: A) { Some(x) }  wrap(k) }",
    ] {
        let m = module(src);
        assert!(format!("{m:?}").contains("type_params: [Ident { name: \"A\""));
    }
    // §3.7: a line break prevents a lower TypeCall; inside () it is a comma.
    module("f = (\n x: a\n (y, z)\n) { x }");
}

#[test]
fn guards_and_pipe_binding_strength() {
    // §6.1 tests 1a/2 need an immediate arrow after the first pattern.
    // Choices or a guard prevent the function reading of that head.
    module("f = { n = (x) | n = (x) -> (y) { y } }");
    module("f = { (x) ? True -> (y) { y } }");
    for src in [
        /* old-syntax-check: E0111 */ "a = n |> { 0 -> 1  _ -> 2 } * 10",
        /* old-syntax-check: E0111 */ "a = opt |> { _ -> 1 }.name",
        /* old-syntax-check: E0111 */ "a = opt |> { _ -> 1 }.0",
    ] {
        assert!(
            parse_module(src)
                .1
                .iter()
                .any(|d| d.code == Code::PipeTarget && d.message.contains("e.{ clauses }")),
            "{src}"
        );
    }
    let m = module("a = n |> { 0 -> 1  _ -> 2 }()");
    assert!(matches!(
        value(&m).kind,
        ExprKind::Binary {
            op: BinOp::Pipe,
            ..
        }
    ));
}

#[test]
fn brace_cache_decides_each_nested_brace_once() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            // §3.5: speculative pattern parses may contain expressions with braces.
            for depth in [40, 200] {
                let mut expr = "1".to_string();
                for _ in 0..depth {
                    expr = format!("{{ <<x:u({expr})>> -> 0  _ -> 1 }}");
                }
                let src = format!("a = {expr}");
                let stats = parse_stats(&src);
                assert_eq!(stats.braces, depth);
                assert_eq!(stats.decisions, stats.braces);
                assert!(parse_module(&src).1.is_empty());
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Canon for these core-tree printer tests removes expression groups only
/// (§3.15). The production shape printer stays strict in S1.
fn core_canon(m: &Module) -> String {
    fn expr(e: &mut Expr) {
        fn stmts(b: &mut Block) {
            for s in &mut b.stmts {
                match s {
                    Stmt::Bind(b) => expr(&mut b.value),
                    Stmt::Env { value, .. } | Stmt::Expr(value) => expr(value),
                }
            }
        }
        match &mut e.kind {
            ExprKind::Block(b) => stmts(b),
            ExprKind::Fn(f) => stmts(&mut f.body),
            ExprKind::Case {
                subject, clauses, ..
            } => {
                expr(subject);
                cs(clauses);
            }
            ExprKind::Clauses(clauses) => cs(clauses),
            ExprKind::Tuple(es) => es.iter_mut().for_each(expr),
            ExprKind::List { items, rest } => {
                items.iter_mut().for_each(expr);
                if let Some(e) = rest {
                    expr(e);
                }
            }
            ExprKind::BitArray(segs) => {
                for s in segs {
                    expr(&mut s.value);
                    if let Some(e) = s.format.as_mut().and_then(|f| f.arg.as_deref_mut()) {
                        expr(e);
                    }
                }
            }
            ExprKind::Call { callee, args } => {
                expr(callee);
                args.iter_mut().for_each(|a| expr(&mut a.value));
            }
            ExprKind::Field { target, .. }
            | ExprKind::TupleIndex { target, .. }
            | ExprKind::Unary { expr: target, .. } => expr(target),
            ExprKind::Binary { lhs, rhs, .. } => {
                expr(lhs);
                expr(rhs);
            }
            ExprKind::RecordUpdate {
                constructor,
                base,
                fields,
            } => {
                expr(constructor);
                expr(base);
                fields.iter_mut().for_each(|(_, e)| expr(e));
            }
            _ => {}
        }
        if let ExprKind::Block(b) = &mut e.kind
            && let [Stmt::Expr(inner)] = &b.stmts[..]
        {
            *e = inner.clone();
        }
    }
    fn cs(clauses: &mut [Clause]) {
        for c in clauses {
            c.guard.iter_mut().for_each(expr);
            expr(&mut c.body);
        }
    }
    let mut m = m.clone();
    for i in &mut m.items {
        if let ItemKind::Binding(b) = &mut i.kind {
            expr(&mut b.value);
        }
    }
    common::shape(&m)
}

fn core_value(src: &str) -> Expr {
    value(&module(src)).clone()
}

fn print_core(e: Expr) -> String {
    let mut m = module("a = 0");
    let ItemKind::Binding(b) = &mut m.items[0].kind else {
        unreachable!()
    };
    b.value = e;
    assert_eq!(unprintable(&m), None);
    let printed = print_module(&m);
    let (again, d) = parse_module(&printed);
    assert!(d.is_empty(), "{printed}\n{d:?}");
    assert_eq!(core_canon(&m), core_canon(&again), "{printed}");
    printed
}

#[test]
fn core_printer_adds_only_required_groups() {
    // SYN2 M2: a postfix match under * or as a callee needs no group.
    let match_e = core_value("a = n |> { 0 -> 1  _ -> 2 }");
    let e = Expr {
        kind: ExprKind::Binary {
            op: BinOp::Mul,
            lhs: Box::new(match_e.clone()),
            rhs: Box::new(core_value("a = 10")),
        },
        span: match_e.span,
    };
    assert!(print_core(e).contains("n.{"));
    let e = Expr {
        kind: ExprKind::Call {
            callee: Box::new(match_e.clone()),
            args: Vec::new(),
        },
        span: match_e.span,
    };
    assert!(print_core(e).contains("}()"));
    for (src, grouped) in [("a = a + b", true), ("a = a == b", true)] {
        let mut e = match_e.clone();
        let ExprKind::Case { subject, .. } = &mut e.kind else {
            unreachable!()
        };
        **subject = core_value(src);
        let text = print_core(e);
        assert_eq!(text.starts_with("a = {"), grouped, "{text}");
    }
    // New match first (tuple) pattern needs no group; lone Clauses does.
    let match_fn = core_value("a = v |> { (x) -> (y) { y } }");
    let text = print_core(match_fn.clone());
    assert!(text.contains("-> (y)"), "{text}");
    let ExprKind::Case { ref clauses, .. } = match_fn.kind else {
        unreachable!()
    };
    let e = Expr {
        kind: ExprKind::Clauses(clauses.clone()),
        span: match_e.span,
    };
    assert!(print_core(e).contains("-> (y)"));
    // A chain-to-tuple first pattern still needs a group (§6.1).
    let m = module("a = v |> { n = (x) -> { (y) { y } } }");
    assert!(print_core(value(&m).clone()).contains("-> {"));
    // Negative/tuple patterns are on a new line (§1.7), independent of
    // spans or whether the tree came from the normal surface.
    let text = print_core(core_value("a = n |> { 0 -> a\n-1 -> b }"));
    assert!(text.contains("\n  -1 ->"), "{text}");
    // The function type ending a pattern needs a type group before | or ?.
    print_core(core_value(
        "a = { f: { (Int) -> Int } ? ok -> f(1)  _ -> 0 }",
    ));
}

#[test]
fn unprintable_core_trees_report_their_span() {
    // §3.13: groups cannot save these trees, and the printer must never
    // invent a g binding (which would take the expected type away).
    let mut m = module("a = v |> { n = (a) -> { g = (x) -> Int { x }  g } }");
    let body = core_value("a = (x) -> Int { x }");
    let ItemKind::Binding(b) = &mut m.items[0].kind else {
        unreachable!()
    };
    let ExprKind::Case { clauses, .. } = &mut b.value.kind else {
        unreachable!()
    };
    let span = body.span;
    clauses[0].body = body;
    assert_eq!(unprintable(&m), Some(span));
    // D4 excludes functions whose parameters are not patterns. The strict
    // round-trip guarantee of §3.13 applies to these parser trees too.
    module("a = { (a) -> (n, log ! Log) -> Int { n } }");
}

#[test]
fn wat_diff_counterexamples() {
    // §3.17 requires this test to run under dev-verify from S1. Keeping it
    // in the syntax suite also avoids changing the shared verify script.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::process::Command::new("python3")
        .arg("-I")
        .arg(root.join("scripts/wat-diff-test"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_type_annotation_before_another_choice_has_no_same_code_text() {
    // §3.13's same-code guarantee and §6.1's longest-type rule: even a
    // type group cannot keep the next `|` out of the annotation's union.
    // Parentheses would instead make a tuple pattern (§7.1).
    for src in [
        "a = { f: { (Int) -> Int } ? ok -> f }",
        "a = { f: Int ? ok -> f }",
        "a = { whole = f: Int ? ok -> f }",
        "a = v |> { f: Int ? ok -> f }",
    ] {
        let mut m = module(src);
        let next = core_value("a = { f: String -> f }");
        let ExprKind::Clauses(ref next) = next.kind else {
            unreachable!()
        };
        let ItemKind::Binding(b) = &mut m.items[0].kind else {
            unreachable!()
        };
        let (ExprKind::Clauses(cs) | ExprKind::Case { clauses: cs, .. }) = &mut b.value.kind else {
            unreachable!()
        };
        cs[0].guard = None;
        let span = cs[0].patterns[0].span;
        cs[0].patterns.push(next[0].patterns[0].clone());
        assert_eq!(unprintable(&m), Some(span));
        assert!(!parse_module(&print_module(&m)).1.is_empty());
    }
    // An annotation in a constructor argument ends at `)`, so this parsed
    // tree is representable and must keep the strict round trip.
    module("a = { Some(f: Int) | Some(f: String) -> f }");
}

#[test]
fn names_and_line_breaks_have_no_keyword_exceptions() {
    // Spec §1.4: every word is a name, and any name may be used as a name.
    for word in ["import", "export", "case", "if"] {
        let m = module(&format!("{word} = (x) {{ x }}\nuse = () {{ {word}(1) }}"));
        assert_eq!(m.items.len(), 2);
        let tokens = ilang_syntax::lex(&format!("x\n{word}"));
        assert!(tokens.tokens[1].brk);
    }
    // A3 ruling 1: old header-shaped calls use the same MacroItem grammar.
    for src in ["export(x)", "import(i/core)", "case(x)"] {
        let m = module(src);
        assert!(matches!(m.items[0].kind, ItemKind::MacroItem { .. }));
    }
}

#[test]
fn old_tag_marks_use_tokens_and_do_not_choose_a_grammar() {
    use ilang_syntax::old_syntax_marks;
    for src in [
        "import (i/core)",
        "export (x)",
        /* old-syntax-check: E0101 */ "x = case n { 0 -> 1  _ -> 2 }",
        /* old-syntax-check: E0101 */ "x = case f(n) { Some(x) -> x }",
    ] {
        assert!(old_syntax_marks(src), "{src}");
    }
    for src in [
        "// import (i/core)\nx = 1",
        "x = \"case n { 0 -> 1 }\"",
        "import = (x) { x }",
        "x = () { export(x) }",
        /* old-syntax-check: E0101,E0101 */ "case\nn { 0 -> 1 }",
        /* old-syntax-check: E0101 */ "x = case n { f(0 -> 1) }",
    ] {
        assert!(!old_syntax_marks(src), "{src}");
    }
}

#[test]
fn structural_shape_keeps_all_string_payloads() {
    // Part 1 review finding b remains a regression after the migration API goes.
    for source in [
        "x = \"@1..2\"",
        "/// @1..2\nx = 1",
        "@external(\"@1..2\", \"f\")\nf: () -> Nil",
        "f = { \"@1..2\" -> 1  _ -> 2 }",
    ] {
        let original = module(source);
        let changed = module(&source.replace("@1..2", "@3..4"));
        assert_ne!(
            ilang_syntax::shape::canon(&original),
            ilang_syntax::shape::canon(&changed)
        );
    }
}

#[test]
fn brace_cache_and_uncached_parser_agree() {
    for (path, source) in [
        ("guards", "f = { (x) ? True -> (y) { y } }"),
        ("nested", "f = { <<x:u({ 8 })>> -> 0  _ -> 1 }"),
        (
            "chain",
            /* old-syntax-check: E0101 */ "f = { n = (a) -> { (x) -> Int { x } } }",
        ),
    ] {
        assert_eq!(
            parse_module(source),
            ilang_syntax::parser::parse_module_uncached(source),
            "{path}"
        );
    }
}

#[test]
fn literal_type_grammar_and_type_call_arguments() {
    // types-joint §5: Literal Prim, grouped negatives and TypeArg=Type.
    let m = module(
        r#"A = Int - -1
B = -{1} | 0x10 | "é"
f = (x: "a" | "b") { x }
"#,
    );
    let ItemKind::TypeDef(a) = &m.items[0].kind else {
        panic!()
    };
    let TypeBody::Alias(t) = &a.body else {
        panic!()
    };
    assert!(
        matches!(&t.kind,TypeKind::Diff(ts) if matches!(&ts[1].kind,TypeKind::Lit(LitValue::Int(n)) if *n==(-1).into()))
    );
    module("A = m(-{1}, \"x\" | String)\n");
    for source in [
        /* old-syntax-check: E0101 */ "a: 1.5 = 1",
        /* old-syntax-check: E0101 */ "a: { 1.5 | 2.5 } = 1",
        /* old-syntax-check: E0101 */ "a: -1.5 = 1",
    ] {
        let ds = parse_module(source).1;
        assert!(
            ds.iter().any(|d| d.message == "Float has no literal types"),
            "{ds:?}"
        );
    }
    for source in [
        /* old-syntax-check: E0101 */ "A = -x",
        /* old-syntax-check: E0101 */ "A = -(1)",
        /* old-syntax-check: E0101 */ "A = -{-1}",
    ] {
        assert!(!parse_module(source).1.is_empty(), "{source}");
    }
}
