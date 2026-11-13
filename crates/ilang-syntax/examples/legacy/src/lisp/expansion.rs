use super::syntax::SyntaxError;
pub use syntax3::lisp::expansion::{MacroDefinition, MacroArgument, ExpansionSession};
pub fn definitions(
    module: &crate::ast::Module,
    lisp: bool,
) -> Result<Vec<MacroDefinition>, Vec<SyntaxError>> {
    use crate::ast::{ExprKind, ItemKind, Param, PatternKind};
    let mut found = Vec::new();
    let mut errors = Vec::new();
    for (index, item) in module.items.iter().enumerate() {
        let attributes = item
            .attributes
            .iter()
            .filter(|a| a.module.is_none() && a.name.name == "macro")
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
                attr.span,
            ));
            continue;
        }
        if attributes.len() != 1 || !attr.args.is_empty() || attr.call.is_some() {
            errors.push(error("@macro is given once without arguments", attr.span));
            continue;
        }
        if let ItemKind::Declaration { name, .. } = &item.kind {
            if !module.items.iter().any(|other| matches!(&other.kind, ItemKind::Binding(b) if matches!(&b.pattern.kind, PatternKind::Var(n) if n == &name.name)) && other.attributes.iter().any(|a| a.module.is_none() && a.name.name == "macro")) {
                errors.push(error("a typed macro signature needs an @macro implementation", item.span));
            }
            continue;
        }
        let ItemKind::Binding(binding) = &item.kind else {
            errors.push(error("@macro needs a binding", item.span));
            continue;
        };
        let name = match &binding.pattern.kind {
            PatternKind::Var(name) => name,
            PatternKind::Typed { binder, .. } if !binder.discard => &binder.name,
            _ => {
                errors.push(error(
                    "a macro binding needs one name",
                    binding.pattern.span,
                ));
                continue;
            }
        };
        if !binding.type_params.is_empty() {
            errors.push(error(
                "a macro binding cannot declare type variables",
                binding.pattern.span,
            ));
            continue;
        }
        let ExprKind::Fn(function) = &binding.value.kind else {
            errors.push(error(
                "a macro binding needs a function expression",
                binding.value.span,
            ));
            continue;
        };
        let mut parameters = Vec::new();
        let mut valid = true;
        for parameter in &function.params {
            match parameter {
                Param::Env { .. } => {}
                Param::Pattern(p) => match &p.kind {
                    PatternKind::Var(name) => parameters.push(name.clone()),
                    PatternKind::Typed { binder, .. } if !binder.discard => {
                        parameters.push(binder.name.clone())
                    }
                    _ => {
                        errors.push(error("a macro parameter must be a name", p.span));
                        valid = false;
                    }
                },
            }
        }
        let signature = module.items.iter().position(|other| matches!(&other.kind, ItemKind::Declaration { name: declared, .. } if declared.name == *name));
        if let Some(index) = signature {
            let declaration = &module.items[index];
            if declaration
                .attributes
                .iter()
                .any(|a| a.module.is_none() && matches!(a.name.name.as_str(), "external" | "js"))
            {
                errors.push(error(
                    "a typed macro signature cannot have @external or @js",
                    declaration.span,
                ));
                valid = false;
            }
            if !declaration
                .attributes
                .iter()
                .any(|a| a.module.is_none() && a.name.name == "macro")
            {
                errors.push(error(
                    "a typed macro signature needs @macro",
                    declaration.span,
                ));
                valid = false;
            }
            if let ItemKind::Declaration { ty, .. } = &declaration.kind {
                struct Holes(bool);
                impl crate::walk::Visitor for Holes {
                    fn ty(&mut self, t: &mut crate::ast::TypeExpr) {
                        self.0 |= matches!(t.kind, crate::ast::TypeKind::Hole);
                    }
                }
                use crate::walk::Walk;
                let mut holes = Holes(false);
                ty.clone().walk(&mut holes);
                if holes.0 {
                    errors.push(error(
                        "a typed macro signature must be fully written without _",
                        declaration.span,
                    ));
                    valid = false;
                }
                match &ty.kind {
                    crate::ast::TypeKind::Fn { params, .. } if params.len() == parameters.len() => {
                    }
                    crate::ast::TypeKind::Fn { .. } => {
                        errors.push(error("a typed macro implementation must have one parameter for each signature parameter", item.span));
                        valid = false;
                    }
                    _ => {
                        errors.push(error(
                            "a typed macro signature must be a full function type",
                            declaration.span,
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
