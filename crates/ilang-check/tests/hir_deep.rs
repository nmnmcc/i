//! Resolved macro trees may be much deeper than the compiler's host stack.
use ilang_check::hir::*;
use ilang_syntax::span::Span;

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
