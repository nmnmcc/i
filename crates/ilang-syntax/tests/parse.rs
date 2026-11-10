//! Interpretation checks cite A1 design §3.4-§3.7 and §3.13. These values
//! come from those rules, independently of the generated goldens.
mod common;

use ilang_syntax::ast::*;
use ilang_syntax::print::{print_module, unprintable};
use ilang_syntax::{Code, parse_module, parse_module_old, parse_stats};

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
fn old_blocks_never_silently_change_meaning() {
    // D4: exact Old tree, with only E0113 in Both mode.
    for src in [
        "a = { (x) -> Int { x } }",
        "r = { (x) -> Int { x + n }(1) }",
        "a = { (x) -> (y) { y } }",
        "a = { (a, b) -> (x) { x + a } }",
        "a = v |> { (a, b) -> (x) { x + a } }(1)",
        "main = () { 5 |> { (a) -> Int { a }  pair }.0(1) }",
        "h = (x, r) { x |> { (a) -> (b) { a } }.g(1) }",
    ] {
        let (old, od) = parse_module_old(src);
        assert!(od.is_empty(), "{src}\n{od:?}");
        let (both, bd) = parse_module(src);
        assert_eq!(bd.len(), 1, "{src}\n{bd:?}");
        assert_eq!(bd[0].code, Code::DualAmbiguous);
        assert!(bd[0].message.contains("g: (Int) -> T"));
        assert_eq!(common::shape(&old), common::shape(&both), "{src}");
        assert!(
            ilang_syntax::format::format(src).unwrap_err()[0]
                .message
                .contains("--from-old")
        );
    }
    // D4's pattern precondition and §3.5 tests 2, 3, 6.
    for expr in [
        "{ (n, log ! Log) -> Int { log.write(n)  n } }",
        "{ go = (n, log ! Log) -> Int { log.write(n)  n }  go(1) }",
        "{ k = 2  g = (x) -> Int { x * k }  g(1) }",
        "{ x = y  f = (a) -> Int { a }  f(x) }",
    ] {
        let src = format!("a = {expr}");
        let m = module(&src);
        assert!(matches!(value(&m).kind, ExprKind::Block(_)), "{src}");
        let (old, od) = parse_module_old(&src);
        assert!(od.is_empty());
        assert_eq!(common::shape(&old), common::shape(&m));
    }
    // D4 skips test 1a for a bare clause block right after `|>`.
    for expr in [
        "v |> { (a, b) -> (x) { x + a } }",
        "hs |> { (name, _) -> (x) { log(name, x) }  _ -> (x) { x } }",
        "v |> { (a, b) -> (x) -> Int { x + a } }",
    ] {
        let src = format!("a = {expr}");
        let m = module(&src);
        assert!(matches!(
            value(&m).kind,
            ExprKind::Case {
                syntax: Syntax::New,
                ..
            }
        ));
        assert!(
            parse_module_old(&src)
                .1
                .iter()
                .any(|d| d.code == Code::PipeTarget)
        );
    }
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
    // §3.5 test 4 and test 5: records vs a clause with a wrong pattern.
    for (src, code) in [
        ("a = { x: 1 -> 2 }", Code::RecordReserved),
        ("a = { p: (Int, Int) -> e }", Code::RecordReserved),
        ("a = x |> { -y -> 1 }", Code::BadPattern),
        ("a = b |> { <<x: Int>> -> 1 }", Code::Unexpected),
    ] {
        let d = parse_module(src).1;
        assert_eq!(d.first().map(|d| d.code), Some(code), "{src}\n{d:?}");
    }
    // The tuple note is required in later clauses too (§3.6).
    for src in [
        "a = v |> { 0 -> 1  p: (Int, Int) -> e }",
        "a = { 0 -> 1  p: (Int, Int) -> e }",
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
    assert_eq!(m.imports_style, HeaderStyle::Group);
    assert_eq!(m.exports_style, HeaderStyle::Group);
    assert_eq!(m.imports[0].path[0].name, "pkg");
    assert_eq!(m.items.len(), 2);
    // §3.4 keeps empty keyword headers until A3; §3.13 requires every
    // diagnostic-free parser tree to round-trip, including their spans.
    for src in ["import ()", "export ()", "import ()\nexport ()\na = 1"] {
        module(src);
    }
    for src in [
        "()",
        "(main)\n(i/fail)",
        "(main)\n(main)",
        "(i/fail)\n(i/core)",
        "x = 1\n(main)",
        "@deprecated\n(\"x\")\nmain = () { Nil }",
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
fn new_forms_need_both_mode_and_joined_parentheses() {
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
        assert!(!parse_module_old(src).1.is_empty(), "{src}");
    }
    for src in [
        "f(A) = x",
        "main = () { wrap(A) = x  wrap }",
        "main = () { k = 1  wrap(A) = (x: A) { Some(x) }  wrap(k) }",
    ] {
        let m = module(src);
        assert!(!parse_module_old(src).1.is_empty());
        assert!(format!("{m:?}").contains("type_params: [Ident { name: \"A\""));
    }
    // §3.7: a line break prevents a lower TypeCall; inside () it is a comma.
    module("f = (\n x: a\n (y, z)\n) { x }");
    let m = module("f = () { g\n(A) = x\nA }");
    assert_eq!(
        common::shape(&m),
        common::shape(&parse_module_old("f = () { g\n(A) = x\nA }").0)
    );
    // J1's two forms, §6.1 (vii): one parse error, one formatter refusal.
    for src in [
        "Id(a) = a  (p, q) = (1, 2)",
        "@external(\"m\", \"f\") f: a  (p) = 1",
    ] {
        assert!(parse_module_old(src).1.is_empty());
        assert!(
            ilang_syntax::format::format(src)
                .unwrap_err()
                .iter()
                .any(|d| d.message.contains("--from-old"))
        );
    }
}

#[test]
fn guards_and_pipe_binding_strength() {
    // §6.1 tests 1a/2 need an immediate arrow after the first pattern.
    // Choices or a guard prevent the function reading of that head.
    module("f = { n = (x) | n = (x) -> (y) { y } }");
    module("f = { (x) ? True -> (y) { y } }");
    // §3.6, including the `if` token in brace test 2.
    for src in [
        "a = v |> { x if c -> 1 }",
        "a = v |> { 0 -> 1  x if c -> 2 }",
    ] {
        assert!(
            parse_module(src)
                .1
                .iter()
                .any(|d| d.message == "a guard in a clause block is written with `?`")
        );
    }
    assert!(
        parse_module("a = case v { x ? c -> 1 }")
            .1
            .iter()
            .any(|d| d.message.contains("expected `->`"))
    );
    for src in [
        "a = n |> { 0 -> 1  _ -> 2 } * 10",
        "a = opt |> { _ -> 1 }.name",
        "a = opt |> { _ -> 1 }.0",
    ] {
        assert!(
            parse_module(src)
                .1
                .iter()
                .any(|d| d.code == Code::PipeTarget && d.message.contains("level 6")),
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
    // §3.13: a core match under * and as a callee needs its own group.
    let match_e = core_value("a = n |> { 0 -> 1  _ -> 2 }");
    let e = Expr {
        kind: ExprKind::Binary {
            op: BinOp::Mul,
            lhs: Box::new(match_e.clone()),
            rhs: Box::new(core_value("a = 10")),
        },
        span: match_e.span,
    };
    assert!(print_core(e).contains("{ n |>"));
    let e = Expr {
        kind: ExprKind::Call {
            callee: Box::new(match_e.clone()),
            args: Vec::new(),
        },
        span: match_e.span,
    };
    assert!(print_core(e).contains("}()"));
    for (src, grouped) in [("a = a + b", false), ("a = a == b", true)] {
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
    let ExprKind::Case { clauses, .. } = match_fn.kind else {
        unreachable!()
    };
    let e = Expr {
        kind: ExprKind::Clauses(clauses.clone()),
        span: match_e.span,
    };
    assert!(print_core(e).contains("-> {"));
    // A chain-to-tuple first pattern needs a group in a new match, but no
    // clause grouping rule applies to an old Case.
    let mut m = parse_module_old("a = case v { n = (x) -> (y) { y } }").0;
    let old = print_module(&m);
    assert!(old.contains("-> (y)"));
    let ItemKind::Binding(b) = &mut m.items[0].kind else {
        unreachable!()
    };
    let ExprKind::Case { syntax, .. } = &mut b.value.kind else {
        unreachable!()
    };
    *syntax = Syntax::New;
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
    let m = parse_module_old("a = { (x) -> Int { x }(1)  2 }").0;
    assert_eq!(unprintable(&m), Some(value(&m).span));
    let m = parse_module_old("a = case v { (a) -> (x) -> Int { x } }").0;
    let ExprKind::Case { clauses, .. } = &value(&m).kind else {
        unreachable!()
    };
    let span = clauses[0].body.span;
    let mut lone = module("a = {}");
    let ItemKind::Binding(b) = &mut lone.items[0].kind else {
        unreachable!()
    };
    b.value.kind = ExprKind::Clauses(clauses.clone());
    assert_eq!(unprintable(&lone), Some(span));
    assert!(!print_module(&lone).contains("g ="));
    let mut m = parse_module_old("a = case v { n = (a) -> (x) -> Int { x } }").0;
    let ItemKind::Binding(b) = &mut m.items[0].kind else {
        unreachable!()
    };
    let ExprKind::Case {
        syntax, clauses, ..
    } = &mut b.value.kind
    else {
        unreachable!()
    };
    *syntax = Syntax::New;
    let span = clauses[0].body.span;
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
        "a = case v { f: Int if ok -> f }",
    ] {
        let mut m = module(src);
        let next = core_value("a = { f: String -> f }");
        let ExprKind::Clauses(next) = next.kind else {
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
