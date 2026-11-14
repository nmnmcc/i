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
    let mut groups = Vec::new();
    for definition in definitions {
        let item = &mut module.items[definition.item];
        item.attributes.retain(|a| !is_macro(a));
        let ItemKind::Bind(binding) = &mut item.kind else {
            continue;
        };
        let ExprKind::Chain(functions) = &mut binding.value.kind else {
            continue;
        };
        if definition.variants.is_empty() {
            for function in functions {
                canonical(function);
            }
        } else {
            let mut values = Vec::new();
            for variant in &definition.variants {
                let mut fs: Vec<_> = functions
                    .iter()
                    .filter(|f| ilang_syntax::lisp::expansion3::function_shape(f) == variant.labels)
                    .cloned()
                    .collect();
                for f in &mut fs {
                    canonical(f);
                }
                let first = fs[0].span.0;
                let cover = first.to(fs.last().unwrap().span.0);
                let at = ilang_syntax::span::Span::new(first.start, first.start + 1);
                groups.push(Item {
                    doc: Vec::new(),
                    attributes: Vec::new(),
                    span: item.span,
                    kind: ItemKind::Bind(Binding {
                        pattern: Pattern::new(PatternKind::Var(variant.target.clone()), at),
                        value: Expr::new(ExprKind::Chain(fs), cover),
                        span: binding.span,
                    }),
                });
                let mut shape = variant
                    .labels
                    .iter()
                    .filter(|x| x.is_none())
                    .count()
                    .to_string();
                for label in variant.labels.iter().flatten() {
                    shape.push(',');
                    shape.push_str(label);
                }
                values.push(ilang_syntax::ast::DItem::Label(
                    ilang_syntax::ast::Ident::new(shape, at),
                    Expr::new(
                        ExprKind::Name {
                            module: None,
                            name: ilang_syntax::ast::Ident::new(&variant.target, at),
                        },
                        at,
                    ),
                ));
            }
            binding.value.kind = ExprKind::Call {
                callee: Box::new(Expr::new(
                    ExprKind::Name {
                        module: None,
                        name: ilang_syntax::ast::Ident::new("%multi", binding.value.span.0),
                    },
                    binding.value.span.0,
                )),
                args: values,
            };
        }
    }
    module.items.extend(groups);
    quote_lower3::lower_expression_quotes(module, module_id, templates)
}

fn canonical(function: &mut ilang_syntax::ast::Function) {
    function.params.sort_by(|a, b| {
        a.key
            .as_ref()
            .map(|k| &k.name)
            .cmp(&b.key.as_ref().map(|k| &k.name))
    });
    for param in &mut function.params {
        param.key = None;
    }
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
