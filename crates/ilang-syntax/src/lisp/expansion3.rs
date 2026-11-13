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
use super::expansion::MacroDefinition;
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
    let on_chain = matches!(&binding.value.kind, ExprKind::Chain(fs) if fs.iter().any(|f| !f.generics.is_empty()));
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
            ExprKind::Chain(fs) if !clause_block(fs) => fs,
            _ => {
                errors.push(error(
                    "a macro binding needs a function expression",
                    binding.value.span.0,
                ));
                continue;
            }
        };
        let mut parameters = Vec::new();
        let mut valid = true;
        match functions.as_slice() {
            [function] if plain(function) => {
                for parameter in &function.params {
                    let p = &parameter.choices[0];
                    match &p.kind {
                        PatternKind::Var(name) => parameters.push(name.clone()),
                        PatternKind::Typed { binder, .. } if !binder.discard => {
                            parameters.push(binder.name.clone())
                        }
                        _ => {
                            errors.push(error("a macro parameter must be a name", p.span.0));
                            valid = false;
                        }
                    }
                }
            }
            // The lowering makes a function of fresh parameters over a
            // match. Their names cannot be written, so no label names them.
            _ => parameters.extend(
                (0..functions.first().map_or(0, |f| f.params.len()))
                    .map(|i| format!("%arg{}", i + 1)),
            ),
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
                        // A parameter that is not written as data is an
                        // error of the lowering, which leaves no parameter.
                        let count = match &f.param.kind {
                            TypeKind::Data(items) => items
                                .iter()
                                .filter(|i| !matches!(i, TItem::Spread(_)))
                                .count(),
                            _ => 0,
                        };
                        if count != parameters.len() {
                            errors.push(error("a typed macro implementation must have one parameter for each signature parameter", item.span.0));
                            valid = false;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The answer on the tree of syntax three and the answer on its
    /// lowering are the same.
    fn both(source: &str, lisp: bool) -> Result<Vec<MacroDefinition>, Vec<SyntaxError>> {
        let (module, errors) = if lisp {
            crate::syn3::lisp::from_lisp(source)
        } else {
            crate::syn3::parse_module(source)
        };
        assert!(errors.is_empty(), "{errors:?}");
        let (lowered, _) = if lisp {
            crate::syn3::lower::lower_lisp(&module)
        } else {
            crate::syn3::lower::lower(&module)
        };
        let new = definitions(&module, lisp);
        let old = super::super::expansion::definitions(&lowered, lisp);
        match (&new, &old) {
            (Ok(a), Ok(b)) => {
                assert_eq!(a.len(), b.len(), "{source}");
                for (a, b) in a.iter().zip(b) {
                    assert_eq!(
                        (a.item, &a.name, a.signature),
                        (b.item, &b.name, b.signature)
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
                    let (module, errors) = crate::syn3::parse_module(case);
                    assert!(errors.is_empty(), "{case}: {errors:?}");
                    let text = crate::syn3::lisp::to_lisp(&module);
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
