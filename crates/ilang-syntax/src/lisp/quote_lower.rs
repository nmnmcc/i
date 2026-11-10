//! Lower expression quotes to ordinary calls into the compiler syntax service.
use std::collections::BTreeSet;

use crate::ast::{Arg, Expr, ExprKind, Ident, ImportItem, Module};
use crate::span::Span;
use crate::walk::{Visitor, Walk};

use super::syntax::{Scope, Syntax};
use super::{Datum, DatumKind};

// Source identifiers cannot contain NUL, so this import cannot capture or be
// shadowed by a written name. Its scope is local to the compiler program.
const ALIAS: &str = "\0quote-syntax";

fn ident(name: &str, span: Span) -> Ident {
    Ident {
        name: name.into(),
        span,
    }
}
fn call(name: &str, arguments: Vec<Expr>, span: Span) -> Expr {
    let callee = Expr {
        kind: ExprKind::Name {
            module: Some(ident(ALIAS, span)),
            name: ident(name, span),
        },
        span,
    };
    Expr {
        kind: ExprKind::Call {
            callee: Box::new(callee),
            args: arguments
                .into_iter()
                .map(|value| Arg { label: None, value })
                .collect(),
        },
        span,
    }
}
fn empty(span: Span) -> Expr {
    Expr {
        kind: ExprKind::List {
            items: Vec::new(),
            rest: None,
        },
        span,
    }
}

struct Lower<'a> {
    scope: Scope,
    templates: &'a mut Vec<Syntax>,
    found: bool,
}
impl Lower<'_> {
    fn template(&mut self, d: &Datum) -> Expr {
        let index = self.templates.len();
        self.templates.push(Syntax::from_datum(d, &self.scope));
        let id = Expr {
            kind: ExprKind::Int(index.into()),
            span: d.span,
        };
        call("_template", vec![id], d.span)
    }
    fn node(&mut self, d: &Datum) -> Expr {
        match &d.kind {
            DatumKind::Unquote(inner) => super::parse_expression(inner)
                .expect("the parser validates every unquote expression"),
            DatumKind::Round(children)
            | DatumKind::Square(children)
            | DatumKind::Curly(children)
            | DatumKind::Bits(children) => {
                let template = self.template(d);
                let mut tail = empty(d.span);
                for child in children.iter().rev() {
                    tail = match &child.kind {
                        DatumKind::Splice(inner) => call(
                            "_append",
                            vec![
                                super::parse_expression(inner)
                                    .expect("the parser validates every splice expression"),
                                tail,
                            ],
                            child.span,
                        ),
                        _ => Expr {
                            kind: ExprKind::List {
                                items: vec![self.node(child)],
                                rest: Some(Box::new(tail)),
                            },
                            span: child.span,
                        },
                    };
                }
                call("_rebuild", vec![template, tail], d.span)
            }
            DatumKind::Quote(_) | DatumKind::Splice(_) => {
                unreachable!("the parser validates quote nesting and splice position")
            }
            _ => self.template(d),
        }
    }
}
impl Visitor for Lower<'_> {
    fn expr(&mut self, expr: &mut Expr, enter: bool) {
        if !enter {
            return;
        }
        if let ExprKind::Quote(quote) = &expr.kind {
            let DatumKind::Quote(body) = &quote.kind else {
                unreachable!("quote AST contains a quote datum");
            };
            self.found = true;
            *expr = self.node(body);
        }
    }
}

/// Replace expression quotes in one source module and append the original
/// template nodes to the compiler program's table. Existing AST spans are kept.
/// Pattern quotes are handled by the pattern compiler, not by this pass.
pub fn lower_expression_quotes(module: &mut Module, module_id: u32, templates: &mut Vec<Syntax>) {
    let mut lower = Lower {
        scope: Scope {
            module: module_id,
            marks: BTreeSet::new(),
        },
        templates,
        found: false,
    };
    module.walk(&mut lower);
    if lower.found {
        let span = module
            .items
            .first()
            .map_or(Span::new(0, 0), |item| item.span);
        module.imports.push(ImportItem {
            alias: Some(ident(ALIAS, span)),
            names: Vec::new(),
            path: vec![ident("i", span), ident("syntax", span)],
            span,
        });
    }
}
