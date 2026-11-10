//! Per-expansion mark and depth management for syntax macro runs.
use super::syntax::{Syntax, SyntaxError, flip_mark_all, validate_expansion};

/// One expansion session owns its fresh marks and counts nested results.
#[derive(Default)]
pub struct ExpansionSession {
    next_mark: u64,
    depth: u16,
}

impl ExpansionSession {
    /// Run a macro, validate and mark its result, then expand that result.
    /// The continuation remains inside this run's expansion depth.
    pub fn run(
        &mut self,
        arguments: &[Syntax],
        call: &Syntax,
        execute: impl FnOnce(&[Syntax], u64) -> Result<Syntax, SyntaxError>,
        expand_result: impl FnOnce(&mut Self, Syntax) -> Result<Syntax, SyntaxError>,
    ) -> Result<Syntax, SyntaxError> {
        if self.depth == 256 {
            return Err(SyntaxError {
                message: "macro expansion exceeds depth 256".into(),
                span: call.span(),
            });
        }
        let mark = self.next_mark;
        self.next_mark = mark.checked_add(1).expect("macro mark space exhausted");
        let marked = flip_mark_all(arguments, mark);
        self.depth += 1;
        let result = execute(&marked, mark).and_then(|result| {
            validate_expansion(&result, &marked)?;
            expand_result(self, result.flip_mark(mark))
        });
        self.depth -= 1;
        result
    }
}

/// The written interface of a syntax macro, before type checking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroDefinition {
    pub item: usize,
    pub name: String,
    pub parameters: Vec<String>,
}

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
        if module.items.iter().any(|other| matches!(&other.kind, ItemKind::Declaration { name: declared, .. } if declared.name == *name)) {
            errors.push(error("a macro cannot have a separate declaration", binding.pattern.span));
            valid = false;
        }
        if valid {
            found.push(MacroDefinition {
                item: index,
                name: name.clone(),
                parameters,
            });
        }
    }
    if errors.is_empty() {
        Ok(found)
    } else {
        Err(errors)
    }
}

/// An unevaluated syntax argument, with its optional written call label.
#[derive(Clone, Debug)]
pub struct MacroArgument {
    pub label: Option<String>,
    pub syntax: Syntax,
}

impl MacroDefinition {
    /// Match call arguments to parameters (§6.6). For an attribute call,
    /// `item` is placed in the last parameter independently of labels.
    pub fn arguments(
        &self,
        arguments: &[MacroArgument],
        item: Option<Syntax>,
        call: &Syntax,
    ) -> Result<Vec<Syntax>, SyntaxError> {
        let error = |message: &str| SyntaxError {
            message: message.into(),
            span: call.span(),
        };
        let expected = self.parameters.len();
        if arguments.len() + usize::from(item.is_some()) != expected {
            return Err(error(
                "the number of macro arguments does not match its parameters",
            ));
        }
        let mut slots = vec![None; expected];
        if let Some(item) = item {
            slots[expected - 1] = Some(item);
        }
        let mut positional = 0;
        let mut labeled = false;
        for argument in arguments {
            let index = if let Some(label) = &argument.label {
                labeled = true;
                self.parameters
                    .iter()
                    .position(|name| name == label)
                    .ok_or_else(|| error("a macro argument label does not match a parameter"))?
            } else {
                if labeled {
                    return Err(error(
                        "positional macro arguments must precede labeled arguments",
                    ));
                }
                let index = positional;
                positional += 1;
                index
            };
            if slots[index].replace(argument.syntax.clone()).is_some() {
                return Err(error("a macro parameter receives more than one argument"));
            }
        }
        Ok(slots
            .into_iter()
            .map(|slot| slot.expect("macro argument count and labels fill every slot"))
            .collect())
    }
}
