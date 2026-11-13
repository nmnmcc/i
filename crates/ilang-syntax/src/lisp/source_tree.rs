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
        if !enter || self.kind != Kind::Expr || self.found.is_some() {
            return;
        }
        let transition_root = matches!(&e.kind, crate::syn3::ast::ExprKind::Chain(fs)
            if fs.iter().any(|f| f.params.iter().any(|p| p.key.is_some())
                || fs.first().is_some_and(|first| first.params.len() != f.params.len())));
        if e.span.0 == self.span
            || transition_root
                && self.span == Span::new(e.span.0.end.saturating_sub(1), e.span.0.end)
        {
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
        Self::read(scope, source, false)
    }

    pub fn read(scope: &Scope, source: &str, lisp: bool) -> SourceTree {
        let (module, _) = crate::syn3::deep::deep(|| {
            if lisp {
                crate::syn3::lisp::from_lisp(source)
            } else {
                crate::syn3::parse_module(source)
            }
        });
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
        let module: &mut Module = &mut module;
        crate::syn3::deep::deep(|| {
            with_tokens(&self.tokens, || walk::module(module, &mut find));
        });
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
        Some(crate::syn3::deep::deep(|| {
            with_tokens(&self.tokens, || item_datum(item))
        }))
    }

    /// The written item with this span.
    pub fn item(&self, span: Span) -> Option<Syntax> {
        self.item_datum(span)
            .map(|d| Syntax::from_datum(&d, &self.scope))
    }

    /// A written type definition, including a member absorbed into a sum
    /// by the transition lowering.
    pub fn definition(&self, name: &str) -> Option<(Item, Datum)> {
        let module = self.module.lock().expect("the source tree is not poisoned");
        let item = module.items.iter().find(|item| {
            matches!(&item.kind, crate::syn3::ast::ItemKind::TypeDef(def) if def.name.name == name)
        })?;
        Some(crate::syn3::deep::deep(|| {
            (item.clone(), with_tokens(&self.tokens, || item_datum(item)))
        }))
    }

    /// Visibility before the transition lowering absorbs sum members and
    /// their exports into the sum's old-tree item.
    pub fn definition_visibility(&self, name: &str) -> Option<(bool, bool)> {
        use crate::syn3::ast::{ItemKind, TypeKind, TypeRhs};
        let module = self.module.lock().expect("the source tree is not poisoned");
        let definition = module.items.iter().find_map(|item| match &item.kind {
            ItemKind::TypeDef(def) if def.name.name == name => Some(def),
            _ => None,
        })?;
        let mut exported = false;
        let mut constructors = false;
        for export in &module.exports {
            if export.name.name == name {
                exported = true;
                constructors |= export.constructors;
            } else if export.constructors {
                let sum = module.items.iter().find_map(|item| match &item.kind {
                    ItemKind::TypeDef(def) if def.name.name == export.name.name => Some(def),
                    _ => None,
                });
                if let Some(def) = sum
                    && let TypeRhs::Type(ty) = &def.rhs
                    && let TypeKind::Union(members) = &ty.kind
                    && members.iter().any(|member| {
                        matches!(&member.kind,
                        TypeKind::Named { module: None, name: member, .. } if member.name == name)
                    })
                {
                    exported = true;
                    constructors = true;
                }
            }
        }
        Some((
            exported,
            constructors || matches!(definition.rhs, TypeRhs::Type(_)),
        ))
    }
}
