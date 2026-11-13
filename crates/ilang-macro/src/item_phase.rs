//! The item phase of syntax expansion.
//!
//! One step leaves all arguments untouched until the selected outermost macro
//! has run. The scheduler installs a successful result only after parsing it.
use crate::item_phase3::{
    builtin, commit_exports, item_names, reject_macro_definitions, type_ready,
};
use crate::{
    body_engine::{BodyEngine, Error},
    body_walk3 as body_walk,
    prepare::MacroPosition,
    session::Transaction,
};
use ilang_syntax::lisp::syntax::{Node, Syntax};
use ilang_syntax::syn3::ast::{Item, ItemKind};

/// A step either finishes an ordinary item or replaces it with the returned
/// items. `retained` is the written type definition: a macro on a type
/// definition adds items beside it and never replaces it.
pub enum Step {
    Finished(Item),
    Expanded {
        retained: Option<Item>,
        items: Vec<Item>,
        transactions: Vec<Transaction>,
    },
}

impl BodyEngine<'_> {
    fn item_syntax(&self, item: &Item) -> Syntax {
        self.item_root3(item).0
    }

    /// Expands one outer item call, or one outer attribute call. A written type
    /// stays in the module; other attributed items are replaced by the result.
    pub fn item_step(
        &mut self,
        mut item: Item,
        definitions: &dyn Definitions,
    ) -> Result<Step, Error> {
        for (index, attribute) in item.attributes.iter().enumerate() {
            if builtin(attribute) {
                continue;
            }
            if let Some(mac) = self.resolve3(
                attribute.module.as_ref(),
                &attribute.name,
                &self.item_locals,
                0,
            ) {
                if let Some(previous) = item.attributes[..index].iter().find(|a| builtin(a)) {
                    return Err(Error::InvalidItem(
                        previous.span.0,
                        "a built-in attribute cannot precede a macro".into(),
                    ));
                }
                if index != 0 {
                    return Err(Error::InvalidItem(
                        item.attributes[0].span.0,
                        "an attribute before a macro must itself name a macro".into(),
                    ));
                }
                let written = self.item_syntax(&item);
                let inner = match written.node() {
                    Node::Round(parts) if matches!(parts.as_slice(), [at, _, _] if matches!(at.node(), Node::Symbol(s) if s == "@")) => {
                        parts[2].clone()
                    }
                    _ => panic!("an attributed item must encode as an attribute wrapper"),
                };
                let args = self.attribute_arguments3(attribute);
                let call = self.atom3(&attribute.name);
                let original_names = item_names(&item);
                let is_type = matches!(item.kind, ItemKind::TypeDef(_));
                let (result, budget, transactions) = self.execute(
                    &mac,
                    args,
                    &call,
                    attribute.span.0,
                    Some(inner.clone()),
                    MacroPosition::Item,
                )?;
                ilang_host::syntax::charge_tree(&budget, &result, true).map_err(|error| {
                    Error::Resource {
                        error: error.into(),
                        macro_name: mac.definition.name.clone(),
                        call: call.clone(),
                        invocation_span: attribute.span.0,
                    }
                })?;
                let result =
                    ilang_syntax::lisp::definitions::freshen_duplicate_heads(&result, || {
                        definitions.fresh_answer()
                    });
                let start =
                    u32::try_from(self.scoped.text().len() + 1).expect("source span overflow");
                let mut items = self
                    .scoped
                    .items3_metered(&result, &call, |work, words| {
                        budget.charge(work, words).map_err(|e| Box::new(e) as _)
                    })
                    .map_err(|error| {
                        crate::body_engine::metered_error(
                            error,
                            &mac.definition.name,
                            &call,
                            attribute.span.0,
                        )
                    })?;
                let end = u32::try_from(self.scoped.text().len()).expect("source span overflow");
                self.scoped.inherit_call_chain(
                    Span::new(start, end),
                    attribute.span.0,
                    &mac.definition.name,
                );
                reject_macro_definitions(&items)?;
                if !is_type
                    && !item.doc.is_empty()
                    && let Some(first) = items.iter_mut().find(|out| {
                        item_names(out)
                            .iter()
                            .any(|name| original_names.contains(name))
                    })
                    && first.doc.is_empty()
                {
                    first.doc.clone_from(&item.doc);
                }
                let retained = if is_type {
                    let root_span = self.item_root3(&item).1;
                    let inner_span = self.scoped.child_span(root_span, 2).unwrap_or(inner.span());
                    self.scoped
                        .set_item_override(item.span.0, inner.clone(), inner_span);
                    self.item_overrides.insert(item.span.0, (inner, inner_span));
                    item.attributes.remove(index);
                    Some(item)
                } else {
                    None
                };
                return Ok(Step::Expanded {
                    retained,
                    items,
                    transactions,
                });
            }
        }
        if let ItemKind::Macro { module, name, args } = &item.kind {
            if let Some(mac) = self.resolve3(module.as_ref(), name, &self.item_locals, 0) {
                let args = self.item_call_arguments3(args);
                let call = self.item_syntax(&item);
                let (result, budget, transactions) =
                    self.execute(&mac, args, &call, item.span.0, None, MacroPosition::Item)?;
                ilang_host::syntax::charge_tree(&budget, &result, true).map_err(|error| {
                    Error::Resource {
                        error: error.into(),
                        macro_name: mac.definition.name.clone(),
                        call: call.clone(),
                        invocation_span: item.span.0,
                    }
                })?;
                let result =
                    ilang_syntax::lisp::definitions::freshen_duplicate_heads(&result, || {
                        definitions.fresh_answer()
                    });
                let start =
                    u32::try_from(self.scoped.text().len() + 1).expect("source span overflow");
                let items = self
                    .scoped
                    .items3_metered(&result, &call, |work, words| {
                        budget.charge(work, words).map_err(|e| Box::new(e) as _)
                    })
                    .map_err(|error| {
                        crate::body_engine::metered_error(
                            error,
                            &mac.definition.name,
                            &call,
                            item.span.0,
                        )
                    })?;
                let end = u32::try_from(self.scoped.text().len()).expect("source span overflow");
                self.scoped.inherit_call_chain(
                    Span::new(start, end),
                    item.span.0,
                    &mac.definition.name,
                );
                reject_macro_definitions(&items)?;
                return Ok(Step::Expanded {
                    retained: None,
                    items,
                    transactions,
                });
            }
            return Err(Error::InvalidItem(
                item.span.0,
                "an item call must name a macro".into(),
            ));
        }
        if matches!(item.kind, ItemKind::TypeDef(_)) {
            let (root, root_span) = self.item_root3(&item);
            let start = self.item_replacements.len();
            let ItemKind::TypeDef(definition) = &mut item.kind else {
                unreachable!()
            };
            body_walk::type_definition(definition, self)?;
            if self.item_replacements.len() != start {
                let replacements = &self.item_replacements[start..];
                for (_, target, budget) in replacements {
                    ilang_host::syntax::charge_tree(budget, &root, true).map_err(|error| {
                        Error::Resource {
                            error: error.into(),
                            macro_name: "type macro".into(),
                            call: root.clone(),
                            invocation_span: item.span.0,
                        }
                    })?;
                    let output = &self
                        .scoped
                        .origin(*target)
                        .expect("expanded type has an origin")
                        .syntax;
                    ilang_host::syntax::charge_tree(budget, output, true).map_err(|error| {
                        Error::Resource {
                            error: error.into(),
                            macro_name: "type macro".into(),
                            call: root.clone(),
                            invocation_span: item.span.0,
                        }
                    })?;
                }
                let spans = replacements
                    .iter()
                    .map(|(old, new, _)| (*old, *new))
                    .collect::<Vec<_>>();
                let rewritten = self.scoped.rewrite_occurrences(&root, root_span, &spans);
                self.scoped
                    .set_item_override(item.span.0, rewritten.clone(), root_span);
                self.item_overrides
                    .insert(item.span.0, (rewritten, root_span));
                self.item_replacements.truncate(start);
            }
        }
        Ok(Step::Finished(item))
    }
}

