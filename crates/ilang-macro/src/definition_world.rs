//! Definition queries over a frozen item-phase package graph.
//!
//! Source modules already expanded by the project driver are immutable. The
//! current module is rebuilt from its item snapshot for every attempt, so a
//! deferred run cannot observe its own uncommitted output.
//!
//! Definition queries need to agree on what a type definition binds, whether
//! its constructor is hidden outside its module, and the type parameters of
//! an item. In the source tree a sum `T = C1 | C2` and its members
//! `C1 = (…)`, `C2 = (…)` are three items, and each is a definition of its
//! own, so definition queries must retain all three items. The table of the
//! definitions and the rows of the answers are in `lisp::definition_table`.
use crate::item_phase::{DefinitionIdentity, Definitions, Resolution, Snapshot};
use ilang_check::{
    Parsed,
    hir::BuiltinType,
    hygiene::{self, Binding, ModuleName},
};
use ilang_host::{
    macro_budget::MacroBudget,
    syntax::{DefinitionReply, charge_definition},
};
use ilang_package::{ModuleKind, PackageGraph};
use ilang_syntax::{
    ast::{
        self, ExprKind, Ident, Item, ItemKind, PatternKind, TItem, Type, TypeDef, TypeKind, TypeRhs,
    },
    lisp::{
        Datum, DatumKind,
        definition_schedule::Mode,
        definition_table::{Definition, DefinitionCost, DefinitionTable, Lookup, QueryResult},
        definitions::type_query,
        syntax::{Node, Scope, ScopeIdentity, Syntax},
    },
    span::Span,
};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

const TEMPLATE_ANSWER: u64 = u64::MAX - 1;

struct TypeEntry {
    key: DefinitionIdentity,
    exported: bool,
    constructors_visible: bool,
    ready: bool,
    position: usize,
    expanded: Option<Syntax>,
}
#[derive(Default)]
struct ModuleNames {
    own: HashMap<String, usize>,
    imported: HashMap<String, usize>,
    aliases: HashMap<String, u32>,
}
struct Index {
    entries: Vec<TypeEntry>,
    modules: HashMap<u32, ModuleNames>,
    generated: Vec<Binding<usize>>,
}

/// Owns the already expanded imported modules and constructs the current
/// module's definition index once per item attempt.
pub struct DefinitionWorld {
    graph: PackageGraph,
    parsed: Vec<Arc<Parsed>>,
    next_answer: Arc<AtomicU64>,
    lisp_documents: HashMap<u32, Vec<Datum>>,
    source_trees: HashMap<u32, std::sync::OnceLock<ilang_syntax::lisp::source_tree::SourceTree>>,
    current: Mutex<Option<(u32, u64, Arc<Index>)>>,
}
impl DefinitionWorld {
    pub fn new(graph: &PackageGraph, parsed: &[Parsed]) -> Self {
        Self::new_shared(
            graph,
            parsed.iter().cloned().map(Arc::new).collect(),
            Arc::new(AtomicU64::new(0)),
        )
    }
    /// All modules in one project share the same answer identity allocator.
    pub fn new_shared(
        graph: &PackageGraph,
        parsed: Vec<Arc<Parsed>>,
        next_answer: Arc<AtomicU64>,
    ) -> Self {
        let source_trees = parsed
            .iter()
            .map(|p| (p.source.0, std::sync::OnceLock::new()))
            .collect();
        let lisp_documents = parsed
            .iter()
            .filter_map(|p| {
                let file = graph.sources.get(p.source);
                file.display.ends_with(".is").then(|| {
                    let end = p
                        .scoped
                        .as_ref()
                        .map_or(file.text.len(), |scoped| scoped.original_len() as usize);
                    (
                        p.source.0,
                        ilang_syntax::lisp::parse(&file.text[..end])
                            .expect("parsed Lisp source remains valid")
                            .datums,
                    )
                })
            })
            .collect();
        Self {
            graph: graph.clone(),
            parsed,
            next_answer,
            lisp_documents,
            source_trees,
            current: Mutex::new(None),
        }
    }
    fn source_tree(&self, parsed: &Parsed) -> &ilang_syntax::lisp::source_tree::SourceTree {
        let file = self.graph.sources.get(parsed.source);
        let end = parsed
            .scoped
            .as_ref()
            .map_or(file.text.len(), |s| s.original_len() as usize);
        self.source_trees[&parsed.source.0].get_or_init(|| {
            ilang_syntax::lisp::source_tree::SourceTree::read(
                &Scope {
                    module: parsed.source.0,
                    marks: Default::default(),
                },
                &file.text[..end],
                file.display.ends_with(".is"),
            )
        })
    }

