use super::syntax::SyntaxError;
pub use syntax3::lisp::expansion::{
    ExpansionSession, MacroArgument, MacroDefinition, MacroVariant,
};
/// Find and validate source macro definitions. The type checker subsequently
/// verifies the Syntax argument, result, error, and environment types.
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
        let (function, mut variants) = match &binding.value.kind {
            ExprKind::Fn(function) => (Some(function.as_ref()), Vec::new()),
            ExprKind::Clauses(_) => (
                None,
                vec![MacroVariant {
                    target: name.clone(),
                    parameters: vec!["argument".into()],
                    labels: vec![None],
                }],
            ),
            ExprKind::Call { callee, args } if matches!(&callee.kind, ExprKind::Name { module: None, name } if name.name == crate::syn3::lower::MULTI) =>
            {
                let mut variants = Vec::new();
                for arg in args {
                    let Some(shape) = &arg.label else { continue };
                    let ExprKind::Name {
                        module: None,
                        name: target,
                    } = &arg.value.kind
                    else {
                        continue;
                    };
                    let mut parts = shape.name.split(',');
                    let positions = parts
                        .next()
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0);
                    let mut labels = vec![None; positions];
                    labels.extend(parts.map(|s| Some(s.to_owned())));
                    variants.push(MacroVariant {
                        target: target.name.clone(),
                        parameters: (0..labels.len()).map(|i| format!("argument{i}")).collect(),
                        labels,
                    });
                }
                (None, variants)
            }
            _ => {
                errors.push(error(
                    "a macro binding needs a function expression or shape branches",
                    binding.value.span,
                ));
                continue;
            }
        };
        let mut parameters = Vec::new();
        let mut labels = Vec::new();
        let mut valid = true;
        for parameter in function.into_iter().flat_map(|f| &f.params) {
            let Param::Pattern(p) = parameter else {
                continue;
            };
            let (label, p) = match &p.kind {
                PatternKind::Named { name, pattern } if name.name.starts_with("%macro_label:") => (
                    Some(name.name.trim_start_matches("%macro_label:").to_owned()),
                    pattern.as_ref(),
                ),
                _ => (None, p),
            };
            match &p.kind {
                PatternKind::Var(name) => parameters.push(name.clone()),
                PatternKind::Typed { binder, .. } if !binder.discard => {
                    parameters.push(binder.name.clone())
                }
                _ => {
                    errors.push(error(
                        "a macro parameter must be a name, optionally with an explicit label",
                        p.span,
                    ));
                    valid = false;
                    continue;
                }
            }
            if label
                .as_ref()
                .is_some_and(|label| labels.contains(&Some(label.clone())))
            {
                errors.push(error("a macro parameter label is repeated", p.span));
                valid = false;
            }
            labels.push(label);
        }
        if let Some(first) = variants.first() {
            parameters = first.parameters.clone();
            labels = first.labels.clone();
        }
        variants.sort_by(|a, b| a.labels.cmp(&b.labels));
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
                    crate::ast::TypeKind::Fn { params, .. } => {
                        if let Some(mut advertised) = signature_shapes(params) {
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
                                errors.push(error("a typed macro implementation must have one parameter for each signature parameter in every parameter-data shape, with matching explicit labels", item.span));
                                valid = false;
                            }
                        }
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

/// Recover directly written shapes; resolved aliases are checked by inference,
/// where declared type parameters and imported names have binding identities.
fn signature_shapes(params: &[crate::ast::FnParamType]) -> Option<Vec<Vec<Option<String>>>> {
    use crate::ast::{TypeExpr, TypeKind};
    fn shapes(ty: &TypeExpr) -> Option<Vec<Vec<Option<String>>>> {
        Some(match &ty.kind {
            TypeKind::Tuple(xs) => vec![vec![None; xs.len()]],
            TypeKind::Record {
                base: None,
                fields,
                removed,
            } if removed.is_empty() => {
                let mut labels = fields
                    .iter()
                    .map(|(l, _)| Some(l.name.clone()))
                    .collect::<Vec<_>>();
                labels.sort();
                vec![labels]
            }
            TypeKind::Union(xs) => {
                let mut out = Vec::new();
                for x in xs {
                    out.extend(shapes(x)?);
                }
                out
            }
            _ => return None,
        })
    }
    if let [param] = params
        && param
            .label
            .as_ref()
            .is_some_and(|l| l.name == crate::syn3::lower::DATA)
    {
        shapes(&param.ty)
    } else {
        let mut labels = params
            .iter()
            .map(|p| p.label.as_ref().map(|l| l.name.clone()))
            .collect::<Vec<_>>();
        labels.sort();
        Some(vec![labels])
    }
}