use ilang_host::syntax::{DefinitionAbort, DefinitionReply};
use ilang_syntax::lisp::definition_schedule::Mode;
use ilang_syntax::lisp::scoped::ScopedSource;
use ilang_syntax::lisp::syntax::Scope;
use ilang_syntax::span::Span;
use ilang_syntax::syn3::ast::Module;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// An identity is independent of a fresh answer's parameter identities.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefinitionIdentity {
    pub source: u32,
    pub scope: Scope,
    pub name: String,
    pub span: Span,
    pub builtin: bool,
}

/// A frozen view passed to a fresh macro run. `ready` is parallel to the
/// module's items and marks type definitions whose item-phase expansion ended.
#[derive(Clone)]
pub struct Snapshot {
    pub module: Module,
    pub ready: Vec<bool>,
    pub scoped: ScopedSource,
    pub source: u32,
    pub other_runs: bool,
    pub active: Option<usize>,
    pub epoch: u64,
    pub item_overrides: std::collections::HashMap<Span, (Syntax, Span)>,
}

pub struct Resolution {
    pub reply: DefinitionReply,
    /// A successful early answer is audited after all items are committed.
    pub audit: Option<DefinitionIdentity>,
}

/// Implemented by the package-graph definition lookup. A resolver receives
/// both the query while the invocation mark is present and the query after
/// that mark is flipped. It must use the same budget as the macro run.
pub trait Definitions: Send + Sync {
    fn fresh_answer(&self) -> u64;
    fn prepare(&self, _snapshot: &Snapshot) {}
    fn resolve(
        &self,
        snapshot: &Snapshot,
        in_run: &Syntax,
        result: &Syntax,
        mode: Mode,
        mark: u64,
        budget: &ilang_host::macro_budget::MacroBudget,
    ) -> Resolution;
}