    fn answer_id(&self) -> u64 {
        self.next_answer
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| {
                (id < TEMPLATE_ANSWER).then_some(id + 1)
            })
            .expect("definition answer identity space exhausted")
    }
    fn source_module(&self, source: u32) -> Option<&Parsed> {
        self.parsed
            .iter()
            .find(|p| p.source.0 == source)
            .map(AsRef::as_ref)
    }
    fn target(&self, p: &Parsed, import: &ilang_syntax::ast::Import) -> Option<u32> {
        let package = &self.graph.packages[p.package];
        let first = &import.path.first()?.name;
        let id = if first == "i" {
            Some(self.graph.library)
        } else if first == &package.name {
            Some(p.package)
        } else {
            package
                .dependencies
                .iter()
                .find(|(name, _)| name == first)
                .and_then(|(_, id)| *id)
                .or_else(|| {
                    (p.kind == ModuleKind::Test)
                        .then(|| {
                            package
                                .dev_dependencies
                                .iter()
                                .find(|(name, _)| name == first)
                                .and_then(|(_, id)| *id)
                        })
                        .flatten()
                })
        }?;
        let tail = import
            .path
            .iter()
            .skip(1)
            .map(|part| part.name.as_str())
            .collect::<Vec<_>>()
            .join("/");
        let path = format!("{}/{tail}", self.graph.packages[id].name);
        self.parsed
            .iter()
            .find(|m| m.package == id && m.path == path && m.kind != ModuleKind::Prelude)
            .map(|m| m.source.0)
    }
    fn build(&self, snapshot: &Snapshot) -> Index {
        let mut index = Index {
            entries: Vec::new(),
            modules: HashMap::new(),
            generated: Vec::new(),
        };
        for p in &self.parsed {
            let current = p.source.0 == snapshot.source;
            let module = if current { &snapshot.module } else { &p.ast };
            let scoped = if current {
                Some(&snapshot.scoped)
            } else {
                p.scoped.as_ref()
            };
            let scope = Scope {
                module: p.source.0,
                marks: Default::default(),
            };
            let names = index.modules.entry(p.source.0).or_default();
            for (position, item) in module.items.iter().enumerate() {
                let ItemKind::TypeDef(def) = &item.kind else {
                    continue;
                };
                let binders = type_binders(def);
                let ready = !current || snapshot.ready[position];
                for binder in binders {
                    let atom = scoped
                        .and_then(|s| s.origin(binder.span.0))
                        .map(|origin| origin.syntax.clone())
                        .unwrap_or_else(|| {
                            Syntax::from_datum(
                                &Datum {
                                    kind: DatumKind::UpName(binder.name.clone()),
                                    span: binder.span.0,
                                    docs: Vec::new(),
                                },
                                &scope,
                            )
                        });
                    let key = DefinitionIdentity {
                        source: p.source.0,
                        scope: atom.scope().clone(),
                        name: binder.name.clone(),
                        span: binder.span.0,
                        builtin: false,
                    };
                    let exported = module.exports.iter().any(|e| e.name.name == binder.name);
                    let constructors_visible = !has_own_constructors(def)
                        || module
                            .exports
                            .iter()
                            .any(|e| e.name.name == binder.name && e.constructors);
                    let (exported, constructors_visible) = self
                        .source_tree(p)
                        .definition_visibility(&binder.name)
                        .unwrap_or((exported, constructors_visible));
                    let override_syntax = if current {
                        snapshot.item_overrides.get(&item.span.0)
                    } else {
                        scoped.and_then(|source| source.item_override(item.span.0))
                    };
                    let expanded =
                        override_syntax
                            .map(|(syntax, _)| syntax.clone())
                            .or_else(|| {
                                scoped.filter(|s| s.is_generated(item.span.0)).map(|s| {
                                    s.origin(item.span.0)
                                        .expect("generated type item retains its origin")
                                        .syntax
                                        .clone()
                                })
                            });
                    let n = index.entries.len();
                    index.entries.push(TypeEntry {
                        key: key.clone(),
                        exported,
                        constructors_visible,
                        ready,
                        position,
                        expanded,
                    });
                    if atom.scope().marks.is_empty() {
                        names.own.entry(binder.name.clone()).or_insert(n);
                    } else if current {
                        index.generated.push(Binding {
                            text: binder.name.clone(),
                            scope: atom.scope().clone(),
                            identity: atom.scope_identity().cloned(),
                            target: n,
                        });
                    }
                }
            }
        }
        for p in &self.parsed {
            let current = p.source.0 == snapshot.source;
            let module = if current { &snapshot.module } else { &p.ast };
            for import in &module.imports {
                let Some(target) = self.target(p, import) else {
                    continue;
                };
                if self.source_module(target).is_none() {
                    continue;
                }
                let alias = import
                    .alias
                    .as_ref()
                    .unwrap_or_else(|| import.path.last().expect("import has a path"));
                let selected: Vec<_> = import
                    .names
                    .iter()
                    .filter_map(|name| {
                        let exported = index.modules[&target]
                            .own
                            .get(&name.name)
                            .is_some_and(|id| index.entries[*id].exported);
                        exported
                            .then(|| index.modules[&target].own.get(&name.name).copied())
                            .flatten()
                            .map(|id| (name.name.clone(), id))
                    })
                    .collect();
                let names = index
                    .modules
                    .get_mut(&p.source.0)
                    .expect("module is indexed");
                names.aliases.entry(alias.name.clone()).or_insert(target);
                for (name, id) in selected {
                    names.imported.entry(name).or_insert(id);
                }
            }
        }
        index
    }
}

