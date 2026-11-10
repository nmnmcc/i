//! Structural normalization for write validation and syntax-tree tests.
use crate::{
    ast::*,
    span::Span,
    walk::{Visitor, Walk},
};

struct Unspan;
impl Visitor for Unspan {
    fn span(&mut self, span: &mut Span) {
        *span = Span::default();
    }
}

/// The debug form of a module with its spans normalized structurally.
/// String literals, names and documentation are never interpreted as spans.
pub fn shape(module: &Module) -> String {
    let mut module = module.clone();
    module.walk(&mut Unspan);
    format!("{module:?}")
}

/// Remove transparent expression groups and normalize spans. All other
/// data, including every string payload, participates in AST equality.
pub(crate) fn canonical(module: &Module) -> Module {
    struct Canon;
    impl Visitor for Canon {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if !enter
                && let ExprKind::Block(b) = &e.kind
                && let [Stmt::Expr(inner)] = &b.stmts[..]
            {
                *e = inner.clone();
            }
        }
    }
    let mut module = module.clone();
    module.walk(&mut Canon);
    module.walk(&mut Unspan);
    module
}

/// The canonical debug form, for readable assertions in tests.
pub fn canon(module: &Module) -> String {
    format!("{:?}", canonical(module))
}
