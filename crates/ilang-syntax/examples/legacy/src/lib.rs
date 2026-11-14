//! Private old syntax for migration tools; never linked into the compiler.
pub use syntax3::{Code, Diagnostic, format, lex, old_syntax_hint, span, token};
pub mod ast;
mod ast_clone;
mod ast_eq;
pub mod legacy;
pub mod lexer;
pub mod lisp;
pub mod macro_body_walk;
pub mod macro_call;
pub mod print;
pub mod shape;
pub mod syn3;
pub mod walk;
pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let (tree, mut errors) = syn3::parse_module(source);
    let (tree, more) = syn3::lower::lower(&tree);
    errors.extend(more);
    errors.sort_by_key(|d| (d.span.start, d.span.end));
    (tree, errors)
}