fn text(s: &Syntax) -> &str {
    match s.node() {
        Node::Up(s) | Node::Name(s) => s,
        _ => unreachable!("type query atoms are names"),
    }
}
fn builtin(name: &str) -> bool {
    BuiltinType::ALL.into_iter().any(|b| b.name() == name)
        || ilang_syntax::lisp::definitions::builtin_definition_names().contains(&name)
}
fn error_id(query: &Syntax, kind: &str) -> DefinitionIdentity {
    DefinitionIdentity {
        source: u32::MAX,
        scope: query.scope().clone(),
        name: format!("{kind}:{}", text(query)),
        span: Span::default(),
        builtin: true,
    }
}
fn resolution(reply: DefinitionReply, audit: Option<DefinitionIdentity>) -> Resolution {
    Resolution { reply, audit }
}

impl Definitions for DefinitionWorld {
    fn fresh_answer(&self) -> u64 {
        self.answer_id()
    }
    fn prepare(&self, snapshot: &Snapshot) {
        let index = Arc::new(self.build(snapshot));
        *self.current.lock().expect("definition index lock poisoned") =
            Some((snapshot.source, snapshot.epoch, index));
    }
    fn resolve(
        &self,
        snapshot: &Snapshot,
        in_run: &Syntax,
        result: &Syntax,
        mode: Mode,
        mark: u64,
        budget: &MacroBudget,
    ) -> Resolution {
        let index = {
            let current = self.current.lock().expect("definition index lock poisoned");
            let (source, epoch, index) = current.as_ref().expect("definition index was prepared");
            assert_eq!(
                (*source, *epoch),
                (snapshot.source, snapshot.epoch),
                "definition lookup uses its frozen item snapshot"
            );
            index.clone()
        };
        let query = match type_query(result) {
            Ok(query) => query,
            Err(error) => {
                return resolution(
                    DefinitionReply::Error {
                        message: error.message,
                        at: result.clone(),
                    },
                    None,
                );
            }
        };
        for atom in query.module.iter().chain(std::iter::once(&query.name)) {
            if !atom.valid_scope_identity() {
                return resolution(
                    DefinitionReply::CompileError {
                        message: "a made name cannot change a compiler-provided name identity"
                            .into(),
                        at: atom.clone(),
                    },
                    None,
                );
            }
        }
        let early = mode == Mode::Fallback || query.name.scope().marks.contains(&mark);
        let own_mark = query.name.scope().marks.contains(&mark);
        if matches!(
            query.name.scope_identity(),
            Some(ScopeIdentity::DefinitionParameter { .. } | ScopeIdentity::TypeVariable { .. })
        ) || self.is_local_type_parameter(snapshot, &query.name)
        {
            return resolution(
                DefinitionReply::Error {
                    message: "a type parameter has no type definition".into(),
                    at: query.name,
                },
                None,
            );
        }
        let name = text(&query.name);
        let from_source = query.name.scope().module;
        let names = index.modules.get(&from_source);
        let found = if let Some(module) = &query.module {
            let aliases = index.modules.get(&module.scope().module);
            let target = hygiene::lookup(
                module,
                snapshot.source,
                &[],
                &[],
                |source, name| {
                    index
                        .modules
                        .get(&source)?
                        .aliases
                        .get(name)
                        .copied()
                        .map(|target| ModuleName {
                            target,
                            exported: false,
                        })
                },
                |_| None,
                |_| None,
            );
            let _ = aliases;
            match target {
                hygiene::Lookup::Found(target) => index
                    .modules
                    .get(&target)
                    .and_then(|m| m.own.get(name))
                    .copied()
                    .filter(|id| index.entries[*id].exported),
                hygiene::Lookup::NotExported | hygiene::Lookup::Missing => None,
            }
        } else if matches!(
            query.name.scope_identity(),
            Some(ScopeIdentity::Builtin { .. })
        ) {
            None
        } else {
            match hygiene::lookup(
                &query.name,
                snapshot.source,
                &[],
                &index.generated,
                |source, name| {
                    let module = index.modules.get(&source)?;
                    let id = module
                        .own
                        .get(name)
                        .or_else(|| module.imported.get(name))
                        .copied()?;
                    Some(ModuleName {
                        target: id,
                        exported: index.entries[id].exported,
                    })
                },
                |_| None,
                |_| None,
            ) {
                hygiene::Lookup::Found(id) => Some(id),
                hygiene::Lookup::NotExported | hygiene::Lookup::Missing => None,
            }
        };
        let _ = names;
        let may_be_defined_here = query.module.is_none()
            && (from_source == snapshot.source || !query.name.scope().marks.is_empty());
        if mode == Mode::Normal && snapshot.other_runs && !own_mark && may_be_defined_here {
            let current_ready = found.is_some_and(|id| {
                index.entries[id].key.source == snapshot.source && index.entries[id].ready
            });
            if !current_ready {
                return resolution(DefinitionReply::Deferred { at: in_run.clone() }, None);
            }
        }
        let builtin_name = query.module.is_none() && found.is_none() && builtin(name);
        if let Some(id) = found {
            let entry = &index.entries[id];
            if !entry.ready {
                return resolution(DefinitionReply::Deferred { at: in_run.clone() }, None);
            }
            let foreign = query.module.is_some() || entry.key.source != from_source;
            let same_generated_scope = query.module.is_none()
                && entry.key.source == snapshot.source
                && !entry.key.scope.marks.is_empty()
                && entry.key.scope == *query.name.scope();
            if foreign && !entry.constructors_visible && !same_generated_scope {
                let identity = error_id(&query.name, "constructors-hidden");
                return resolution(
                    DefinitionReply::Error {
                        message: "the type's constructors are not exported".into(),
                        at: query.name,
                    },
                    early.then_some(identity),
                );
            }
            let key = entry.key.clone();
            let answer = self.answer_id();
            let construction = if let Some(syntax) = &entry.expanded {
                charge_definition(budget, DefinitionCost::Syntax(syntax))
                    .and_then(|()| charge_definition(budget, DefinitionCost::Syntax(syntax)))
                    .map(|()| {
                        ilang_syntax::lisp::definitions::expanded_definition(syntax, answer)
                            .expect("expanded type item remains valid")
                    })
            } else {
                let parsed = self
                    .source_module(entry.key.source)
                    .expect("definition source exists");
                let current = entry.key.source == snapshot.source;
                let module = if current {
                    &snapshot.module
                } else {
                    &parsed.ast
                };
                let item = &module.items[entry.position];
                let datum = if let Some(document) = self.lisp_documents.get(&parsed.source.0) {
                    document
                        .iter()
                        .find(|datum| datum.span == item.span.0)
                        .expect("written Lisp type has its source datum")
                        .clone()
                } else {
                    self.source_tree(parsed)
                        .item_datum(item.span.0)
                        .expect("a written definition has its new tree item")
                };
                let mut table = DefinitionTable::default();
                table.insert(
                    (),
                    Definition::tree_with_datum(
                        item.clone(),
                        datum,
                        Scope {
                            module: parsed.source.0,
                            marks: Default::default(),
                        },
                        true,
                    ),
                );
                return match table.query_metered(
                    result,
                    mode,
                    answer,
                    |_, _| Lookup::Found(()),
                    |cost| charge_definition(budget, cost).map_err(Into::into),
                ) {
                    Ok(QueryResult::Answer { syntax, .. }) => {
                        resolution(DefinitionReply::Answer(syntax), early.then_some(key))
                    }
                    Err(error) => resolution(
                        DefinitionReply::CompileError {
                            message: error.to_string(),
                            at: query.name,
                        },
                        None,
                    ),
                    _ => unreachable!("a ready written definition answers its resolved query"),
                };
            };
            match construction {
                Ok(syntax) => resolution(DefinitionReply::Answer(syntax), early.then_some(key)),
                Err(error) => resolution(
                    DefinitionReply::CompileError {
                        message: error.to_string(),
                        at: query.name,
                    },
                    None,
                ),
            }
        } else if builtin_name {
            let key = DefinitionIdentity {
                source: u32::MAX,
                scope: Scope {
                    module: 0,
                    marks: Default::default(),
                },
                name: name.to_owned(),
                span: Span::default(),
                builtin: true,
            };
            let mut table = DefinitionTable::default();
            table.insert(
                key.clone(),
                Definition::Builtin {
                    name: name.to_owned(),
                },
            );
            match table.query_metered(
                result,
                mode,
                self.answer_id(),
                |_, _| Lookup::Found(key.clone()),
                |cost| charge_definition(budget, cost).map_err(|e| e.into()),
            ) {
                Ok(QueryResult::Answer { syntax, .. }) => {
                    resolution(DefinitionReply::Answer(syntax), early.then_some(key))
                }
                Ok(QueryResult::Error(e)) => resolution(
                    DefinitionReply::Error {
                        message: e.message,
                        at: e.at,
                    },
                    early.then_some(key),
                ),
                Ok(QueryResult::Deferred { at }) => {
                    resolution(DefinitionReply::Deferred { at }, None)
                }
                Ok(QueryResult::CompileError(e)) => resolution(
                    DefinitionReply::CompileError {
                        message: e.message,
                        at: e.at,
                    },
                    None,
                ),
                Err(error) => resolution(
                    DefinitionReply::CompileError {
                        message: error.to_string(),
                        at: query.name,
                    },
                    None,
                ),
            }
        } else if mode == Mode::Fallback && may_be_defined_here && !own_mark && snapshot.other_runs
        {
            resolution(DefinitionReply::Deferred { at: in_run.clone() }, None)
        } else {
            resolution(
                DefinitionReply::CompileError {
                    message: format!("the type `{name}` has no definition"),
                    at: query.name,
                },
                None,
            )
        }
    }
}
impl DefinitionWorld {
    fn is_local_type_parameter(&self, snapshot: &Snapshot, name: &Syntax) -> bool {
        let Some(p) = self.source_module(name.scope().module) else {
            return false;
        };
        let module = if p.source.0 == snapshot.source {
            &snapshot.module
        } else {
            &p.ast
        };
        let scoped = if p.source.0 == snapshot.source {
            Some(&snapshot.scoped)
        } else {
            p.scoped.as_ref()
        };
        module.items.iter().any(|item| {
            if name.span().start < item.span.0.start || name.span().end > item.span.0.end {
                return false;
            }
            let params = type_parameters(item);
            params.iter().any(|parameter| {
                let scope = scoped
                    .and_then(|s| s.origin(parameter.span.0))
                    .map(|origin| origin.syntax.scope().clone())
                    .unwrap_or_else(|| Scope {
                        module: p.source.0,
                        marks: Default::default(),
                    });
                parameter.name == text(name) && scope == *name.scope()
            })
        })
    }
}