#[derive(Debug)]
pub enum PhaseError {
    Step(Error),
    Waiting(Vec<Syntax>),
    Changed {
        query: Syntax,
        before: Box<DefinitionIdentity>,
        after: Option<Box<DefinitionIdentity>>,
    },
}

struct Slot {
    id: u64,
    item: Arc<Item>,
    ready: bool,
    depth: u16,
    root: Span,
    path: Vec<u32>,
}
/// Notes of successful item runs before a later phase failure.
#[derive(Clone, Debug)]
pub struct PhaseNote {
    pub mark: u64,
    pub origin: crate::session::ItemOrigin,
    pub message: String,
}
struct Audit {
    in_run: Syntax,
    result: Syntax,
    mark: u64,
    identity: DefinitionIdentity,
}

/// Expands a module's item phase with fresh attempts. The callback implements
/// hygienic package lookup; this driver enforces retry, ordering and atomicity.
/// Later phases consume transactions only after this function succeeds.
pub fn expand_items(
    engine: &mut BodyEngine<'_>,
    module: &mut Module,
    definitions: Arc<dyn Definitions>,
) -> Result<Vec<Transaction>, PhaseError> {
    ilang_syntax::syn3::deep::deep(|| expand_items_here(engine, module, definitions))
}

fn expand_items_here(
    engine: &mut BodyEngine<'_>,
    module: &mut Module,
    definitions: Arc<dyn Definitions>,
) -> Result<Vec<Transaction>, PhaseError> {
    engine.clear_failure_source();
    engine.phase_notes.clear();
    let original_source = engine.scoped.clone();
    let original_resolver = engine.resolver.clone();
    let original_transactions = engine.transactions.len();
    let original_overrides = engine.item_overrides.clone();
    let original_replacements = engine.item_replacements.clone();
    let result = expand_items_inner(engine, module, definitions);
    engine.resolver = original_resolver;
    if result.is_ok() {
        engine.phase_notes.clear();
    }
    if result.is_err() {
        if engine.failure_scoped().is_none() {
            engine.save_failure_source();
        }
        engine.scoped = original_source;
        engine.transactions.truncate(original_transactions);
        engine.item_overrides = original_overrides;
        engine.item_replacements = original_replacements;
    }
    result
}

