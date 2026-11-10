//! Scoped macro input from either source surface, with transparent groups removed.
use super::syntax::{Node, Scope, Syntax};
use super::{Datum, DatumKind};
use crate::ast::{Expr, Item, Pattern, TypeExpr};
use crate::span::Span;

fn find(datum: &Datum, span: Span) -> Option<&Datum> {
    if datum.span == span {
        return Some(datum);
    }
    match &datum.kind {
        DatumKind::Round(xs)
        | DatumKind::Square(xs)
        | DatumKind::Curly(xs)
        | DatumKind::Bits(xs) => xs.iter().find_map(|x| find(x, span)),
        DatumKind::Quote(x) | DatumKind::Unquote(x) | DatumKind::Splice(x) => find(x, span),
        _ => None,
    }
}
fn lisp_source(span: Span, scope: &Scope, source: &str) -> Syntax {
    let document = super::parse(source).expect("parsed Lisp source remains valid");
    let datum = document
        .iter()
        .find_map(|x| find(x, span))
        .expect("AST node belongs to a Lisp datum");
    Syntax::from_datum(datum, scope)
}
fn symbol(s: &Syntax, expected: &str) -> bool {
    matches!(s.node(), Node::Symbol(value) if value == expected)
}
fn children(s: &Syntax, f: impl Fn(&Syntax) -> Syntax) -> Syntax {
    match s.node() {
        Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
            s.replace_children(xs.iter().map(f).collect())
        }
        _ => s.clone(),
    }
}
fn ty(s: &Syntax) -> Syntax {
    match s.node() {
        Node::Curly(xs) if xs.len() == 1 => ty(&xs[0]),
        Node::Quote(_) | Node::Unquote(_) | Node::Splice(_) => s.clone(),
        _ => children(s, ty),
    }
}
fn pattern(s: &Syntax) -> Syntax {
    match s.node() {
        Node::Round(xs) if xs.len() == 3 && symbol(&xs[0], ":") => {
            s.replace_children(vec![xs[0].clone(), xs[1].clone(), ty(&xs[2])])
        }
        Node::Bits(xs) => s.replace_children(xs.iter().map(|x| segment(x, pattern)).collect()),
        Node::Quote(_) | Node::Unquote(_) | Node::Splice(_) => s.clone(),
        _ => children(s, pattern),
    }
}
fn segment(s: &Syntax, value: fn(&Syntax) -> Syntax) -> Syntax {
    match s.node() {
        Node::Round(xs) if xs.len() == 3 && symbol(&xs[0], ":") => {
            s.replace_children(vec![xs[0].clone(), value(&xs[1]), expr(&xs[2])])
        }
        _ => value(s),
    }
}
fn params(s: &Syntax) -> Syntax {
    children(s, |p| match p.node() {
        Node::Round(xs) if xs.len() == 3 && symbol(&xs[0], "!") => {
            p.replace_children(vec![xs[0].clone(), xs[1].clone(), ty(&xs[2])])
        }
        _ => pattern(p),
    })
}
fn statement(s: &Syntax) -> Syntax {
    match s.node() {
        Node::Round(xs) if xs.len() == 3 && symbol(&xs[0], "=") => {
            s.replace_children(vec![xs[0].clone(), pattern(&xs[1]), expr(&xs[2])])
        }
        Node::Round(xs) if xs.len() == 3 && symbol(&xs[0], "!") => {
            s.replace_children(vec![xs[0].clone(), ty(&xs[1]), expr(&xs[2])])
        }
        _ => expr(s),
    }
}
fn block(s: &Syntax) -> Syntax {
    children(s, statement)
}
fn clause_head(s: &Syntax) -> Syntax {
    match s.node() {
        Node::Round(xs) if xs.len() == 3 && symbol(&xs[0], "?") => {
            s.replace_children(vec![xs[0].clone(), pattern(&xs[1]), expr(&xs[2])])
        }
        Node::Round(xs) if !xs.is_empty() && symbol(&xs[0], "|") => s.replace_children(
            xs.iter()
                .enumerate()
                .map(|(i, x)| if i == 0 { x.clone() } else { pattern(x) })
                .collect(),
        ),
        _ => pattern(s),
    }
}
fn clause(s: &Syntax) -> Syntax {
    match s.node() {
        Node::Round(xs) if xs.len() == 3 && symbol(&xs[0], "->") => {
            s.replace_children(vec![xs[0].clone(), clause_head(&xs[1]), expr(&xs[2])])
        }
        _ => panic!("parsed clause has invalid shape"),
    }
}
fn clauses(s: &Syntax) -> Syntax {
    children(s, clause)
}
fn expr(s: &Syntax) -> Syntax {
    match s.node() {
        Node::Curly(xs) => {
            let ast =
                super::parse_expression(&s.to_datum()).expect("parsed expression remains valid");
            match ast.kind {
                crate::ast::ExprKind::Clauses(_) => clauses(s),
                crate::ast::ExprKind::Block(ref b)
                    if b.stmts.len() == 1 && matches!(&b.stmts[0], crate::ast::Stmt::Expr(_)) =>
                {
                    expr(&xs[0])
                }
                crate::ast::ExprKind::Block(_) => block(s),
                _ => unreachable!("curly expression is a block or clauses"),
            }
        }
        Node::Round(xs) if xs.len() == 4 && symbol(&xs[0], "->") => {
            let body = if matches!(xs[3].node(), Node::Curly(_)) {
                block(&xs[3])
            } else {
                Syntax::written(Node::Curly(vec![expr(&xs[3])]), xs[3].scope(), xs[3].span())
            };
            s.replace_children(vec![xs[0].clone(), params(&xs[1]), ty(&xs[2]), body])
        }
        Node::Round(xs)
            if xs.len() == 3 && symbol(&xs[0], ".") && matches!(xs[2].node(), Node::Curly(_)) =>
        {
            s.replace_children(vec![xs[0].clone(), expr(&xs[1]), clauses(&xs[2])])
        }
        Node::Bits(xs) => s.replace_children(xs.iter().map(|x| segment(x, expr)).collect()),
        Node::Quote(_) | Node::Unquote(_) | Node::Splice(_) => s.clone(),
        _ => children(s, expr),
    }
}
fn item(s: &Syntax) -> Syntax {
    let parsed = super::parse_item(&s.to_datum()).expect("parsed item remains valid");
    let Node::Round(xs) = s.node() else {
        unreachable!("item is round")
    };
    if let [at, attribute, inner] = xs.as_slice()
        && symbol(at, "@")
    {
        return s.replace_children(vec![at.clone(), children(attribute, expr), item(inner)]);
    }
    match parsed.kind {
        crate::ast::ItemKind::TypeDef(_) => match xs.as_slice() {
            [head, body] if matches!(body.node(), Node::Curly(_)) => {
                let constructors = children(body, |constructor| match constructor.node() {
                    Node::Round(fields) if !fields.is_empty() => constructor.replace_children(
                        fields
                            .iter()
                            .enumerate()
                            .map(|(i, x)| if i == 0 { x.clone() } else { ty(x) })
                            .collect(),
                    ),
                    _ => constructor.clone(),
                });
                s.replace_children(vec![head.clone(), constructors])
            }
            [eq, head, value] if symbol(eq, "=") => {
                s.replace_children(vec![eq.clone(), head.clone(), ty(value)])
            }
            _ => unreachable!("type definition shape"),
        },
        crate::ast::ItemKind::Binding(_) => match xs.as_slice() {
            [eq, head, value] if symbol(eq, "=") => {
                s.replace_children(vec![eq.clone(), pattern(head), expr(value)])
            }
            _ => unreachable!("binding shape"),
        },
        crate::ast::ItemKind::Declaration { .. } => match xs.as_slice() {
            [colon, head, value] if symbol(colon, ":") => {
                s.replace_children(vec![colon.clone(), head.clone(), ty(value)])
            }
            _ => unreachable!("declaration shape"),
        },
        crate::ast::ItemKind::MacroItem { .. } => children(s, expr),
    }
}