/// `T(head) = value` where the right side is the type of a value and the
/// head is not empty: an alias pattern, which binds the names of its head.
pub fn alias_pattern(def: &TypeDef) -> bool {
    def.alias_pattern
}

/// The type names that a definition binds: its own name, or for an alias
/// pattern the plain names of its head.
pub fn type_binders(def: &TypeDef) -> Vec<&Ident> {
    let mut binders = Vec::new();
    if alias_pattern(def) {
        for part in &def.head {
            pattern_binders(part, &mut binders);
        }
    } else {
        binders.push(&def.name);
    }
    binders
}

/// The plain names of a type pattern: a name with no module and no
/// argument, inside named types and positional data.
pub fn pattern_binders<'a>(ty: &'a Type, output: &mut Vec<&'a Ident>) {
    let mut pending = vec![ty];
    while let Some(ty) = pending.pop() {
        match &ty.kind {
            TypeKind::Named {
                module: None,
                name,
                args,
            } if args.is_empty() => output.push(name),
            TypeKind::Named { args, .. } => pending.extend(args.iter().rev()),
            // Tuple handling requires positional data only, and
            // at least one.
            TypeKind::Data(items)
                if !items.is_empty() && items.iter().all(|i| matches!(i, TItem::Pos(_))) =>
            {
                pending.extend(items.iter().rev().map(|i| match i {
                    TItem::Pos(t) | TItem::Label(_, t) | TItem::Spread(t) => t,
                }))
            }
            _ => {}
        }
    }
}

