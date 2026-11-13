//! Old node assertions over the production native parser and validation.
#![allow(dead_code)]
#[allow(unused_imports)]
pub use migration_syntax::*;
extern crate ilang_syntax as native;

pub fn parse_module(source: &str) -> (ast::Module, Vec<Diagnostic>) {
    let (module, mut errors) = native::parse_module(source);
    let (old, old_errors) = migration_syntax::syn3::lower::lower(&module);
    let (_, validation) = native::syn3::facts::Transition::capture_module(&module, false);
    let records = |ds: &[Diagnostic]| ds.iter().map(|d| (d.code, d.span, d.message.clone())).collect::<Vec<_>>();
    assert_eq!(records(&validation), records(&old_errors), "native validation of {source:?}");
    errors.sort_by_key(|d| (d.span.start, d.span.end));
    (old, errors)
}
