//! The parts of `prepare` and `project` that read or write the tree, on
//! the tree of syntax three.
use ilang_syntax::lisp::{expansion::MacroDefinition, quote_lower3, syntax::Syntax};
use ilang_syntax::syn3::ast::{
    Attribute, Binding, Expr, ExprKind, Item, ItemKind, Module, Pattern, PatternKind,
};

fn is_macro(attribute: &Attribute) -> bool {
    attribute.module.is_none() && attribute.name.name == "macro"
}

/// Whether a module defines a macro (an item with `@macro`).
pub fn defines_macro(module: &Module) -> bool {
    module
        .items
        .iter()
        .any(|item| item.attributes.iter().any(is_macro))
}

/// `_ = 0` in the place of an item, with its span.
pub fn placeholder(item: &Item) -> ItemKind {
    let span = item.span.0;
    ItemKind::Bind(Binding {
        pattern: Pattern::new(PatternKind::Discard("_".into()), span),
        value: Expr::new(ExprKind::Int(0u32.into()), span),
        span: span.into(),
    })
}

/// Makes a module ready for the compiler program: the runtime signature of
/// a typed macro is not part of it, the definitions lose their phase
/// marker, and the quotes are lowered. The quotes that do not read are
/// returned.
pub fn compiler_module(
    module: &mut Module,
    module_id: u32,
    definitions: &[MacroDefinition],
    templates: &mut Vec<Syntax>,
) -> Vec<ilang_syntax::Diagnostic> {
    for definition in definitions {
        if let Some(index) = definition.signature {
            module.items[index].attributes.clear();
            module.items[index].kind = placeholder(&module.items[index]);
        }
    }
    for definition in definitions {
        module.items[definition.item]
            .attributes
            .retain(|a| !is_macro(a));
    }
    quote_lower3::lower_expression_quotes(module, module_id, templates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_compiler_module_has_no_marker_and_no_signature() {
        let source = "(@ macro (: m (-> (, Syntax) Syntax)))\n(@ macro (= m (-> (, a) _ a)))";
        let (mut module, errors) = ilang_syntax::syn3::lisp::from_lisp(source);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(defines_macro(&module));
        let definitions = ilang_syntax::lisp::expansion3::definitions(&module, true).unwrap();
        let mut templates = Vec::new();
        let errors = compiler_module(&mut module, 0, &definitions, &mut templates);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(!defines_macro(&module));
        assert!(matches!(&module.items[0].kind, ItemKind::Bind(b)
            if matches!(b.pattern.kind, PatternKind::Discard(_))));
        assert!(templates.is_empty());
    }
}
