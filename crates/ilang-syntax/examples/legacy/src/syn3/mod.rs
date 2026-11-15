pub use syntax3::syn3::*;
pub mod convert;
pub mod lisp;
pub mod lower;
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
