//! `quote_lower` on the tree of syntax three: expression quotes become
//! ordinary calls into the compiler syntax service, and quote patterns get
//! their template slot. Also the reader of a quote pattern's captures.
use std::collections::BTreeSet;

use crate::span::Span;
use crate::syn3::ast::{
    DItem, Expr, ExprKind, Ident, Import, Module, Pattern, PatternKind, QuotePattern, Quoted,
};
use crate::syn3::walk::{self, Visitor};
use crate::{Code, Diagnostic};

use super::syntax::{Scope, Syntax};
use super::{Datum, DatumKind};

// Source identifiers cannot contain NUL, so this import cannot capture or be
// shadowed by a written name. Its scope is local to the compiler program.
const ALIAS: &str = "\0quote-syntax";

/// Reads a quote pattern: its unquotes are patterns in the rows of syntax
/// three (§6.8).
pub fn read_pattern(d: &Datum) -> Result<QuotePattern, Diagnostic> {
    Ok(QuotePattern {
        datum: Quoted(Box::new(d.clone())),
        template: None,
        captures: crate::syn3::lisp::quote_captures(d, true)?,
    })
}

/// [`read_pattern`], or the quote with no captures when it does not read.
/// For the time in which the lowering to the old tree still checks every
/// quote pattern (in the rows before syntax three) and reports the error.
pub fn read_pattern_lenient(d: &Datum) -> QuotePattern {
    read_pattern(d).unwrap_or_else(|_| QuotePattern::unread(d.clone()))
}

/// Checks an expression quote: its shape, and that every unquote reads as
/// an expression.
pub fn check_expression(d: &Datum) -> Result<(), Diagnostic> {
    crate::syn3::lisp::quote_captures(d, false).map(|_| ())
}

fn ident(name: &str, span: Span) -> Ident {
    Ident::new(name, span)
}
fn call(name: &str, arguments: Vec<Expr>, span: Span) -> Expr {
    let callee = Expr::new(
        ExprKind::Name {
            module: Some(ident(ALIAS, span)),
            name: ident(name, span),
        },
        span,
    );
    Expr::new(
        ExprKind::Call {
            callee: Box::new(callee),
            args: arguments.into_iter().map(DItem::Pos).collect(),
        },
        span,
    )
}
fn empty(span: Span) -> Expr {
    Expr::new(
        ExprKind::List {
            items: Vec::new(),
            rest: None,
        },
        span,
    )
}

struct Lower<'a> {
    scope: Scope,
    templates: &'a mut Vec<Syntax>,
    found: bool,
    errors: Vec<Diagnostic>,
}
impl Lower<'_> {
    fn template(&mut self, d: &Datum) -> Expr {
        let index = self.templates.len();
        self.templates.push(Syntax::from_datum(d, &self.scope));
        let id = Expr::new(ExprKind::Int(index.into()), d.span);
        call("_template", vec![id], d.span)
    }
    fn unquoted(&mut self, inner: &Datum) -> Expr {
        match crate::syn3::lisp::expr_from_datum(inner) {
            Ok(e) => e,
            Err(error) => {
                self.errors.push(error);
                empty(inner.span)
            }
        }
    }
    fn node(&mut self, d: &Datum) -> Expr {
        match &d.kind {
            DatumKind::Unquote(inner) => self.unquoted(inner),
            DatumKind::Round(children)
            | DatumKind::Square(children)
            | DatumKind::Curly(children)
            | DatumKind::Bits(children) => {
                let template = self.template(d);
                let mut tail = empty(d.span);
                for child in children.iter().rev() {
                    tail = match &child.kind {
                        DatumKind::Splice(inner) => {
                            call("_append", vec![self.unquoted(inner), tail], child.span)
                        }
                        _ => Expr::new(
                            ExprKind::List {
                                items: vec![self.node(child)],
                                rest: Some(Box::new(tail)),
                            },
                            child.span,
                        ),
                    };
                }
                call("_rebuild", vec![template, tail], d.span)
            }
            DatumKind::Quote(_) | DatumKind::Splice(_) => {
                self.errors.push(Diagnostic::new(
                    Code::Unexpected,
                    d.span,
                    match d.kind {
                        DatumKind::Quote(_) => "expected a nested quote",
                        _ => "expected a splice in this position",
                    },
                ));
                empty(d.span)
            }
            _ => self.template(d),
        }
    }
}
impl Visitor for Lower<'_> {
    fn pattern(&mut self, pattern: &mut Pattern, enter: bool) {
        if !enter {
            return;
        }
        if let PatternKind::Quote(quote) = &mut pattern.kind {
            self.found = true;
            quote.template = Some(
                self.templates
                    .len()
                    .try_into()
                    .expect("too many quote templates"),
            );
            self.templates
                .push(Syntax::from_datum(&quote.datum.0, &self.scope));
        }
    }

    fn expr(&mut self, expr: &mut Expr, enter: bool) {
        if !enter {
            return;
        }
        if let ExprKind::Quote(quote) = &expr.kind {
            let lowered = match &quote.0.kind {
                DatumKind::Quote(body) => self.node(body),
                _ => {
                    self.errors.push(Diagnostic::new(
                        Code::Unexpected,
                        quote.0.span,
                        "expected a quote",
                    ));
                    empty(quote.0.span)
                }
            };
            self.found = true;
            // The walker goes on into the replacement: a quote inside an
            // unquote is lowered in its turn.
            *expr = lowered;
        }
    }
}

