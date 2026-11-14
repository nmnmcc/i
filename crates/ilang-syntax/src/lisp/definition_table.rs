//! Definition-query answers with caller-supplied hygienic name resolution.
use super::definition_schedule::Mode;
use super::definitions::{
    TypeQuery, builtin_definition, freshen_parameters, source_definition_from_syntax, type_query,
};
use super::syntax::{Scope, Syntax};
use crate::ast::{Item, ItemKind};
use std::collections::HashMap;
use std::hash::Hash;

/// One definition, retained in its written form until it is ready.
#[derive(Clone)]
pub enum Definition {
    Written {
        item: Box<Item>,
        source: String,
        scope: Scope,
        ready: bool,
    },
    /// An expanded item keeps the individual scopes of its syntax atoms.
    /// Parameter identities, when present, are freshened on each query.
    Expanded {
        syntax: Syntax,
        parameter_answer: Option<u64>,
        ready: bool,
    },
    Builtin {
        name: String,
    },
}

/// Hygienic lookup decides lexical shadowing, visibility, marks and whether
/// outstanding item runs may still change the answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lookup<Key> {
    Found(Key),
    TypeParameter,
    ConstructorsHidden,
    Deferred,
    Missing,
}

/// A query failure keeps the actual syntax node, including its source module.
#[derive(Clone, Debug)]
pub struct QueryError {
    pub message: String,
    pub at: Syntax,
}

#[derive(Clone, Debug)]
pub enum QueryResult<Key> {
    Answer {
        key: Key,
        syntax: Syntax,
    },
    /// The run must be discarded and retried with a fresh mark.
    Deferred {
        at: Syntax,
    },
    /// A catchable `SyntaxError` raised by `definition`.
    Error(QueryError),
    /// A source compile error that the macro cannot catch.
    CompileError(QueryError),
}

