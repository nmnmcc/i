//! The written code of a normal-surface file as macro syntax: the node of
//! the syntax-three tree at a span, as a datum of the §6.8 rows with the
//! spans of the written names and operators.
use super::Datum;
use super::syntax::{Scope, Syntax};
use crate::span::Span;
use crate::syn3::ast::{Expr, Item, Module, Pattern, Type};
use crate::syn3::lisp::{
    TokenIndex, expr_datum, item_datum, pattern_datum, type_datum, with_tokens,
};
use crate::syn3::walk::{self, Visitor};
use std::sync::{Arc, Mutex};

/// One source file, read once. Lookups are by the span of a node, which the
/// lowered tree keeps.
pub struct SourceTree {
    scope: Scope,
    // The walker takes the tree mutably; it does not change it.
    module: Mutex<Module>,
    tokens: Arc<TokenIndex>,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Expr,
    Pattern,
    Type,
}

struct Find {
    span: Span,
    kind: Kind,
    found: Option<Datum>,
}

impl Visitor for Find {
    fn expr(&mut self, e: &mut Expr, enter: bool) {
        if enter && self.kind == Kind::Expr && self.found.is_none() && e.span.0 == self.span {
            self.found = Some(expr_datum(e));
        }
    }
    fn pattern(&mut self, p: &mut Pattern, enter: bool) {
        if enter && self.kind == Kind::Pattern && self.found.is_none() && p.span.0 == self.span {
            self.found = Some(pattern_datum(p));
        }
    }
    fn ty(&mut self, t: &mut Type, enter: bool) {
        if enter && self.kind == Kind::Type && self.found.is_none() && t.span.0 == self.span {
            self.found = Some(type_datum(t));
        }
    }
}

impl SourceTree {
    pub fn new(scope: &Scope, source: &str) -> SourceTree {
        let (module, _) = crate::syn3::parse_module(source);
        SourceTree {
            scope: scope.clone(),
            module: Mutex::new(module),
            tokens: Arc::new(TokenIndex::new(source)),
        }
    }

    fn find(&self, span: Span, kind: Kind) -> Option<Syntax> {
        let mut module = self.module.lock().expect("the source tree is not poisoned");
        let mut find = Find {
            span,
            kind,
            found: None,
        };
        with_tokens(&self.tokens, || walk::module(&mut module, &mut find));
        find.found.map(|d| Syntax::from_datum(&d, &self.scope))
    }

    /// The outermost written expression with this span.
    pub fn expression(&self, span: Span) -> Option<Syntax> {
        self.find(span, Kind::Expr)
    }

    /// The outermost written pattern with this span.
    pub fn pattern(&self, span: Span) -> Option<Syntax> {
        self.find(span, Kind::Pattern)
    }

    /// The outermost written type with this span.
    pub fn ty(&self, span: Span) -> Option<Syntax> {
        self.find(span, Kind::Type)
    }

    /// The written item with this span, as a datum (for its charge).
    pub fn item_datum(&self, span: Span) -> Option<Datum> {
        let module = self.module.lock().expect("the source tree is not poisoned");
        let item: &Item = module.items.iter().find(|i| i.span.0 == span)?;
        Some(with_tokens(&self.tokens, || item_datum(item)))
    }

    /// The written item with this span.
    pub fn item(&self, span: Span) -> Option<Syntax> {
        self.item_datum(span)
            .map(|d| Syntax::from_datum(&d, &self.scope))
    }
}