fn expand_items_inner(
    engine: &mut BodyEngine<'_>,
    module: &mut Module,
    definitions: Arc<dyn Definitions>,
) -> Result<Vec<Transaction>, PhaseError> {
    let mut slots: Vec<Slot> = module
        .items
        .iter()
        .cloned()
        .enumerate()
        .map(|(id, item)| Slot {
            id: id as u64,
            root: item.span.0,
            path: vec![id as u32],
            item: Arc::new(item),
            ready: false,
            depth: 0,
        })
        .collect();
    for slot in &mut slots {
        slot.ready = type_ready(engine, &slot.item)?;
    }
    let mut next_id = slots.len() as u64;
    let mut pending: VecDeque<u64> = slots.iter().map(|s| s.id).collect();
    let mut mode = Mode::Normal;
    let mut deferred = 0usize;
    let mut waits: Vec<(u64, Syntax)> = Vec::new();
    let mut audits = Vec::new();
    let mut epoch = 0u64;
    let mut transactions = Vec::new();
    while let Some(id) = pending.pop_front() {
        let index = slots
            .iter()
            .position(|s| s.id == id)
            .expect("pending item remains in module");
        engine.item_locals = generated_values(engine, &slots);
        if item_is_finished_without_run(engine, &slots[index].item, slots[index].ready) {
            slots[index].ready = true;
            waits.retain(|(waiting, _)| *waiting != id);
            deferred = 0;
            mode = Mode::Normal;
            continue;
        }
        // Freeze the items by sharing their immutable trees. Most item macros
        // never query definitions, so materialize the lookup module only when
        // the run asks for it, while retaining exactly this attempt's view.
        let current = Module {
            doc: module.doc.clone(),
            imports: module.imports.clone(),
            exports: module.exports.clone(),
            items: Vec::new(),
        };
        let items: Vec<_> = slots.iter().map(|s| s.item.clone()).collect();
        let ready: Vec<_> = slots.iter().map(|s| s.ready).collect();
        // One copy of the source map serves both this attempt's frozen
        // definition view and the rollback of a deferred or failed attempt.
        let frozen = Arc::new(engine.scoped.clone());
        let scoped = frozen.clone();
        let source = engine.source.0;
        let other_runs = !pending.is_empty();
        let snapshot_epoch = epoch;
        let item_overrides = engine.item_overrides.clone();
        let snapshot = std::sync::OnceLock::new();
        epoch = epoch.checked_add(1).expect("item snapshot epoch exhausted");
        let mark_source = engine.run_mark.clone();
        let collected = Arc::new(Mutex::new(Vec::<Audit>::new()));
        let seen = collected.clone();
        let defs = definitions.clone();
        engine.resolver = Arc::new(move |query, budget| {
            let mark = mark_source.load(std::sync::atomic::Ordering::Relaxed);
            let result_query = query.flip_mark(mark);
            let snapshot = snapshot.get_or_init(|| {
                let mut module = current.clone();
                module.items = items.iter().map(|item| item.as_ref().clone()).collect();
                let snapshot = Snapshot {
                    module,
                    ready: ready.clone(),
                    scoped: scoped.as_ref().clone(),
                    source,
                    other_runs,
                    active: Some(index),
                    epoch: snapshot_epoch,
                    item_overrides: item_overrides.clone(),
                };
                defs.prepare(&snapshot);
                snapshot
            });
            let resolution = defs.resolve(snapshot, query, &result_query, mode, mark, budget);
            if let Some(identity) = resolution.audit {
                seen.lock()
                    .expect("definition audit lock poisoned")
                    .push(Audit {
                        in_run: query.clone(),
                        result: result_query,
                        mark,
                        identity,
                    });
            }
            resolution.reply
        });
        let original_origin = crate::session::ItemOrigin {
            source: engine.source.0,
            call: slots[index].root,
            path: slots[index].path.clone(),
        };
        let old_scoped = frozen;
        let old_transactions = engine.transactions.len();
        let old_overrides = engine.item_overrides.clone();
        let old_replacements = engine.item_replacements.clone();
        let attempt = engine.item_step(slots[index].item.as_ref().clone(), definitions.as_ref());
        match attempt {
            Ok(step) => {
                waits.retain(|(waiting, _)| *waiting != id);
                deferred = 0;
                mode = Mode::Normal;
                audits.extend(
                    collected
                        .lock()
                        .expect("definition audit lock poisoned")
                        .drain(..),
                );
                match step {
                    Step::Finished(item) => {
                        slots[index].item = Arc::new(item);
                        slots[index].ready = true;
                    }
                    Step::Expanded {
                        retained,
                        items,
                        transactions: effects,
                    } => {
                        let depth = slots[index]
                            .depth
                            .checked_add(1)
                            .expect("expansion depth overflow");
                        if depth > 256 {
                            return Err(PhaseError::Step(Error::Depth(slots[index].item.span.0)));
                        }
                        let mut replacements = Vec::new();
                        let root = slots[index].root;
                        let parent_path = slots[index].path.clone();
                        if let Some(item) = retained {
                            let ready = type_ready(engine, &item)?;
                            replacements.push(Slot {
                                id: next_id,
                                item: Arc::new(item),
                                ready,
                                depth,
                                root,
                                path: {
                                    let mut path = parent_path.clone();
                                    path.push(replacements.len() as u32);
                                    path
                                },
                            });
                            next_id += 1;
                        }
                        for item in items {
                            let ready = type_ready(engine, &item)?;
                            replacements.push(Slot {
                                id: next_id,
                                item: Arc::new(item),
                                ready,
                                depth,
                                root,
                                path: {
                                    let mut path = parent_path.clone();
                                    path.push(replacements.len() as u32);
                                    path
                                },
                            });
                            next_id += 1;
                        }
                        let new_ids: Vec<_> = replacements.iter().map(|s| s.id).collect();
                        slots.splice(index..=index, replacements);
                        for id in new_ids.into_iter().rev() {
                            pending.push_front(id);
                        }
                        transactions.extend(effects.into_iter().map(|mut transaction| {
                            let origin = crate::session::ItemOrigin {
                                source: engine.source.0,
                                call: root,
                                path: parent_path.clone(),
                            };
                            engine
                                .phase_notes
                                .extend(transaction.notes.iter().map(|message| PhaseNote {
                                    mark: transaction.mark,
                                    origin: origin.clone(),
                                    message: message.clone(),
                                }));
                            transaction.item_origin = Some(origin);
                            transaction
                        }));
                    }
                }
                for mut transaction in engine.transactions.drain(old_transactions..) {
                    engine
                        .phase_notes
                        .extend(transaction.notes.iter().map(|message| PhaseNote {
                            mark: transaction.mark,
                            origin: original_origin.clone(),
                            message: message.clone(),
                        }));
                    transaction.item_origin = Some(original_origin.clone());
                    transactions.push(transaction);
                }
            }
            Err(Error::Run {
                failure: crate::session::Failure::Aborted(DefinitionAbort::Deferred { at }),
                ..
            }) => {
                engine.scoped = Arc::unwrap_or_clone(old_scoped);
                engine.item_overrides = old_overrides;
                engine.item_replacements = old_replacements;
                engine.transactions.truncate(old_transactions);
                waits.retain(|(waiting, _)| *waiting != id);
                waits.push((id, at));
                pending.push_back(id);
                deferred += 1;
                if deferred == pending.len() {
                    if mode == Mode::Normal {
                        mode = Mode::Fallback;
                        deferred = 0;
                    } else {
                        return Err(PhaseError::Waiting(
                            pending
                                .iter()
                                .map(|id| {
                                    waits
                                        .iter()
                                        .find(|(waiting, _)| waiting == id)
                                        .expect("every pending item was deferred")
                                        .1
                                        .clone()
                                })
                                .collect(),
                        ));
                    }
                }
            }
            Err(error) => {
                engine.save_failure_source();
                engine.scoped = Arc::unwrap_or_clone(old_scoped);
                engine.item_overrides = old_overrides;
                engine.item_replacements = old_replacements;
                engine.transactions.truncate(old_transactions);
                return Err(PhaseError::Step(error));
            }
        }
    }
    let mut completed = Module {
        doc: module.doc.clone(),
        imports: module.imports.clone(),
        exports: module.exports.clone(),
        items: slots
            .into_iter()
            .map(|s| Arc::unwrap_or_clone(s.item))
            .collect(),
    };
    commit_exports(
        &mut completed,
        &transactions,
        engine.source.0,
        &engine.scoped,
    )?;
    let snapshot = Snapshot {
        module: completed.clone(),
        ready: vec![true; completed.items.len()],
        scoped: engine.scoped.clone(),
        source: engine.source.0,
        other_runs: false,
        active: None,
        epoch,
        item_overrides: engine.item_overrides.clone(),
    };
    definitions.prepare(&snapshot);
    for audit in audits {
        let budget = ilang_host::macro_budget::MacroBudget::default();
        let after = definitions
            .resolve(
                &snapshot,
                &audit.in_run,
                &audit.result,
                Mode::Fallback,
                audit.mark,
                &budget,
            )
            .audit;
        if after.as_ref() != Some(&audit.identity) {
            engine.phase_notes.retain(|note| note.mark != audit.mark);
            return Err(PhaseError::Changed {
                query: audit.result,
                before: Box::new(audit.identity),
                after: after.map(Box::new),
            });
        }
    }
    *module = completed;
    Ok(transactions)
}

