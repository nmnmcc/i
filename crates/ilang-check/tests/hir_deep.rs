//! Resolved macro trees may be much deeper than the compiler's host stack.
use ilang_check::hir::*;
use ilang_syntax::{ast::UnOp, span::Span};

fn tiny_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}
fn e(kind: ExprKind, n: u32) -> Expr {
    Expr {
        kind,
        span: Span::new(n, n + 1),
    }
}
fn p(kind: PatternKind, n: u32) -> Pattern {
    Pattern {
        kind,
        span: Span::new(n, n + 1),
    }
}
fn t(kind: TypeKind, n: u32) -> Type {
    Type {
        kind,
        span: Span::new(n, n + 1),
    }
}

#[test]
fn resolved_trees_clone_walk_and_drop_on_small_stack() {
    tiny_stack(|| {
        let mut expr = e(ExprKind::Int(7u32.into()), 0);
        let mut pattern = p(
            PatternKind::Typed {
                binder: None,
                ty: t(TypeKind::Hole, 0),
            },
            0,
        );
        let mut ty = t(TypeKind::Hole, 0);
        for n in 1..=10_000 {
            expr = e(
                ExprKind::Unary {
                    op: UnOp::Neg,
                    expr: Box::new(expr),
                },
                n,
            );
            pattern = p(PatternKind::Tuple(vec![pattern]), n);
            ty = t(TypeKind::Tuple(vec![ty]), n);
        }
        let cloned = expr.clone();
        let mut node = &cloned;
        for n in (1..=10_000).rev() {
            assert_eq!(node.span.start, n);
            match &node.kind {
                ExprKind::Unary { expr, .. } => node = expr,
                _ => panic!("lost expression at {n}"),
            }
        }
        assert!(matches!(node.kind, ExprKind::Int(_)));
        let cloned_pattern = pattern.clone();
        let mut node = &cloned_pattern;
        for n in (1..=10_000).rev() {
            assert_eq!(node.span.start, n);
            match &node.kind {
                PatternKind::Tuple(xs) => node = &xs[0],
                _ => panic!("lost pattern at {n}"),
            }
        }
        let cloned_type = ty.clone();
        let mut spans = Vec::new();
        walk_type(&cloned_type, &mut |t| spans.push(t.span.start));
        assert_eq!(spans, (0..=10_000).rev().collect::<Vec<_>>());
        let binding = Binding {
            pattern: cloned_pattern,
            value: cloned,
            direct: false,
            ty_vars: vec![],
            span: Span::default(),
        };
        let mut found = 0;
        binding_types(&binding, &mut |_| found += 1);
        assert_eq!(found, 1);
        drop((binding, expr, pattern, ty, cloned_type));
    });
}

#[test]
fn nested_type_patterns_clone_and_drop_on_small_stack() {
    tiny_stack(|| {
        let mut p = TypePattern::Exact(t(TypeKind::Var(TyVarId(7)), 0));
        for _ in 0..10_000 {
            p = TypePattern::Tuple(vec![p]);
        }
        let clone = p.clone();
        let mut node = &clone;
        for _ in 0..10_000 {
            match node {
                TypePattern::Tuple(xs) => node = &xs[0],
                _ => panic!("lost nested pattern"),
            }
        }
        assert!(matches!(
            node,
            TypePattern::Exact(Type {
                kind: TypeKind::Var(TyVarId(7)),
                ..
            })
        ));
        drop((p, clone));
    });
}

#[test]
fn binding_type_visitor_preserves_order_across_mixed_nodes() {
    let typed = |n| {
        p(
            PatternKind::Typed {
                binder: None,
                ty: t(TypeKind::Hole, n),
            },
            n,
        )
    };
    let value = e(
        ExprKind::Fn(Box::new(Function {
            params: vec![Param::Pattern {
                pattern: typed(1),
                label: None,
            }],
            ret: Some(RetType {
                value: t(TypeKind::Hole, 2),
                err: Some(t(TypeKind::Hole, 3)),
                env: Some(t(TypeKind::Hole, 4)),
            }),
            body: Block {
                stmts: vec![Stmt::Env {
                    ty: t(TypeKind::Hole, 5),
                    value: e(ExprKind::Error, 6),
                    span: Span::default(),
                }],
                span: Span::default(),
            },
            span: Span::default(),
        })),
        0,
    );
    let binding = Binding {
        pattern: typed(0),
        value,
        direct: false,
        ty_vars: vec![],
        span: Span::default(),
    };
    let mut got = Vec::new();
    binding_types(&binding, &mut |t| got.push(t.span.start));
    assert_eq!(got, vec![0, 1, 2, 3, 4, 5]);
}

