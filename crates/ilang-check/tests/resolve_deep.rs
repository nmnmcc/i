//! Resolution of macro-generated trees must not depend on the host call stack.

use ilang_syntax::ast::{self, Expr, ExprKind, Ident};
use ilang_syntax::span::Span;

#[test]
fn deep_mixed_expression_preserves_diagnostic_origin() {
    let (graph, mut parsed) = fixture();
    let origin = Span::new(101, 108);
    let mut expr = Expr {
        kind: ExprKind::Name {
            module: None,
            name: Ident {
                name: "missing".into(),
                span: origin,
            },
        },
        span: origin,
    };
    for depth in 0..10_000 {
        let span = Span::new(0, 1);
        expr = match depth % 4 {
            0 => Expr {
                kind: ExprKind::Tuple(vec![expr]),
                span,
            },
            1 => Expr {
                kind: ExprKind::Call {
                    callee: Box::new(expr),
                    args: vec![],
                },
                span,
            },
            2 => Expr {
                kind: ExprKind::Block(ast::Block {
                    stmts: vec![ast::Stmt::Expr(expr)],
                    span,
                }),
                span,
            },
            _ => Expr {
                kind: ExprKind::Fn(Box::new(ast::Function {
                    params: vec![],
                    ret: None,
                    body: ast::Block {
                        stmts: vec![ast::Stmt::Expr(expr)],
                        span,
                    },
                })),
                span,
            },
        };
    }
    let module = parsed.iter_mut().find(|m| m.path == "app/main").unwrap();
    let ast::ItemKind::Binding(binding) = &mut module.ast.items[0].kind else {
        panic!("fixture binding")
    };
    binding.value = expr;
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let (program, diags) = ilang_check::resolve::resolve(&graph, &parsed);
            assert!(
                diags
                    .iter()
                    .any(|d| d.span == origin && d.message.contains("missing")),
                "{diags:?}"
            );
            drop(program);
        })
        .unwrap()
        .join()
        .unwrap();
}

fn fixture() -> (ilang_package::PackageGraph, Vec<ilang_check::Parsed>) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"app\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/main.i"), "value = 1\n").unwrap();
    let (graph, load_errors) = ilang_package::load(dir.path());
    assert!(load_errors.is_empty(), "{load_errors:?}");
    let (parsed, parse_errors) = ilang_check::parse(&graph);
    assert!(parse_errors.is_empty(), "{parse_errors:?}");
    (graph, parsed)
}

#[test]
fn deep_alternating_pattern_and_format_expression() {
    let (graph, mut parsed) = fixture();
    let span = Span::new(0, 1);
    let origin = Span::new(101, 108);
    let mut expr = Expr {
        kind: ExprKind::Name {
            module: None,
            name: Ident {
                name: "missing".into(),
                span: origin,
            },
        },
        span: origin,
    };
    for _ in 0..10_000 {
        let pattern = ast::Pattern {
            kind: ast::PatternKind::BitArray(vec![ast::Segment {
                value: ast::Pattern {
                    kind: ast::PatternKind::Discard("_".into()),
                    span,
                },
                format: Some(ast::Format {
                    name: Ident {
                        name: "size".into(),
                        span,
                    },
                    arg: Some(Box::new(expr)),
                }),
                span,
            }]),
            span,
        };
        expr = Expr {
            kind: ExprKind::Fn(Box::new(ast::Function {
                params: vec![ast::Param::Pattern(pattern)],
                ret: None,
                body: ast::Block {
                    stmts: vec![],
                    span,
                },
            })),
            span,
        };
    }
    let module = parsed.iter_mut().find(|m| m.path == "app/main").unwrap();
    let ast::ItemKind::Binding(binding) = &mut module.ast.items[0].kind else {
        panic!("fixture binding")
    };
    binding.value = expr;
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let (program, diags) = ilang_check::resolve::resolve(&graph, &parsed);
            assert!(
                diags
                    .iter()
                    .any(|d| d.span == origin && d.message.contains("missing")),
                "{diags:?}"
            );
            drop(program);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deep_pattern_and_type_preserve_diagnostic_origin() {
    let (graph, mut parsed) = fixture();
    let span = Span::new(0, 1);
    let origin = Span::new(101, 108);
    let mut ty = ast::TypeExpr {
        kind: ast::TypeKind::Named {
            module: None,
            name: Ident {
                name: "Missing".into(),
                span: origin,
            },
            args: vec![],
        },
        span: origin,
    };
    for _ in 0..10_000 {
        ty = ast::TypeExpr {
            kind: ast::TypeKind::Tuple(vec![ty]),
            span,
        };
    }
    let mut pattern = ast::Pattern {
        kind: ast::PatternKind::Typed {
            binder: ast::Binder {
                name: "value".into(),
                discard: false,
                span,
            },
            ty,
        },
        span,
    };
    for _ in 0..10_000 {
        pattern = ast::Pattern {
            kind: ast::PatternKind::Tuple(vec![pattern]),
            span,
        };
    }
    let module = parsed.iter_mut().find(|m| m.path == "app/main").unwrap();
    let ast::ItemKind::Binding(binding) = &mut module.ast.items[0].kind else {
        panic!("fixture binding")
    };
    binding.pattern = pattern;
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let (program, diags) = ilang_check::resolve::resolve(&graph, &parsed);
            assert!(
                diags
                    .iter()
                    .any(|d| d.span == origin && d.message.contains("Missing")),
                "{diags:?}"
            );
            drop(program);
        })
        .unwrap()
        .join()
        .unwrap();
}