/// A definition with a constructor of its own (data, or opaque): its
/// definition is given to another module only when the export shows the
/// constructors, `T(..)`.
pub fn has_own_constructors(def: &TypeDef) -> bool {
    !matches!(def.rhs, TypeRhs::Type(_))
}

/// The plain names of a head: the type parameters of a definition that is
/// not an alias pattern. `None` when a part is not a plain name.
pub fn head_parameters(head: &[Type]) -> Option<Vec<&Ident>> {
    head.iter()
        .map(|t| match &t.kind {
            TypeKind::Named {
                module: None,
                name,
                args,
            } if args.is_empty() => Some(name),
            _ => None,
        })
        .collect()
}

/// The type parameters of a binding: the `<A>` lists of the branches of
/// its chain, each name once per list that adds it, then those of its
/// function type.
pub fn binding_type_parameters(binding: &ast::Binding) -> Vec<&Ident> {
    let mut generics: Vec<&Ident> = Vec::new();
    if let ExprKind::Chain(fs) = &binding.value.kind {
        for f in fs {
            let before = generics.len();
            for g in &f.generics {
                if !generics[..before].iter().any(|x| x.name == g.name) {
                    generics.push(g);
                }
            }
        }
    }
    if let PatternKind::Typed { ty, .. } = &binding.pattern.kind
        && let TypeKind::Fn(f) = &ty.kind
    {
        for g in &f.generics {
            if !generics.iter().any(|x| x.name == g.name) {
                generics.push(g);
            }
        }
    }
    generics
}

