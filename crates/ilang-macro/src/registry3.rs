//! The part of `registry` that reads the tree, on the tree of syntax
//! three: the value names that a module defines. The macro definitions are
//! `ilang_syntax::lisp::expansion3::definitions`; imports and exports have
//! the same fields in both trees.
use ilang_syntax::span::Span;
use ilang_syntax::syn3::ast::{ItemKind, Module};

/// The names of the value namespace that the items of a module define,
/// with the span of each binder: these may shadow a macro or a module alias.
pub fn value_names(module: &Module) -> Vec<(&str, Span)> {
    let mut names = Vec::new();
    for item in &module.items {
        match &item.kind {
            ItemKind::Bind(binding) => {
                crate::body_walk3::for_each_binder(&binding.pattern, |name, span| {
                    names.push((name, span))
                })
            }
            ItemKind::Decl { name, .. } => names.push((name.name.as_str(), name.span.0)),
            _ => {}
        }
    }
    names
}

/// Whether the module exports the value or type `name`.
pub fn exported(module: &Module, name: &str) -> bool {
    module.exports.iter().any(|e| e.name.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_names_are_those_of_the_lowered_tree() {
        let source = "T = (Int)\nx: Int\n(a, [b, ..c]) = f()\nC(k = d) = g()\ne: Int = 1";
        let (module, errors) = ilang_syntax::syn3::parse_module(source);
        assert!(errors.is_empty(), "{errors:?}");
        let (lowered, errors) = ilang_syntax::syn3::lower::lower(&module);
        assert!(errors.is_empty(), "{errors:?}");
        let mut old = Vec::new();
        for item in &lowered.items {
            match &item.kind {
                ilang_syntax::ast::ItemKind::Binding(binding) => {
                    crate::body_walk::for_each_binder(&binding.pattern, |name, span| {
                        old.push((name, span))
                    })
                }
                ilang_syntax::ast::ItemKind::Declaration { name, .. } => {
                    old.push((name.name.as_str(), name.span))
                }
                _ => {}
            }
        }
        assert_eq!(value_names(&module), old);
        assert_eq!(
            old.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            ["x", "a", "b", "c", "d", "k", "e"]
        );
    }
}