/// The construction to charge before a query answer is allocated.
pub enum DefinitionCost<'a> {
    Syntax(&'a Syntax),
    Builtin(&'a str),
    /// One streamed charge for a written definition's encoded shape.
    Event(super::SyntaxCharge),
    /// Already parsed Lisp item, without constructing Syntax nodes.
    Datum(&'a super::Datum),
}

const TEMPLATE_ANSWER: u64 = u64::MAX;
struct Entry {
    definition: Definition,
    lisp_document: Option<Vec<super::Datum>>,
    source_encoder: Option<super::SourceEncoder>,
}
impl Entry {
    fn new(definition: Definition) -> Self {
        if let Definition::Written { item, .. } = &definition {
            assert!(
                matches!(item.kind, ItemKind::TypeDef(_)),
                "definition table accepts type definitions only"
            );
        }
        let lisp_document = match &definition {
            Definition::Written { item, source, .. }
                if source[item.span.start as usize..item.span.end as usize].starts_with('(') =>
            {
                Some(
                    super::parse(source)
                        .expect("parsed Lisp source remains valid")
                        .datums,
                )
            }
            _ => None,
        };
        let source_encoder = match &definition {
            Definition::Written { source, scope, .. } if lisp_document.is_none() => {
                Some(super::SourceEncoder::new(scope, source))
            }
            _ => None,
        };
        Self {
            definition,
            lisp_document,
            source_encoder,
        }
    }
}

/// Maps resolved identities to definitions. Lookup is injected by the
/// compiler so the table does not invent a second name-resolution policy.
pub struct DefinitionTable<Key> {
    entries: HashMap<Key, Entry>,
}
impl<Key> Default for DefinitionTable<Key> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}
impl<Key: Eq + Hash> DefinitionTable<Key> {
    pub fn insert(&mut self, key: Key, definition: Definition) {
        assert!(
            self.entries.insert(key, Entry::new(definition)).is_none(),
            "duplicate definition identity"
        );
    }
    pub fn set_ready(&mut self, key: &Key, ready: bool) {
        let definition = self
            .entries
            .get_mut(key)
            .expect("definition identity must be registered");
        match &mut definition.definition {
            Definition::Written { ready: current, .. }
            | Definition::Expanded { ready: current, .. } => *current = ready,
            Definition::Builtin { .. } => panic!("builtin definitions are always ready"),
        }
    }
    /// Replace a pending definition after its type macros have expanded.
    pub fn finish(&mut self, key: &Key, definition: Definition) {
        let entry = self
            .entries
            .get_mut(key)
            .expect("definition identity must be registered");
        assert!(
            !matches!(entry.definition, Definition::Builtin { .. }),
            "cannot replace a builtin definition"
        );
        assert!(
            matches!(
                definition,
                Definition::Written { ready: true, .. } | Definition::Expanded { ready: true, .. }
            ),
            "a finished definition must be ready"
        );
        *entry = Entry::new(definition);
    }
    /// Query with a fresh answer identity. A successful answer carries the
    /// stable definition key used by the scheduler's end-of-phase audit.
    pub fn query(
        &self,
        of: &Syntax,
        mode: Mode,
        answer: u64,
        lookup: impl FnOnce(&TypeQuery, Mode) -> Lookup<Key>,
    ) -> QueryResult<Key> {
        self.query_metered(of, mode, answer, lookup, |_| Ok(()))
            .expect("unmetered query cannot exhaust a budget")
    }

    /// Query while charging each answer template before freshening or copying.
    /// The callback shares the current macro run's budget and returns its
    /// resource error without converting it into a catchable SyntaxError.
    pub fn query_metered(
        &self,
        of: &Syntax,
        mode: Mode,
        answer: u64,
        lookup: impl FnOnce(&TypeQuery, Mode) -> Lookup<Key>,
        mut preflight: impl FnMut(DefinitionCost<'_>) -> super::scoped::ChargeResult,
    ) -> Result<QueryResult<Key>, Box<dyn std::error::Error + Send + Sync>> {
        assert_ne!(
            answer, TEMPLATE_ANSWER,
            "answer identity is reserved for templates"
        );
        let parsed = match type_query(of) {
            Ok(query) => query,
            Err(error) => {
                return Ok(QueryResult::Error(QueryError {
                    message: error.message,
                    at: of.clone(),
                }));
            }
        };
        for atom in parsed.module.iter().chain(std::iter::once(&parsed.name)) {
            if !atom.valid_scope_identity() {
                return Ok(QueryResult::CompileError(QueryError {
                    message: "a made name cannot change a compiler-provided name identity".into(),
                    at: atom.clone(),
                }));
            }
        }
        let error = |message: &str| QueryError {
            message: message.into(),
            at: parsed.name.clone(),
        };
        let key = match lookup(&parsed, mode) {
            Lookup::Found(key) => key,
            Lookup::Deferred => {
                return Ok(QueryResult::Deferred {
                    at: parsed.name.clone(),
                });
            }
            Lookup::TypeParameter => {
                return Ok(QueryResult::Error(error(
                    "a type parameter has no type definition",
                )));
            }
            Lookup::ConstructorsHidden => {
                return Ok(QueryResult::Error(error(
                    "the type's constructors are not exported",
                )));
            }
            Lookup::Missing => {
                return Ok(QueryResult::CompileError(error(
                    "the type name has no definition",
                )));
            }
        };
        let definition = self
            .entries
            .get(&key)
            .expect("resolved definition identity must be registered");
        let syntax = match &definition.definition {
            Definition::Written { ready: false, .. }
            | Definition::Expanded { ready: false, .. } => {
                return Ok(QueryResult::Deferred {
                    at: parsed.name.clone(),
                });
            }
            Definition::Written {
                item,
                scope,
                ready: true,
                ..
            } => {
                if let Some(document) = &definition.lisp_document {
                    let datum = document
                        .iter()
                        .find(|datum| datum.span == item.span)
                        .expect("written Lisp type has its source datum");
                    preflight(DefinitionCost::Datum(datum))?;
                    let syntax = Syntax::from_datum(datum, scope);
                    super::definitions::expanded_definition(&syntax, answer)
                        .expect("written Lisp type remains valid")
                } else {
                    let encoder = definition
                        .source_encoder
                        .as_ref()
                        .expect("written source has a lexical index");
                    encoder
                        .visit_item_charge(item, |event| preflight(DefinitionCost::Event(event)))?;
                    source_definition_from_syntax(item, &encoder.item(item), answer)
                }
            }
            Definition::Expanded {
                syntax,
                parameter_answer,
                ready: true,
            } => {
                preflight(DefinitionCost::Syntax(syntax))?;
                if parameter_answer.is_some() {
                    // Freshening and answer-provenance conversion each build a tree.
                    preflight(DefinitionCost::Syntax(syntax))?;
                }
                match parameter_answer {
                    Some(old) => freshen_parameters(syntax, *old, answer),
                    None => syntax.clone(),
                }
                .as_answer()
            }
            Definition::Builtin { name } => {
                if matches!(
                    name.as_str(),
                    "Bool"
                        | "True"
                        | "False"
                        | "Nil"
                        | "Result"
                        | "Ok"
                        | "Error"
                        | "Assert"
                        | "AssertionFailure"
                ) {
                    preflight(DefinitionCost::Builtin(name))?;
                }
                match builtin_definition(name, parsed.name.scope(), parsed.name.span(), answer) {
                    Some(syntax) => syntax,
                    None => {
                        return Ok(QueryResult::Error(error(
                            "this built-in type has no written definition",
                        )));
                    }
                }
            }
        };
        Ok(QueryResult::Answer { key, syntax })
    }
}
