//! Syntax three (docs/design/syntax-3.md), beside the current parser: its
//! tree, parser, printer and formatter, Lisp surface, and the converter from
//! the current tree.

pub mod ast;
mod parse;
pub(crate) mod print;

pub use parse::{parse_expr, parse_module, parse_pattern, parse_type, pipe_target};
pub use print::{
    MAX_CALL_DEPTH, WIDTH, call_depth, format, print_expr, print_module, print_pattern, print_type,
    print_with_comments, record_label, string_literal,
};
pub mod convert;
pub mod deep;
pub mod facts;
pub mod group;
pub mod lisp;
pub mod lower;
pub mod surface;
pub mod walk;

/// Reads a normal-surface file of syntax three into the current tree.
pub fn parse_lowered(source: &str) -> (crate::ast::Module, Vec<crate::Diagnostic>) {
    let (module, mut diags) = parse_module(source);
    let (lowered, more) = lower::lower(&module);
    diags.extend(more);
    diags.sort_by_key(|d| (d.span.start, d.span.end));
    (lowered, diags)
}

/// Reads a Lisp-surface file of syntax three into the current tree.
pub fn parse_lisp_lowered(source: &str) -> (crate::ast::Module, Vec<crate::Diagnostic>) {
    let (module, mut diags) = lisp::from_lisp(source);
    let (lowered, more) = lower::lower_lisp(&module);
    diags.extend(more);
    diags.sort_by_key(|d| (d.span.start, d.span.end));
    (lowered, diags)
}

/// Prints a current tree (an expanded module) as syntax three: the normal
/// surface, or the Lisp surface with `lisp`.
pub fn print_current(module: &crate::ast::Module, lisp: bool) -> String {
    let (module, _) = convert::convert(module, None, &convert::Signatures::default());
    if lisp {
        self::lisp::to_lisp(&module)
    } else {
        print_module(&module)
    }
}

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
