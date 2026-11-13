//! Compatibility namespace for the syntax-three tree now at the crate root.
pub use crate::parse::{parse_expr, parse_module, parse_pattern, parse_type, pipe_target};
pub use crate::print::{
    MAX_CALL_DEPTH, WIDTH, call_depth, format, print_expr, print_module, print_pattern, print_type,
    print_with_comments, record_label, string_literal,
};
pub use crate::{ast, deep, facts, group, print, rows as lisp, surface, walk};
/// Temporary integration switch for the remaining non-engine tree readers.
/// Keep off until the rows, macro engine and resolver have moved together.
pub const FINAL_AST_READERS: bool = true;

/// Wraps an expanded expression for the module printers used by editor tools.
pub fn expression_module(value: ast::Expr) -> ast::Module {
    let span = value.span;
    ast::Module {
        items: vec![ast::Item {
            doc: Vec::new(),
            attributes: Vec::new(),
            span,
            kind: ast::ItemKind::Bind(ast::Binding {
                pattern: ast::Pattern {
                    kind: ast::PatternKind::Discard("_".into()),
                    span,
                },
                value,
                span,
            }),
        }],
        ..ast::Module::default()
    }
}

/// Prints an expanded syntax-three module in either surface.
pub fn print_tree(module: &ast::Module, lisp: bool) -> String {
    deep::deep(|| {
        if lisp {
            self::lisp::to_lisp(module)
        } else {
            let mut module = module.clone();
            group::regroup(&mut module);
            print_module(&module)
        }
    })
}