/// Replace expression quotes in one source module and append the original
/// template nodes to the compiler program's table. Existing spans are kept.
/// Pattern quotes get their template slot here and are matched by the
/// pattern compiler. A quote that does not read is reported; a tree read by
/// `syn3::lisp::from_lisp` with [`check_expression`] has none.
///
/// The walk is a plain recursion: run it on a deep stack (`syn3::deep`)
/// when the module can hold the result of a macro.
pub fn lower_expression_quotes(
    module: &mut Module,
    module_id: u32,
    templates: &mut Vec<Syntax>,
) -> Vec<Diagnostic> {
    let mut lower = Lower {
        scope: Scope {
            module: module_id,
            marks: BTreeSet::new(),
        },
        templates,
        found: false,
        errors: Vec::new(),
    };
    walk::module(module, &mut lower);
    if lower.found {
        let span = module
            .items
            .first()
            .map_or(Span::new(0, 0), |item| item.span.0);
        module.imports.push(Import {
            alias: Some(ident(ALIAS, span)),
            names: Vec::new(),
            path: vec![ident("i", span), ident("syntax", span)],
            span: span.into(),
        });
    }
    lower.errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syn3::ast::{ItemKind, Stmt};

    fn module(source: &str) -> Module {
        let (module, errors) = crate::syn3::lisp::from_lisp(source);
        assert!(errors.is_empty(), "{errors:?}");
        module
    }

    fn callee_name(e: &Expr) -> &str {
        let ExprKind::Call { callee, .. } = &e.kind else {
            panic!("not a call: {e:?}")
        };
        let ExprKind::Name {
            module: Some(alias),
            name,
        } = &callee.kind
        else {
            panic!("not a service call: {callee:?}")
        };
        assert_eq!(alias.name, ALIAS);
        &name.name
    }

    #[test]
    fn expression_quote_becomes_service_calls_and_an_import() {
        let mut m = module("(= f (-> (, x) _ `(g ~x (, ~..x))))");
        let mut templates = Vec::new();
        let errors = lower_expression_quotes(&mut m, 7, &mut templates);
        assert!(errors.is_empty(), "{errors:?}");
        let ItemKind::Bind(binding) = &m.items[0].kind else {
            panic!()
        };
        let ExprKind::Chain(fs) = &binding.value.kind else {
            panic!()
        };
        let Stmt::Expr(body) = &fs[0].body.stmts[0] else {
            panic!()
        };
        assert_eq!(callee_name(body), "_rebuild");
        // `(g ~x (, ~..x))`, `g`, `(, ~..x)` and `,`.
        assert_eq!(templates.len(), 4);
        let alias = m.imports.last().unwrap();
        assert_eq!(alias.alias.as_ref().unwrap().name, ALIAS);
        assert_eq!(
            alias
                .path
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["i", "syntax"]
        );
    }

    #[test]
    fn pattern_quote_gets_a_slot_and_its_captures() {
        let mut m = module("(= f (-> (, `(g ~a ~..rest)) _ a))");
        let ItemKind::Bind(binding) = &m.items[0].kind else {
            panic!()
        };
        let ExprKind::Chain(fs) = &binding.value.kind else {
            panic!()
        };
        let PatternKind::Quote(quote) = &fs[0].params[0].choices[0].kind else {
            panic!()
        };
        assert_eq!(quote.template, None);
        assert_eq!(
            quote
                .captures
                .iter()
                .map(|(splice, p)| (*splice, p.kind.clone()))
                .collect::<Vec<_>>(),
            [
                (false, PatternKind::Var("a".into())),
                (true, PatternKind::Var("rest".into()))
            ]
        );
        let mut templates = Vec::new();
        assert!(lower_expression_quotes(&mut m, 0, &mut templates).is_empty());
        let ItemKind::Bind(binding) = &m.items[0].kind else {
            panic!()
        };
        let ExprKind::Chain(fs) = &binding.value.kind else {
            panic!()
        };
        let PatternKind::Quote(quote) = &fs[0].params[0].choices[0].kind else {
            panic!()
        };
        assert_eq!(quote.template, Some(0));
        assert_eq!(templates.len(), 1);
        assert_eq!(m.imports.len(), 1);
    }

    #[test]
    fn quote_shapes_are_checked() {
        let quote = |source: &str| {
            let document = crate::lisp::parse3(source).unwrap();
            document.datums.into_iter().next().unwrap()
        };
        assert!(read_pattern(&quote("`(a ~..x b)")).is_err());
        assert!(read_pattern(&quote("`(a ~..x)")).is_ok());
        assert!(check_expression(&quote("`(a ~..x b)")).is_ok());
        assert!(check_expression(&quote("`~..x")).is_err());
    }
}