fn item_is_finished_without_run(engine: &BodyEngine<'_>, item: &Item, ready: bool) -> bool {
    engine.item_is_finished_without_run3(item, ready)
}

impl BodyEngine<'_> {
    /// After the item barrier all module definitions are ready. Body/type
    /// expansion shares the same hygienic lookup without retrying item runs.
    pub fn use_completed_definitions(
        &mut self,
        definitions: Arc<dyn Definitions>,
        module: &Module,
    ) {
        let snapshot = Snapshot {
            module: module.clone(),
            ready: vec![true; module.items.len()],
            scoped: self.scoped.clone(),
            source: self.source.0,
            other_runs: false,
            active: None,
            epoch: u64::MAX,
            item_overrides: self.item_overrides.clone(),
        };
        definitions.prepare(&snapshot);
        let mark_source = self.run_mark.clone();
        self.resolver = Arc::new(move |query, budget| {
            let mark = mark_source.load(std::sync::atomic::Ordering::Relaxed);
            definitions
                .resolve(
                    &snapshot,
                    query,
                    &query.flip_mark(mark),
                    Mode::Fallback,
                    mark,
                    budget,
                )
                .reply
        });
    }
}

fn generated_values(engine: &BodyEngine<'_>, slots: &[Slot]) -> Vec<body_walk::Local> {
    engine.generated_values3(slots.iter().map(|slot| slot.item.as_ref()))
}
