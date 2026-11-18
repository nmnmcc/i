pub use syntax3::parse::{
    ONE_BINDING_HINT, binding_line_traps, parse_expr, parse_module, parse_pattern, parse_type,
    pipe_target,
};
pub use syntax3::print::{
    MAX_CALL_DEPTH, WIDTH, call_depth, format, print_annotation, print_expr, print_module,
    print_pattern, print_type, print_with_comments, record_label, string_literal,
};
pub use syntax3::{ast, deep, expression_module, facts, group, print, print_tree, surface, walk};
pub mod convert;
pub mod lisp;
pub mod lower;
/// Reads a normal-surface file of the source syntax into the current tree.
pub fn parse_lowered(source: &str) -> (crate::ast::Module, Vec<crate::Diagnostic>) {
    let (module, mut diags) = parse_module(source);
    let (lowered, more) = lower::lower(&module);
    diags.extend(more);
    diags.sort_by_key(|d| (d.span.start, d.span.end));
    (lowered, diags)
}

/// Reads a Lisp-surface file of the source syntax into the current tree.
pub fn parse_lisp_lowered(source: &str) -> (crate::ast::Module, Vec<crate::Diagnostic>) {
    let (module, mut diags) = lisp::from_lisp(source);
    let (lowered, more) = lower::lower_lisp(&module);
    diags.extend(more);
    diags.sort_by_key(|d| (d.span.start, d.span.end));
    (lowered, diags)
}

/// Prints a current tree (an expanded module) in the source syntax: the normal
/// surface, or the Lisp surface with `lisp`.
pub fn print_current(module: &crate::ast::Module, lisp: bool) -> String {
    let (module, _) = convert::convert(module, None, &convert::Signatures::default());
    if lisp {
        self::lisp::to_lisp(&module)
    } else {
        print_module(&module)
    }
}