/// Encode a source expression for a macro, preserving each written atom's span.
pub fn source_expression_syntax(
    expr_ast: &Expr,
    scope: &Scope,
    source: &str,
    lisp: bool,
) -> Syntax {
    if lisp {
        expr(&lisp_source(expr_ast.span, scope, source))
    } else {
        super::expression_syntax(expr_ast, scope, source)
    }
}
/// Encode a source type for a macro, preserving each written atom's span.
pub fn source_type_syntax(type_ast: &TypeExpr, scope: &Scope, source: &str, lisp: bool) -> Syntax {
    if lisp {
        ty(&lisp_source(type_ast.span, scope, source))
    } else {
        super::type_syntax(type_ast, scope, source)
    }
}
/// Encode a source item for a macro, preserving each written atom's span.
pub fn source_item_syntax(item_ast: &Item, scope: &Scope, source: &str, lisp: bool) -> Syntax {
    if lisp {
        item(&lisp_source(item_ast.span, scope, source))
    } else {
        super::item_syntax(item_ast, scope, source)
    }
}
/// Encode a source pattern for a macro, preserving each written atom's span.
pub fn source_pattern_syntax(
    pattern_ast: &Pattern,
    scope: &Scope,
    source: &str,
    lisp: bool,
) -> Syntax {
    if lisp {
        pattern(&lisp_source(pattern_ast.span, scope, source))
    } else {
        super::pattern_syntax(pattern_ast, scope, source)
    }
}
