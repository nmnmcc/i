//! Syntax three (docs/design/syntax-3.md), beside the current parser: its
//! tree, parser, printer and formatter, Lisp surface, and the converter from
//! the current tree.

pub mod ast;
mod parse;
mod print;

pub use parse::{parse_expr, parse_module, parse_pattern, parse_type, pipe_target};
pub use print::{
    MAX_CALL_DEPTH, WIDTH, call_depth, format, print_expr, print_module, print_pattern, print_type,
    print_with_comments,
};
pub mod convert;
pub mod lisp;
pub mod lower;

/// The switch of the transition (stage 8 to stage 12): every source file is
/// read as syntax three and lowered to the current tree, unless the
/// environment variable `I_SYNTAX=2` asks for the syntax before it. Deleted in
/// stage 12.
pub fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("I_SYNTAX").map_or(true, |v| v != "2"))
}

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

/// Rewrites a source text of the current normal surface (compiler-made
/// modules) as syntax three.
pub fn convert_source(source: &str) -> String {
    let (old, diags) = crate::parser::parse_module_legacy(source);
    assert!(diags.is_empty(), "compiler-made source must parse: {diags:?}");
    let lexed = crate::lex(source);
    let (module, _) = convert::convert(&old, Some(&lexed), &convert::Signatures::default());
    print_module(&module)
}
