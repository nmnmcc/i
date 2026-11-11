use ilang_syntax::ast::{ExprKind, PatternKind, TypeKind};
use ilang_syntax::lisp::{Datum, DatumKind as K, parse_expression, parse_pattern, parse_type};
use ilang_syntax::span::Span;

fn datum(kind: K, offset: u32) -> Datum {
    Datum {
        kind,
        span: Span::new(offset, offset + 1),
        docs: Vec::new(),
    }
}
fn name(name: &str) -> Datum {
    datum(K::Name(name.into()), 0)
}
fn upper(name: &str) -> Datum {
    datum(K::UpName(name.into()), 0)
}
fn wrapped(mut d: Datum, head: impl Fn() -> Datum, n: usize) -> Datum {
    for _ in 0..n {
        d = datum(K::Round(vec![head(), d]), 0);
    }
    d
}

#[test]
fn deep_expression_type_and_pattern_convert_and_drop_without_host_recursion() {
    let depth = 10_000;
    let source = wrapped(name("x"), || name("f"), depth);
    let ast = parse_expression(&source).unwrap();
    let mut current = &ast;
    for _ in 0..depth {
        let ExprKind::Call { args, .. } = &current.kind else {
            panic!("expected call")
        };
        current = &args[0].value;
    }
    assert!(matches!(&current.kind, ExprKind::Name { name, .. } if name.name == "x"));
    drop(ast);
    let copy = source.clone();
    drop(source);
    drop(copy);

    let source = wrapped(upper("Int"), || upper("List"), depth);
    let ast = parse_type(&source).unwrap();
    let mut current = &ast;
    for _ in 0..depth {
        let TypeKind::Named { args, .. } = &current.kind else {
            panic!("expected named type")
        };
        current = &args[0];
    }
    assert!(matches!(&current.kind, TypeKind::Named { name, .. } if name.name == "Int"));
    drop(ast);
    drop(source);

    let source = wrapped(name("x"), || upper("Some"), depth);
    let ast = parse_pattern(&source).unwrap();
    let mut current = &ast;
    for _ in 0..depth {
        let PatternKind::Constructor { args, .. } = &current.kind else {
            panic!("expected constructor")
        };
        current = &args[0];
    }
    assert!(matches!(&current.kind, PatternKind::Var(name) if name == "x"));
    drop(ast);
    drop(source);
}

#[test]
fn deep_error_keeps_the_failing_datum_span() {
    let source = wrapped(datum(K::Symbol("?".into()), 42), || name("f"), 10_000);
    let error = parse_expression(&source).unwrap_err();
    assert_eq!(error.span, Span::new(42, 43));
    drop(source);
}

#[test]
fn deep_quoted_datum_can_be_cloned_and_dropped() {
    let source = datum(
        K::Quote(Box::new(wrapped(name("x"), || name("f"), 20_000))),
        0,
    );
    let ast = parse_expression(&source).unwrap();
    drop(source);
    drop(ast);
}

