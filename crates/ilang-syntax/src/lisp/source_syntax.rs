//! Scoped macro input from either source surface, with transparent groups removed.
use super::syntax::{Node, Scope, Syntax};
use super::{Datum, DatumKind};
use crate::ast::{Expr, Item, Pattern, TypeExpr};
use crate::span::Span;

fn find(datum: &Datum, span: Span) -> Option<&Datum> {
    let mut pending = vec![datum];
    while let Some(datum) = pending.pop() {
        if datum.span == span {
            return Some(datum);
        }
        match &datum.kind {
            DatumKind::Round(xs)
            | DatumKind::Square(xs)
            | DatumKind::Curly(xs)
            | DatumKind::Bits(xs) => pending.extend(xs.iter().rev()),
            DatumKind::Quote(x) | DatumKind::Unquote(x) | DatumKind::Splice(x) => pending.push(x),
            _ => {}
        }
    }
    None
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
#[derive(Clone, Copy)]
enum Mode {
    Expr,
    Type,
    Pattern,
    SegmentExpr,
    SegmentPattern,
    Params,
    Param,
    Statement,
    Block,
    ClauseHead,
    Clause,
    Clauses,
    Keep,
}

// Normalize source datums without consuming the host stack on nested input.
fn normalize(root: &Syntax, mode: Mode) -> Syntax {
    enum Task {
        Enter(Syntax, Mode),
        Finish(Syntax, usize, bool),
        Wrap(Syntax),
    }
    let mut work = vec![Task::Enter(root.clone(), mode)];
    let mut built = Vec::new();
    while let Some(task) = work.pop() {
        let (node, mode) = match task {
            Task::Finish(node, count, unwrap) => {
                let children = built.split_off(built.len() - count);
                built.push(if unwrap {
                    children.into_iter().next().expect("group child")
                } else {
                    node.replace_children(children)
                });
                continue;
            }
            Task::Wrap(like) => {
                let child = built.pop().expect("function body");
                built.push(Syntax::written(
                    Node::Curly(vec![child]),
                    like.scope(),
                    like.span(),
                ));
                continue;
            }
            Task::Enter(node, mode) => (node, mode),
        };
        let xs = match node.node() {
            Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => xs,
            _ => {
                built.push(node);
                continue;
            }
        };
        let mut modes = vec![Mode::Keep; xs.len()];
        let mut unwrap = false;
        let mut wrap_last = false;
        match mode {
            Mode::Keep => {
                built.push(node);
                continue;
            }
            Mode::Type => {
                if matches!(node.node(), Node::Curly(_)) && xs.len() == 1 {
                    unwrap = true;
                }
                modes.fill(Mode::Type);
            }
            Mode::Pattern => {
                if matches!(node.node(), Node::Round(_)) && xs.len() == 3 && symbol(&xs[0], ":") {
                    modes[2] = Mode::Type;
                } else if matches!(node.node(), Node::Bits(_)) {
                    modes.fill(Mode::SegmentPattern);
                } else {
                    modes.fill(Mode::Pattern);
                }
            }
            Mode::SegmentExpr | Mode::SegmentPattern => {
                let value = if matches!(mode, Mode::SegmentExpr) {
                    Mode::Expr
                } else {
                    Mode::Pattern
                };
                if matches!(node.node(), Node::Round(_)) && xs.len() == 3 && symbol(&xs[0], ":") {
                    modes[1] = value;
                    modes[2] = Mode::Expr;
                } else {
                    work.push(Task::Enter(node, value));
                    continue;
                }
            }
            Mode::Params => modes.fill(Mode::Param),
            Mode::Param => {
                if matches!(node.node(), Node::Round(_)) && xs.len() == 3 && symbol(&xs[0], "!") {
                    modes[2] = Mode::Type;
                } else {
                    work.push(Task::Enter(node, Mode::Pattern));
                    continue;
                }
            }
            Mode::Statement => {
                if matches!(node.node(), Node::Round(_)) && xs.len() == 3 && symbol(&xs[0], "=") {
                    modes[1] = Mode::Pattern;
                    modes[2] = Mode::Expr;
                } else if matches!(node.node(), Node::Round(_))
                    && xs.len() == 3
                    && symbol(&xs[0], "!")
                {
                    modes[1] = Mode::Type;
                    modes[2] = Mode::Expr;
                } else {
                    work.push(Task::Enter(node, Mode::Expr));
                    continue;
                }
            }
            Mode::Block => modes.fill(Mode::Statement),
            Mode::ClauseHead => {
                if matches!(node.node(), Node::Round(_)) && xs.len() == 3 && symbol(&xs[0], "?") {
                    modes[1] = Mode::Pattern;
                    modes[2] = Mode::Expr;
                } else if matches!(node.node(), Node::Round(_))
                    && !xs.is_empty()
                    && symbol(&xs[0], "|")
                {
                    for m in &mut modes[1..] {
                        *m = Mode::Pattern;
                    }
                } else {
                    work.push(Task::Enter(node, Mode::Pattern));
                    continue;
                }
            }
            Mode::Clause => {
                assert!(
                    xs.len() == 3 && symbol(&xs[0], "->"),
                    "parsed clause has invalid shape"
                );
                modes[1] = Mode::ClauseHead;
                modes[2] = Mode::Expr;
            }
            Mode::Clauses => modes.fill(Mode::Clause),
            Mode::Expr => {
                if matches!(node.node(), Node::Curly(_)) {
                    {
                        let clause = xs.first().is_some_and(|x| matches!(x.node(), Node::Round(parts) if parts.len() == 3 && symbol(&parts[0], "->")));
                        if xs.is_empty() || clause {
                            modes.fill(Mode::Clause);
                        } else if xs.len() == 1
                            && !matches!(xs[0].node(), Node::Round(parts) if parts.len() == 3 && (symbol(&parts[0], "=") || symbol(&parts[0], "!")))
                        {
                            unwrap = true;
                            modes[0] = Mode::Expr;
                        } else {
                            modes.fill(Mode::Statement);
                        }
                    }
                } else if matches!(node.node(), Node::Round(_))
                    && xs.len() == 4
                    && symbol(&xs[0], "->")
                {
                    modes[1] = Mode::Params;
                    modes[2] = Mode::Type;
                    if matches!(xs[3].node(), Node::Curly(_)) {
                        modes[3] = Mode::Block;
                    } else {
                        modes[3] = Mode::Expr;
                        wrap_last = true;
                    }
                } else if matches!(node.node(), Node::Round(_))
                    && xs.len() == 3
                    && symbol(&xs[0], ".")
                    && matches!(xs[2].node(), Node::Curly(_))
                {
                    modes[1] = Mode::Expr;
                    modes[2] = Mode::Clauses;
                } else if matches!(node.node(), Node::Bits(_)) {
                    modes.fill(Mode::SegmentExpr);
                } else {
                    modes.fill(Mode::Expr);
                }
            }
        }
        let children = xs.clone();
        work.push(Task::Finish(node, children.len(), unwrap));
        for (i, (child, mode)) in children.into_iter().zip(modes).enumerate().rev() {
            if wrap_last && i == 3 {
                work.push(Task::Wrap(child.clone()));
            }
            work.push(Task::Enter(child, mode));
        }
    }
    built.pop().expect("normalization produced a root")
}
fn ty(s: &Syntax) -> Syntax {
    normalize(s, Mode::Type)
}
fn pattern(s: &Syntax) -> Syntax {
    normalize(s, Mode::Pattern)
}
fn expr(s: &Syntax) -> Syntax {
    normalize(s, Mode::Expr)
}
fn item(s: &Syntax) -> Syntax {
    let mut wrappers = Vec::new();
    let mut current = s.clone();
    while let Node::Round(xs) = current.node() {
        if let [at, attribute, inner] = xs.as_slice()
            && symbol(at, "@")
        {
            wrappers.push((current.clone(), at.clone(), children(attribute, expr)));
            current = inner.clone();
        } else {
            break;
        }
    }
    let s = &current;
    let parsed = super::parse_item(&s.to_datum()).expect("parsed item remains valid");
    let Node::Round(xs) = s.node() else {
        unreachable!("item is round")
    };
    let mut result = match parsed.kind {
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
                s.replace_children(vec![ty(head), constructors])
            }
            [eq, head, value] if symbol(eq, "=") => {
                s.replace_children(vec![eq.clone(), ty(head), ty(value)])
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
    };
    for (wrapper, at, attribute) in wrappers.into_iter().rev() {
        result = wrapper.replace_children(vec![at, attribute, result]);
    }
    result
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn single_field_records_are_not_transparent_groups() {
        let scope = Scope {
            module: 0,
            marks: BTreeSet::new(),
        };
        let datum = super::super::parse("((: x { T }))").unwrap();
        let syntax = Syntax::from_datum(&datum[0], &scope);
        assert_eq!(ty(&syntax).show(), "((: x T))");
        let datum = super::super::parse("((: x { y }))").unwrap();
        let syntax = Syntax::from_datum(&datum[0], &scope);
        assert_eq!(expr(&syntax).show(), "((: x y))");
    }

    #[test]
    fn deeply_nested_source_normalizes_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let scope = Scope {
                    module: 0,
                    marks: BTreeSet::new(),
                };
                let datum = super::super::parse("x").unwrap();
                let mut value = Syntax::from_datum(&datum[0], &scope);
                for _ in 0..20_000 {
                    value = Syntax::make(Node::Curly(vec![value.clone()]), &value).unwrap();
                }
                assert_eq!(expr(&value).show(), "x");
                assert_eq!(ty(&value).show(), "x");
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
