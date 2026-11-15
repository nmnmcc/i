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

/// One ordinary parameter-data shape of a macro implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroVariant {
    pub target: String,
    pub parameters: Vec<String>,
    pub labels: Vec<Option<String>>,
}

/// The written interface of a syntax macro, before type checking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroDefinition {
    pub item: usize,
    pub name: String,
    pub parameters: Vec<String>,
    /// Explicit argument labels; bare names are positional (§6.11).
    pub labels: Vec<Option<String>>,
    pub variants: Vec<MacroVariant>,
    /// Index of the declared runtime signature, for a typed macro.
    pub signature: Option<usize>,
}

/// An unevaluated syntax argument, with its optional written call label.
#[derive(Clone, Debug)]
pub struct MacroArgument {
    pub label: Option<String>,
    pub syntax: Syntax,
}

impl MacroDefinition {
    /// Select an implementation shape before compiling its fixed-arity ABI.
    pub fn variant(
        &self,
        arguments: &[MacroArgument],
        item: Option<&Syntax>,
        call: &Syntax,
    ) -> Result<(Self, String), SyntaxError> {
        if self.variants.is_empty() {
            self.arguments(arguments, item.cloned(), call)?;
            return Ok((self.clone(), self.name.clone()));
        }
        for variant in &self.variants {
            let mut definition = self.clone();
            definition.parameters = variant.parameters.clone();
            definition.labels = variant.labels.clone();
            definition.variants.clear();
            if definition.arguments(arguments, item.cloned(), call).is_ok() {
                return Ok((definition, variant.target.clone()));
            }
        }
        Err(SyntaxError {
            message: "macro arguments do not match an implementation parameter-data shape".into(),
            span: call.span(),
        })
    }

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
        let mut labeled = false;
        for argument in arguments {
            let index = if let Some(label) = &argument.label {
                labeled = true;
                self.labels
                    .iter()
                    .position(|name| name.as_ref() == Some(label))
                    .ok_or_else(|| error("a macro argument label does not match a parameter"))?
            } else {
                if labeled {
                    return Err(error(
                        "positional macro arguments must precede labeled arguments",
                    ));
                }
                self.labels
                    .iter()
                    .enumerate()
                    .find(|(i, label)| label.is_none() && slots[*i].is_none())
                    .map(|(i, _)| i)
                    .ok_or_else(|| {
                        error("a positional macro argument does not match a positional parameter")
                    })?
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

pub use super::expansion3::definitions;