#[test]
fn reference_order_and_function_boundary_survive_deep_trees() {
    tiny_stack(|| {
        let def = |index| {
            ValueRef::Def(DefId {
                module: ModuleId(7),
                index,
            })
        };
        let first = e(ExprKind::Name(def(1)), 1);
        let second = e(ExprKind::Name(def(2)), 2);
        let third = e(ExprKind::Name(def(3)), 3);
        let function = e(
            ExprKind::Fn(Box::new(Function {
                params: vec![],
                ret: None,
                body: Block {
                    stmts: vec![Stmt::Expr(second)],
                    span: Span::default(),
                },
                span: Span::default(),
            })),
            0,
        );
        let mut body = e(ExprKind::Tuple(vec![first, function, third]), 0);
        for n in 1..=10_000 {
            body = e(
                ExprKind::Unary {
                    op: UnOp::Neg,
                    expr: Box::new(body),
                },
                n,
            );
        }
        let refs = ilang_check::order::references(&p(PatternKind::Discard, 0), &body);
        assert_eq!(
            refs,
            vec![
                (
                    DefId {
                        module: ModuleId(7),
                        index: 1
                    },
                    false
                ),
                (
                    DefId {
                        module: ModuleId(7),
                        index: 2
                    },
                    true
                ),
                (
                    DefId {
                        module: ModuleId(7),
                        index: 3
                    },
                    false
                )
            ]
        );
    });
}

#[test]
fn value_dependency_follows_deep_aliases_and_cycles_on_small_stack() {
    tiny_stack(|| {
        let mut program = Program::default();
        program.of_sites.push(OfSite {
            module: ModuleId(0),
            span: Span::default(),
            value: ValueRef::Local(VarId(0)),
            projection: None,
            permanent: false,
        });
        for i in 0..10_000u32 {
            let body = if i == 9_999 {
                t(
                    TypeKind::Of {
                        value: ValueRef::Local(VarId(0)),
                        site: OfId(0),
                        projection: None,
                    },
                    i,
                )
            } else {
                t(
                    TypeKind::Named {
                        ty: TypeRef::Custom(TypeId(i + 1)),
                        args: vec![],
                    },
                    i,
                )
            };
            program.types.push(TypeDef {
                name: format!("T{i}"),
                span: Span::default(),
                module: ModuleId(0),
                params: vec![],
                body: TypeBody::Alias(body),
                exported: false,
                constructors_exported: false,
                host: None,
                deprecated: None,
            });
        }
        let first = t(
            TypeKind::Named {
                ty: TypeRef::Custom(TypeId(0)),
                args: vec![],
            },
            0,
        );
        assert!(value_dependent(&program, &first));
        program.types[9_999].body = TypeBody::Alias(t(
            TypeKind::Named {
                ty: TypeRef::Custom(TypeId(0)),
                args: vec![],
            },
            0,
        ));
        assert!(!value_dependent(&program, &first));
    });
}

#[test]
fn quote_capture_clone_visitors_and_drop_on_small_stack() {
    tiny_stack(|| {
        let functions = [0, 1, 2, 3, 4].map(|index| DefId {
            module: ModuleId(3),
            index,
        });
        let mut pattern = p(
            PatternKind::Typed {
                binder: None,
                ty: t(TypeKind::Hole, 42),
            },
            0,
        );
        for n in 1..=10_000 {
            pattern = p(
                PatternKind::Quote {
                    template: n,
                    syntax: TypeId(8),
                    functions,
                    captures: vec![(n % 2 == 0, pattern)],
                },
                n,
            );
        }
        let cloned = pattern.clone();
        let mut node = &cloned;
        for n in (1..=10_000).rev() {
            match &node.kind {
                PatternKind::Quote {
                    template,
                    syntax,
                    functions: fs,
                    captures,
                } => {
                    assert_eq!((*template, *syntax, *fs), (n, TypeId(8), functions));
                    assert_eq!(captures.len(), 1);
                    assert_eq!(captures[0].0, n % 2 == 0);
                    node = &captures[0].1;
                }
                _ => panic!("lost quote capture at {n}"),
            }
        }
        let body = e(ExprKind::Error, 0);
        let refs = ilang_check::order::references(&cloned, &body);
        assert_eq!(refs.len(), 50_000);
        for chunk in refs.as_chunks::<5>().0 {
            assert_eq!(chunk, &functions.map(|f| (f, false)));
        }
        let binding = Binding {
            pattern: cloned,
            value: body,
            direct: false,
            ty_vars: vec![],
            span: Span::default(),
        };
        let mut spans = Vec::new();
        binding_types(&binding, &mut |ty| spans.push(ty.span.start));
        assert_eq!(spans, [42]);
        drop((pattern, binding));
    });
}
