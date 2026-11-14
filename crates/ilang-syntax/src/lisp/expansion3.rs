//! `expansion::definitions` on the tree of syntax three: the written
//! interface of the syntax macros of a module.
//!
//! The answers are those of `expansion::definitions` on the lowered tree
//! (`syn3::lower`), form by form:
//! - an attribute with brackets is "given with arguments";
//! - the type parameters of a binding are the `<A>` lists of its chain and
//!   of its function type;
//! - a chain is a function expression when it is one plain function (no
//!   guard, one choice per parameter), or when it is not a clause block
//!   (several positions, or a written need or result). In the second case
//!   the lowering names the parameters itself, so they take no label.
use super::expansion::{MacroDefinition, MacroVariant};
use super::syntax::SyntaxError;
use crate::syn3::ast::{
    Attribute, Binding, ExprKind, Function, Item, ItemKind, Module, PatternKind, TItem, Type,
    TypeKind,
};
use crate::syn3::walk::{self, Visitor};

fn is_macro(a: &Attribute) -> bool {
    a.module.is_none() && a.name.name == "macro"
}

/// The type parameters that the lowering puts on a binding.
pub fn has_type_parameters(binding: &Binding) -> bool {
    let on_chain = matches!(&binding.value.kind, ExprKind::Chain(fs) if matches!(fs.as_slice(), [f] if plain(f) && !f.generics.is_empty()));
    let on_type = matches!(&binding.pattern.kind, PatternKind::Typed { ty, .. }
        if matches!(&ty.kind, TypeKind::Fn(f) if !f.generics.is_empty()));
    on_chain || on_type
}

/// A function with no guard and one choice per parameter: with no other
/// branch it is a plain function (`Fn` of the old tree).
pub fn plain(f: &Function) -> bool {
    f.guard.is_none() && f.params.iter().all(|p| p.choices.len() == 1)
}

/// A chain that is a clause block and not a function (`Clauses` of the old
/// tree): not one plain function, one position, no written need or result.
pub fn clause_block(fs: &[Function]) -> bool {
    !matches!(fs, [f] if plain(f))
        && fs.first().is_some_and(|f| f.params.len() == 1)
        && fs.iter().all(|f| f.services.is_none() && f.ret.is_none())
}

fn has_hole(ty: &Type) -> bool {
    struct Holes(bool);
    impl Visitor for Holes {
        fn ty(&mut self, t: &mut Type, _: bool) {
            self.0 |= matches!(t.kind, TypeKind::Hole);
        }
    }
    let mut holes = Holes(false);
    walk::ty(&mut ty.clone(), &mut holes);
    holes.0
}

