//! Definition-query answers with caller-supplied hygienic name resolution.
use super::definition_schedule::Mode;
use super::definitions::{
    TypeQuery, builtin_definition, freshen_parameters, source_definition, type_query,
};
use super::syntax::{Scope, Syntax, SyntaxError};
use crate::ast::{Item, ItemKind};
use crate::span::Span;
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

#[derive(Clone, Debug)]
pub enum QueryResult<Key> {
    Answer {
        key: Key,
        syntax: Syntax,
    },
    /// The run must be discarded and retried with a fresh mark.
    Deferred {
        span: Span,
    },
    /// A catchable `SyntaxError` raised by `definition`.
    Error(SyntaxError),
    /// A source compile error that the macro cannot catch.
    CompileError(SyntaxError),
}

/// Maps resolved identities to definitions. Lookup is injected by the
/// compiler so the table does not invent a second name-resolution policy.
pub struct DefinitionTable<Key> {
    entries: HashMap<Key, Definition>,
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
        if let Definition::Written { item, .. } = &definition {
            assert!(
                matches!(item.kind, ItemKind::TypeDef(_)),
                "definition table accepts type definitions only"
            );
        }
        assert!(
            self.entries.insert(key, definition).is_none(),
            "duplicate definition identity"
        );
    }
    pub fn set_ready(&mut self, key: &Key, ready: bool) {
        let definition = self
            .entries
            .get_mut(key)
            .expect("definition identity must be registered");
        match definition {
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
            !matches!(entry, Definition::Builtin { .. }),
            "cannot replace a builtin definition"
        );
        assert!(
            matches!(
                definition,
                Definition::Written { ready: true, .. } | Definition::Expanded { ready: true, .. }
            ),
            "a finished definition must be ready"
        );
        *entry = definition;
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
        let parsed = match type_query(of) {
            Ok(query) => query,
            Err(error) => return QueryResult::Error(error),
        };
        for atom in parsed.module.iter().chain(std::iter::once(&parsed.name)) {
            if !atom.valid_scope_identity() {
                return QueryResult::CompileError(SyntaxError {
                    message: "a made name cannot change a compiler-provided name identity".into(),
                    span: atom.span(),
                });
            }
        }
        let error = |message: &str| SyntaxError {
            message: message.into(),
            span: parsed.name.span(),
        };
        let key = match lookup(&parsed, mode) {
            Lookup::Found(key) => key,
            Lookup::Deferred => {
                return QueryResult::Deferred {
                    span: parsed.name.span(),
                };
            }
            Lookup::TypeParameter => {
                return QueryResult::Error(error("a type parameter has no type definition"));
            }
            Lookup::ConstructorsHidden => {
                return QueryResult::Error(error("the type's constructors are not exported"));
            }
            Lookup::Missing => {
                return QueryResult::CompileError(error("the type name has no definition"));
            }
        };
        let definition = self
            .entries
            .get(&key)
            .expect("resolved definition identity must be registered");
        let syntax = match definition {
            Definition::Written { ready: false, .. }
            | Definition::Expanded { ready: false, .. } => {
                return QueryResult::Deferred {
                    span: parsed.name.span(),
                };
            }
            Definition::Written {
                item,
                source,
                scope,
                ready: true,
            } => source_definition(item, source, scope, answer),
            Definition::Expanded {
                syntax,
                parameter_answer,
                ready: true,
            } => match parameter_answer {
                Some(old) => freshen_parameters(syntax, *old, answer),
                None => syntax.clone(),
            }
            .as_answer(),
            Definition::Builtin { name } => {
                match builtin_definition(name, parsed.name.scope(), parsed.name.span(), answer) {
                    Some(syntax) => syntax,
                    None => {
                        return QueryResult::Error(error(
                            "this built-in type has no written definition",
                        ));
                    }
                }
            }
        };
        QueryResult::Answer { key, syntax }
    }
}
