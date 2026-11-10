//! The shape of a syntax tree: its debug form with the spans taken out, to
//! compare trees. Used by the tests.

use crate::ast::Module;

/// The debug form of a module without spans.
pub fn shape(module: &Module) -> String {
    let text = format!("{module:?}");
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(at) = rest.find('@') {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        let skip = tail
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(tail.len());
        if skip > 0 && tail[..skip].contains("..") {
            rest = &tail[skip..];
        } else {
            out.push('@');
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// The shape after removing expression groups, for the migration write
/// check. No binding, parameter, annotation, or clause is erased.
pub fn canon(module: &Module) -> String {
    struct Canon;
    impl crate::migrate::Visit for Canon {
        fn expr(&mut self, e: &mut crate::ast::Expr, enter: bool) {
            if !enter
                && let crate::ast::ExprKind::Block(b) = &e.kind
                && let [crate::ast::Stmt::Expr(inner)] = &b.stmts[..]
            {
                *e = inner.clone();
            }
        }
    }
    let mut module = module.clone();
    crate::migrate::visit_module(&mut module, &mut Canon);
    shape(&module)
}