/// The type parameters that an item declares.
pub fn type_parameters(item: &Item) -> Vec<&Ident> {
    match &item.kind {
        ItemKind::TypeDef(def) if alias_pattern(def) => Vec::new(),
        ItemKind::TypeDef(def) => head_parameters(&def.head).unwrap_or_default(),
        ItemKind::Bind(binding) => binding_type_parameters(binding),
        ItemKind::Decl { ty, .. } => match &ty.kind {
            TypeKind::Fn(f) => f.generics.iter().collect(),
            _ => Vec::new(),
        },
        ItemKind::Macro { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use migration_syntax::ast as old;

    /// The answers on the source tree and on its lowering.
    #[test]
    fn answers_agree_with_the_lowered_tree() {
        let source = "\
T(A, B) = (A, B)
U = Opaque
V(A) = List(A)
W((A, List(B)), C) = value
f = <A, B>(a: A) { a } & <B, C>(b: B) { b }
g: <A, N>(A) <- N -> A = (a) { a }
h: <A>(A) -> A
k = 1
";
        let (module, errors) = ilang_syntax::parse::parse_module(source);
        assert!(errors.is_empty(), "{errors:?}");
        let (lowered, _) = migration_syntax::syn3::lower::lower(&module);
        assert_eq!(module.items.len(), lowered.items.len());
        let names = |ids: Vec<&Ident>| ids.iter().map(|i| i.name.clone()).collect::<Vec<_>>();
        let old_names = |ids: &[old::Ident]| {
            ids.iter()
                .map(|i| i.name.clone())
                // The row parameter that the lowering adds.
                .filter(|n| !n.contains('%'))
                .collect::<Vec<_>>()
        };
        for (new, old) in module.items.iter().zip(&lowered.items) {
            let old_parameters = match &old.kind {
                old::ItemKind::TypeDef(def) => old_names(&def.params),
                old::ItemKind::Binding(b) => old_names(&b.type_params),
                old::ItemKind::Declaration { type_params, .. } => old_names(type_params),
                old::ItemKind::MacroItem { .. } => Vec::new(),
            };
            assert_eq!(names(type_parameters(new)), old_parameters);
            if let (ItemKind::TypeDef(new), old::ItemKind::TypeDef(old)) = (&new.kind, &old.kind) {
                assert_eq!(alias_pattern(new), old.alias_pattern, "{}", new.name.name);
                assert_eq!(
                    has_own_constructors(new),
                    matches!(old.body, old::TypeBody::Custom(_)),
                    "{}",
                    new.name.name
                );
            }
        }
        let ItemKind::TypeDef(w) = &module.items[3].kind else {
            panic!()
        };
        assert_eq!(names(type_binders(w)), ["A", "B", "C"]);
        let ItemKind::TypeDef(t) = &module.items[0].kind else {
            panic!()
        };
        assert_eq!(names(type_binders(t)), ["T"]);
    }
}