/// Find and validate source macro definitions. The type checker subsequently
/// verifies the Syntax argument, result, error, and environment types.
pub fn definitions(module: &Module, lisp: bool) -> Result<Vec<MacroDefinition>, Vec<SyntaxError>> {
    let mut found = Vec::new();
    let mut errors = Vec::new();
    let has_macro = |item: &Item| item.attributes.iter().any(is_macro);
    for (index, item) in module.items.iter().enumerate() {
        let attributes = item
            .attributes
            .iter()
            .filter(|a| is_macro(a))
            .collect::<Vec<_>>();
        if attributes.is_empty() {
            continue;
        }
        let attr = attributes[0];
        let error = |message: &str, span| SyntaxError {
            message: message.into(),
            span,
        };
        if !lisp {
            errors.push(error(
                "a syntax macro must be defined in a .is file",
                attr.span.0,
            ));
            continue;
        }
        if attributes.len() != 1 || attr.args.is_some() {
            errors.push(error("@macro is given once without arguments", attr.span.0));
            continue;
        }
        if let ItemKind::Decl { name, .. } = &item.kind {
            let implemented = module.items.iter().any(|other| {
                matches!(&other.kind, ItemKind::Bind(b)
                    if matches!(&b.pattern.kind, PatternKind::Var(n) if n == &name.name))
                    && has_macro(other)
            });
            if !implemented {
                errors.push(error(
                    "a typed macro signature needs an @macro implementation",
                    item.span.0,
                ));
            }
            continue;
        }
        let ItemKind::Bind(binding) = &item.kind else {
            errors.push(error("@macro needs a binding", item.span.0));
            continue;
        };
        let name = match &binding.pattern.kind {
            PatternKind::Var(name) => name,
            PatternKind::Typed { binder, .. } if !binder.discard => &binder.name,
            _ => {
                errors.push(error(
                    "a macro binding needs one name",
                    binding.pattern.span.0,
                ));
                continue;
            }
        };
        if has_type_parameters(binding) {
            errors.push(error(
                "a macro binding cannot declare type variables",
                binding.pattern.span.0,
            ));
            continue;
        }
        let functions = match &binding.value.kind {
            ExprKind::Chain(fs) => fs,
            _ => {
                errors.push(error(
                    "a macro binding needs a function expression or shape branches",
                    binding.value.span.0,
                ));
                continue;
            }
        };
        let mut parameters = Vec::new();
        let mut labels = Vec::new();
        let mut variants = Vec::new();
        let mut valid = true;
        if let [function] = functions.as_slice()
            && plain(function)
        {
            let mut params: Vec<_> = function.params.iter().collect();
            params.sort_by(|a, b| {
                a.key
                    .as_ref()
                    .map(|k| &k.name)
                    .cmp(&b.key.as_ref().map(|k| &k.name))
            });
            for parameter in params {
                let p = &parameter.choices[0];
                match &p.kind {
                    PatternKind::Var(name) => parameters.push(name.clone()),
                    PatternKind::Typed { binder, .. } if !binder.discard => {
                        parameters.push(binder.name.clone())
                    }
                    _ => {
                        errors.push(error(
                            "a macro parameter must be a name, optionally with an explicit label",
                            p.span.0,
                        ));
                        valid = false;
                        continue;
                    }
                }
                let label = parameter.key.as_ref().map(|k| k.name.clone());
                if label
                    .as_ref()
                    .is_some_and(|l| labels.contains(&Some(l.clone())))
                {
                    errors.push(error("a macro parameter label is repeated", p.span.0));
                    valid = false;
                }
                labels.push(label);
            }
        } else {
            let mut shapes: Vec<Vec<Option<String>>> = Vec::new();
            for function in functions {
                let shape = function_shape(function);
                if !shapes.contains(&shape) {
                    shapes.push(shape);
                }
            }
            for (i, shape) in shapes.into_iter().enumerate() {
                variants.push(MacroVariant {
                    target: format!("{name}%g{i}"),
                    parameters: (0..shape.len()).map(|j| format!("argument{j}")).collect(),
                    labels: shape,
                });
            }
            if let Some(first) = variants.first() {
                parameters = first.parameters.clone();
                labels = first.labels.clone();
            }
            variants.sort_by(|a, b| a.labels.cmp(&b.labels));
        }
        let signature = module.items.iter().position(
            |other| matches!(&other.kind, ItemKind::Decl { name: declared, .. } if declared.name == *name),
        );
        if let Some(index) = signature {
            let declaration = &module.items[index];
            if declaration
                .attributes
                .iter()
                .any(|a| a.module.is_none() && matches!(a.name.name.as_str(), "external" | "js"))
            {
                errors.push(error(
                    "a typed macro signature cannot have @external or @js",
                    declaration.span.0,
                ));
                valid = false;
            }
            if !has_macro(declaration) {
                errors.push(error(
                    "a typed macro signature needs @macro",
                    declaration.span.0,
                ));
                valid = false;
            }
            if let ItemKind::Decl { ty, .. } = &declaration.kind {
                if has_hole(ty) {
                    errors.push(error(
                        "a typed macro signature must be fully written without _",
                        declaration.span.0,
                    ));
                    valid = false;
                }
                match &ty.kind {
                    TypeKind::Fn(f) => {
                        if let Some(mut advertised) = signature_shapes(&f.param) {
                            let mut implemented = if variants.is_empty() {
                                vec![labels.clone()]
                            } else {
                                variants.iter().map(|v| v.labels.clone()).collect()
                            };
                            advertised.sort();
                            advertised.dedup();
                            implemented.sort();
                            implemented.dedup();
                            if !advertised.iter().all(|shape| implemented.contains(shape)) {
                                errors.push(error("a typed macro implementation must have one parameter for each signature parameter in every parameter-data shape, with matching explicit labels",item.span.0));
                                valid = false;
                            }
                        }
                    }
                    _ => {
                        errors.push(error(
                            "a typed macro signature must be a full function type",
                            declaration.span.0,
                        ));
                        valid = false;
                    }
                }
            }
        }
        if valid {
            found.push(MacroDefinition {
                item: index,
                name: name.clone(),
                parameters,
                labels,
                variants,
                signature,
            });
        }
    }
    if errors.is_empty() {
        Ok(found)
    } else {
        Err(errors)
    }
}

/// Canonical ordinary call shape, preserving explicit labels only.
pub fn function_shape(f: &Function) -> Vec<Option<String>> {
    let mut labels: Vec<_> = f
        .params
        .iter()
        .map(|p| p.key.as_ref().map(|k| k.name.clone()))
        .collect();
    labels.sort();
    labels
}
fn signature_shapes(ty: &Type) -> Option<Vec<Vec<Option<String>>>> {
    // Mirror main's signature projection: a directly written parameter data
    // lists its parts; data inside a union uses its lowered record/tuple view.
    // Unsupported mixed/spread data keeps the lowering diagnostics, without
    // gaining a second, different macro-shape diagnostic here.
    fn nested(ty: &Type) -> Option<Vec<Vec<Option<String>>>> {
        Some(match &ty.kind {
            TypeKind::Data(items) => {
                let mut labels =
                    if !items.is_empty() && items.iter().all(|i| matches!(i, TItem::Pos(_))) {
                        vec![None; items.len()]
                    } else {
                        items
                            .iter()
                            .filter_map(|i| match i {
                                TItem::Label(k, _) => Some(Some(k.name.clone())),
                                _ => None,
                            })
                            .collect()
                    };
                labels.sort();
                vec![labels]
            }
            TypeKind::Union(xs) => {
                let mut out = Vec::new();
                for x in xs {
                    out.extend(nested(x)?);
                }
                out
            }
            _ => return None,
        })
    }
    if let TypeKind::Data(items) = &ty.kind
        && (items.is_empty() || items.iter().any(|i| !matches!(i, TItem::Label(..))))
    {
        let mut labels: Vec<_> = items
            .iter()
            .filter_map(|i| match i {
                TItem::Pos(_) => Some(None),
                TItem::Label(k, _) => Some(Some(k.name.clone())),
                TItem::Spread(_) => None,
            })
            .collect();
        labels.sort();
        Some(vec![labels])
    } else {
        nested(ty)
    }
}