#[test]
fn deep_conversion_and_drop_work_on_a_small_host_stack() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            let source = wrapped(name("x"), || name("f"), 10_000);
            let ast = parse_expression(&source).unwrap();
            drop(ast);
            drop(source);
            let ty = wrapped(upper("Int"), || upper("List"), 10_000);
            drop(parse_type(&ty).unwrap());
            drop(ty);
            let pattern = wrapped(name("x"), || upper("Some"), 10_000);
            drop(parse_pattern(&pattern).unwrap());
            drop(pattern);
            let quoted = datum(
                K::Quote(Box::new(wrapped(name("x"), || name("f"), 20_000))),
                0,
            );
            let copied = quoted.clone();
            drop(parse_expression(&quoted).unwrap());
            drop(quoted);
            drop(copied);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deeply_nested_functions_mix_expression_pattern_and_type_positions() {
    let mut body = name("x");
    for i in 0..1_000 {
        let ty = wrapped(upper("Int"), || upper("List"), 4);
        let param = datum(
            K::Round(vec![datum(K::Symbol(":".into()), i), name("x"), ty]),
            i,
        );
        body = datum(
            K::Round(vec![
                datum(K::Symbol("->".into()), i),
                datum(K::Round(vec![param]), i),
                datum(K::Discard("_".into()), i),
                datum(K::Curly(vec![body]), i),
            ]),
            i,
        );
    }
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let ast = parse_expression(&body).unwrap();
            let mut current = &ast;
            for _ in 0..1_000 {
                let ExprKind::Fn(f) = &current.kind else {
                    panic!("expected function")
                };
                let ilang_syntax::ast::Stmt::Expr(next) = &f.body.stmts[0] else {
                    panic!("expected body")
                };
                current = next;
            }
            assert!(matches!(&current.kind, ExprKind::Name { name, .. } if name.name == "x"));
            drop(ast);
            drop(body);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deep_quote_splice_error_preserves_its_origin() {
    let splice = datum(K::Splice(Box::new(name("capture"))), 77);
    let mut body = datum(K::Round(vec![splice, name("later")]), 0);
    for _ in 0..10_000 {
        body = datum(K::Square(vec![body]), 0);
    }
    let quoted = datum(K::Quote(Box::new(body)), 0);
    let error = parse_pattern(&quoted).unwrap_err();
    assert_eq!(error.span, Span::new(77, 78));
}

#[test]
fn deep_datum_equality_uses_no_recursive_stack() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            use ilang_syntax::lisp::{Datum, DatumKind};
            use ilang_syntax::span::Span;
            let span = Span::new(0, 1);
            let mut value = Datum {
                kind: DatumKind::Name("x".into()),
                span,
                docs: vec![],
            };
            for _ in 0..10_000 {
                value = Datum {
                    kind: DatumKind::Quote(Box::new(value)),
                    span,
                    docs: vec![],
                };
            }
            let same = value.clone();
            assert!(value == same);
            let different = Datum {
                kind: DatumKind::Quote(Box::new(value.clone())),
                span,
                docs: vec![],
            };
            assert!(value != different);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deep_ast_structural_equality_and_difference_are_stack_safe() {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            use ilang_syntax::ast::{Expr, Pattern, TypeExpr};
            let span = Span::new(0, 1);
            let make_expr = |n: u32| {
                let mut e = Expr {
                    kind: ExprKind::Int(n.into()),
                    span,
                };
                for _ in 0..10_000 {
                    e = Expr {
                        kind: ExprKind::Tuple(vec![e]),
                        span,
                    };
                }
                e
            };
            let make_pattern = |name: &str| {
                let mut p = Pattern {
                    kind: PatternKind::Var(name.into()),
                    span,
                };
                for _ in 0..10_000 {
                    p = Pattern {
                        kind: PatternKind::Tuple(vec![p]),
                        span,
                    };
                }
                p
            };
            let make_type = |kind: TypeKind| {
                let mut t = TypeExpr { kind, span };
                for _ in 0..10_000 {
                    t = TypeExpr {
                        kind: TypeKind::Tuple(vec![t]),
                        span,
                    };
                }
                t
            };
            let e = make_expr(1);
            assert!(e.clone() == e);
            let p = make_pattern("a");
            assert!(p.clone() == p);
            let t = make_type(TypeKind::Hole);
            assert!(t.clone() == t);
            assert!(make_expr(1) == make_expr(1));
            assert!(make_expr(1) != make_expr(2));
            assert!(make_pattern("a") == make_pattern("a"));
            assert!(make_pattern("a") != make_pattern("b"));
            assert!(make_type(TypeKind::Hole) == make_type(TypeKind::Hole));
            assert!(
                make_type(TypeKind::Hole)
                    != make_type(TypeKind::Lit(ilang_syntax::ast::LitValue::Int(1.into())))
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

