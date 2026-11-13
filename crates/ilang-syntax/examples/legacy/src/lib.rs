//! Private old syntax for migration tools; never linked into the compiler.
pub use syntax3::{Code, Diagnostic, span, token, format, old_syntax_hint, lex};
pub mod macro_body_walk;
pub mod macro_call;
pub mod lexer;
pub mod ast;
mod ast_clone;
mod ast_eq;
pub mod walk;
pub mod print;
pub mod shape;
pub mod legacy;
pub mod lisp;
pub mod syn3;
pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let (tree, mut errors) = syn3::parse_module(source);
    let (tree, more) = syn3::lower::lower(&tree);
    errors.extend(more); errors.sort_by_key(|d| (d.span.start, d.span.end)); (tree, errors)
}