#[cfg(test)]
mod tests {
    use migration_syntax::lisp::expansion::MacroDefinition;
    use migration_syntax::lisp::expansion3::*;
    use migration_syntax::lisp::syntax::SyntaxError;

    /// The answer on the tree of syntax three and the answer on its
    /// lowering are the same.
    fn both(source: &str, lisp: bool) -> Result<Vec<MacroDefinition>, Vec<SyntaxError>> {
        let (module, errors) = if lisp {
            migration_syntax::syn3::lisp::from_lisp(source)
        } else {
            migration_syntax::syn3::parse_module(source)
        };
        assert!(errors.is_empty(), "{errors:?}");
        let (lowered, _) = if lisp {
            migration_syntax::syn3::lower::lower_lisp(&module)
        } else {
            migration_syntax::syn3::lower::lower(&module)
        };
        let new = definitions(&module, lisp);
        let old = migration_syntax::lisp::expansion::definitions(&lowered, lisp);
        match (&new, &old) {
            (Ok(a), Ok(b)) => {
                assert_eq!(a.len(), b.len(), "{source}");
                for (a, b) in a.iter().zip(b) {
                    assert_eq!(
                        (
                            module.items[a.item].span.0,
                            &a.name,
                            a.signature.map(|i| module.items[i].span.0)
                        ),
                        (
                            lowered.items[b.item].span,
                            &b.name,
                            b.signature.map(|i| lowered.items[i].span)
                        )
                    );
                    assert_eq!(a.parameters.len(), b.parameters.len(), "{source}");
                    if !b.parameters.iter().any(|p| p.starts_with('%')) {
                        assert_eq!(a.parameters, b.parameters, "{source}");
                    }
                }
            }
            (Err(a), Err(b)) => {
                let show = |es: &[SyntaxError]| {
                    es.iter()
                        .map(|e| (e.message.clone(), e.span))
                        .collect::<Vec<_>>()
                };
                assert_eq!(show(a), show(b), "{source}");
            }
            _ => panic!("{source}: new {new:?}, old {old:?}"),
        }
        new
    }

    #[test]
    fn definitions_agree_with_the_lowered_tree() {
        let cases = [
            "@macro\nm = (a, b) { a }",
            "@macro\nm = (a: Syntax, _b) { a }",
            "@macro\nm = (0) { 1 } | (a) { a }",
            "@macro\nm = (a, 0) { a } | (a, b) { b }",
            "@macro\nm = (a) -> Syntax { a } | (b) { b }",
            "@macro\nm = <A>(a) { a }",
            "@macro\nm = <A>(a : A) { a } | (b, c) { b }",
            "@macro\nm : { (Int, x = Int) | (Int) } -> Int\n@macro\nm = (x = a) { a } | (b) { b }",
            "@macro\nm = 1",
            "@macro\n(a, b) = (1, 2)",
            "@macro(\"x\")\nm = (a) { a }",
            "@macro()\nm = (a) { a }",
            "@macro\n@macro\nm = (a) { a }",
            "@macro\nm: (Syntax) -> Syntax\n@macro\nm = (a) { a }",
            "@macro\nm: (Syntax, Syntax) -> Syntax\n@macro\nm = (a) { a }",
            "@macro\nm: (_) -> Syntax\n@macro\nm = (a) { a }",
            "m: (Syntax) -> Syntax\n@macro\nm = (a) { a }",
            "@macro\nm: (Syntax) -> Syntax",
            "@macro\nm: Syntax\n@macro\nm = (a) { a }",
            "@macro\nT = (Int)",
            "@macro\nm = (a) <- (s: S) { a }",
        ];
        for case in cases {
            for lisp in [false, true] {
                if lisp {
                    // The same module in the Lisp surface.
                    let (module, errors) = migration_syntax::syn3::parse_module(case);
                    assert!(errors.is_empty(), "{case}: {errors:?}");
                    let text = migration_syntax::syn3::lisp::to_lisp(&module);
                    both(&text, true).ok();
                } else {
                    assert!(both(case, false).is_err(), "{case}");
                }
            }
        }
    }

    #[test]
    fn a_plain_macro_is_found_with_its_parameters() {
        let found = both("(@ macro (= m (-> (, a b) _ a)))", true);
        if let Ok(found) = found {
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].parameters, ["a", "b"]);
        } else {
            panic!("{found:?}");
        }
    }
}
