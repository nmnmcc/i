//! The item phase of syntax expansion.
//!
//! One step leaves all arguments untouched until the selected outermost macro
//! has run. The scheduler installs a successful result only after parsing it.
use crate::{
    body_engine::{BodyEngine, Error},
    body_walk::{self, Hook},
    prepare::MacroPosition,
    session::Transaction,
};
use ilang_syntax::ast::{Attribute, Item, ItemKind, Pattern, PatternKind};
use ilang_syntax::lisp::{
    Datum, DatumKind,
    expansion::MacroArgument,
    source_syntax::source_item_syntax,
    syntax::{Node, Syntax},
};

/// A step either finishes an ordinary item or replaces it with the returned
/// items. `retained` is the written type definition protected by §4.3.
pub enum Step {
    Finished(Item),
    Expanded {
        retained: Option<Item>,
        items: Vec<Item>,
        transactions: Vec<Transaction>,
    },
}

fn builtin(attribute: &Attribute) -> bool {
    attribute.module.is_none()
        && matches!(
            attribute.name.name.as_str(),
            "external" | "js" | "host" | "deprecated" | "macro" | "main" | "test" | "wasm"
        )
}

fn names(pattern: &Pattern, output: &mut Vec<String>) {
    match &pattern.kind {
        PatternKind::Var(name) => output.push(name.clone()),
        PatternKind::Typed { binder, .. } if !binder.discard => output.push(binder.name.clone()),
        PatternKind::Named { name, pattern } => {
            output.push(name.name.clone());
            names(pattern, output);
        }
        PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
            for arg in args {
                names(arg, output);
            }
        }
        PatternKind::List { items, rest } => {
            for item in items {
                names(item, output);
            }
            if let Some(Some(binder)) = rest
                && !binder.discard
            {
                output.push(binder.name.clone());
            }
        }
        PatternKind::Prefix { rest, .. } if !rest.discard => output.push(rest.name.clone()),
        _ => {}
    }
}
fn item_names(item: &Item) -> Vec<String> {
    match &item.kind {
        ItemKind::TypeDef(t) => vec![t.name.name.clone()],
        ItemKind::Declaration { name, .. } => vec![name.name.clone()],
        ItemKind::Binding(b) => {
            let mut out = Vec::new();
            names(&b.pattern, &mut out);
            out
        }
        ItemKind::MacroItem { .. } => Vec::new(),
    }
}

impl BodyEngine<'_> {
    fn item_root(&self, item: &Item) -> (Syntax, Span) {
        if let Some(root) = self.item_overrides.get(&item.span) {
            return root.clone();
        }
        if let Some(origin) = self.scoped.origin(item.span) {
            return (origin.syntax.clone(), item.span);
        }
        let file = self.graph.sources.get(self.source);
        (
            source_item_syntax(
                item,
                &self.scope(),
                &file.text,
                file.display.ends_with(".is"),
            ),
            item.span,
        )
    }
    fn item_syntax(&self, item: &Item) -> Syntax {
        self.item_root(item).0
    }

    fn attribute_arguments(&self, attribute: &Attribute) -> Vec<MacroArgument> {
        if let Some((args, _)) = &attribute.call {
            args.iter()
                .map(|arg| MacroArgument {
                    label: arg.label.as_ref().map(|label| label.name.clone()),
                    syntax: self.expr_syntax(&arg.value),
                })
                .collect()
        } else {
            attribute
                .args
                .iter()
                .map(|(text, span)| MacroArgument {
                    label: None,
                    syntax: Syntax::from_datum(
                        &Datum {
                            kind: DatumKind::String(text.clone()),
                            span: *span,
                            docs: Vec::new(),
                        },
                        &self.scope(),
                    ),
                })
                .collect()
        }
    }

    /// Expand one outer item call, or one outer attribute call. A written type
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
            if let Some(mac) = self.resolve(
                attribute.module.as_ref(),
                &attribute.name,
                &self.item_locals,
                0,
            ) {
                if let Some(previous) = item.attributes[..index].iter().find(|a| builtin(a)) {
                    return Err(Error::InvalidItem(
                        previous.span,
                        "a built-in attribute cannot precede a macro".into(),
                    ));
                }
                if index != 0 {
                    return Err(Error::InvalidItem(
                        item.attributes[0].span,
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
                let args = self.attribute_arguments(attribute);
                let call = self.atom(&attribute.name);
                let original_names = item_names(&item);
                let is_type = matches!(item.kind, ItemKind::TypeDef(_));
                let (result, budget, transactions) =
                    self.execute(&mac, args, &call, Some(inner.clone()), MacroPosition::Item)?;
                ilang_host::syntax::charge_tree(&budget, &result, true)
                    .map_err(|e| Error::Run(crate::session::Failure::Host(e)))?;
                let result =
                    ilang_syntax::lisp::definitions::freshen_duplicate_heads(&result, || {
                        definitions.fresh_answer()
                    });
                let mut items = self
                    .scoped
                    .items_metered(&result, &call, |work, words| {
                        budget.charge(work, words).map_err(|e| Box::new(e) as _)
                    })
                    .map_err(Error::Metered)?;
                reject_macro_definitions(&items)?;
                if !is_type && !item.doc.is_empty() {
                    if let Some(first) = items.iter_mut().find(|out| {
                        item_names(out)
                            .iter()
                            .any(|name| original_names.contains(name))
                    }) {
                        if first.doc.is_empty() {
                            first.doc.clone_from(&item.doc);
                        }
                    }
                }
                let retained = if is_type {
                    let root_span = self.item_root(&item).1;
                    let inner_span = self.scoped.child_span(root_span, 2).unwrap_or(inner.span());
                    self.item_overrides.insert(item.span, (inner, inner_span));
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
        if let ItemKind::MacroItem { module, name, args } = &item.kind {
            if let Some(mac) = self.resolve(module.as_ref(), name, &self.item_locals, 0) {
                let args = args
                    .iter()
                    .map(|arg| MacroArgument {
                        label: arg.label.as_ref().map(|label| label.name.clone()),
                        syntax: self.expr_syntax(&arg.value),
                    })
                    .collect();
                let call = self.item_syntax(&item);
                let (result, budget, transactions) =
                    self.execute(&mac, args, &call, None, MacroPosition::Item)?;
                ilang_host::syntax::charge_tree(&budget, &result, true)
                    .map_err(|e| Error::Run(crate::session::Failure::Host(e)))?;
                let result =
                    ilang_syntax::lisp::definitions::freshen_duplicate_heads(&result, || {
                        definitions.fresh_answer()
                    });
                let items = self
                    .scoped
                    .items_metered(&result, &call, |work, words| {
                        budget.charge(work, words).map_err(|e| Box::new(e) as _)
                    })
                    .map_err(Error::Metered)?;
                reject_macro_definitions(&items)?;
                return Ok(Step::Expanded {
                    retained: None,
                    items,
                    transactions,
                });
            }
            return Err(Error::InvalidItem(
                item.span,
                "an item call must name a macro".into(),
            ));
        }
        if matches!(item.kind, ItemKind::TypeDef(_)) {
            let (root, root_span) = self.item_root(&item);
            let start = self.type_replacements.len();
            let ItemKind::TypeDef(definition) = &mut item.kind else {
                unreachable!()
            };
            body_walk::type_definition(definition, self)?;
            if self.type_replacements.len() != start {
                let replacements = &self.type_replacements[start..];
                for (_, target, budget) in replacements {
                    ilang_host::syntax::charge_tree(budget, &root, true)
                        .map_err(|e| Error::Run(crate::session::Failure::Host(e)))?;
                    let output = &self
                        .scoped
                        .origin(*target)
                        .expect("expanded type has an origin")
                        .syntax;
                    ilang_host::syntax::charge_tree(budget, output, true)
                        .map_err(|e| Error::Run(crate::session::Failure::Host(e)))?;
                }
                let spans = replacements
                    .iter()
                    .map(|(old, new, _)| (*old, *new))
                    .collect::<Vec<_>>();
                let rewritten = self.scoped.rewrite_occurrences(&root, root_span, &spans);
                self.item_overrides
                    .insert(item.span, (rewritten, root_span));
                self.type_replacements.truncate(start);
            }
        }
        Ok(Step::Finished(item))
    }
}

fn reject_macro_definitions(items: &[Item]) -> Result<(), Error> {
    for item in items {
        if let Some(attribute) = item
            .attributes
            .iter()
            .find(|a| a.module.is_none() && a.name.name == "macro")
        {
            return Err(Error::InvalidItem(
                attribute.span,
                "an expansion cannot define a macro".into(),
            ));
        }
    }
    Ok(())
}

use ilang_host::syntax::{DefinitionAbort, DefinitionReply};
use ilang_syntax::ast::Module;
use ilang_syntax::lisp::definition_schedule::Mode;
use ilang_syntax::lisp::scoped::ScopedSource;
use ilang_syntax::lisp::syntax::Scope;
use ilang_syntax::span::Span;
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
        before: DefinitionIdentity,
        after: Option<DefinitionIdentity>,
    },
}

struct Slot {
    id: u64,
    item: Item,
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

/// Expand a module's item phase with fresh attempts. The callback implements
/// hygienic package lookup; this driver enforces retry, ordering and atomicity.
/// Later phases consume transactions only after this function succeeds.
pub fn expand_items(
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
    let original_replacements = engine.type_replacements.clone();
    let result = expand_items_inner(engine, module, definitions);
    engine.resolver = original_resolver;
    if result.is_ok() { engine.phase_notes.clear(); }
    if result.is_err() {
        if engine.failure_scoped().is_none() {
            engine.save_failure_source();
        }
        engine.scoped = original_source;
        engine.transactions.truncate(original_transactions);
        engine.item_overrides = original_overrides;
        engine.type_replacements = original_replacements;
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
            root: item.span,
            path: vec![id as u32],
            item,
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
        let mut current = module.clone();
        current.items = slots.iter().map(|s| s.item.clone()).collect();
        let snapshot = Snapshot {
            module: current,
            ready: slots.iter().map(|s| s.ready).collect(),
            scoped: engine.scoped.clone(),
            source: engine.source.0,
            other_runs: !pending.is_empty(),
            active: Some(index),
            epoch,
            item_overrides: engine.item_overrides.clone(),
        };
        epoch = epoch.checked_add(1).expect("item snapshot epoch exhausted");
        definitions.prepare(&snapshot);
        let mark_source = engine.run_mark.clone();
        let collected = Arc::new(Mutex::new(Vec::<Audit>::new()));
        let seen = collected.clone();
        let defs = definitions.clone();
        engine.resolver = Arc::new(move |query, budget| {
            let mark = mark_source.load(std::sync::atomic::Ordering::Relaxed);
            let result_query = query.flip_mark(mark);
            let resolution = defs.resolve(&snapshot, query, &result_query, mode, mark, budget);
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
        let old_scoped = engine.scoped.clone();
        let old_transactions = engine.transactions.len();
        let attempt = engine.item_step(slots[index].item.clone(), definitions.as_ref());
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
                        slots[index].item = item;
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
                            return Err(PhaseError::Step(Error::Depth(slots[index].item.span)));
                        }
                        let mut replacements = Vec::new();
                        let root = slots[index].root;
                        let parent_path = slots[index].path.clone();
                        if let Some(item) = retained {
                            let ready = type_ready(engine, &item)?;
                            replacements.push(Slot {
                                id: next_id,
                                item,
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
                                item,
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
                            let origin = crate::session::ItemOrigin { source: engine.source.0, call: root, path: parent_path.clone() };
                            engine.phase_notes.extend(transaction.notes.iter().map(|message| PhaseNote { mark: transaction.mark, origin: origin.clone(), message: message.clone() }));
                            transaction.item_origin = Some(origin);
                            transaction
                        }));
                    }
                }
                for mut transaction in engine.transactions.drain(old_transactions..) {
                    engine.phase_notes.extend(transaction.notes.iter().map(|message| PhaseNote { mark: transaction.mark, origin: original_origin.clone(), message: message.clone() }));
                    transaction.item_origin = Some(original_origin.clone());
                    transactions.push(transaction);
                }
            }
            Err(Error::Run(crate::session::Failure::Aborted(DefinitionAbort::Deferred { at }))) => {
                engine.scoped = old_scoped;
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
                engine.scoped = old_scoped;
                engine.transactions.truncate(old_transactions);
                return Err(PhaseError::Step(error));
            }
        }
    }
    let mut completed = module.clone();
    completed.items = slots.into_iter().map(|s| s.item).collect();
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
                before: audit.identity,
                after,
            });
        }
    }
    *module = completed;
    Ok(transactions)
}

/// A written type is visible immediately, and is ready as soon as none of
/// its type positions names a macro; no macro is executed by this probe.
fn type_ready(engine: &mut BodyEngine<'_>, item: &Item) -> Result<bool, PhaseError> {
    let ItemKind::TypeDef(definition) = &item.kind else {
        return Ok(false);
    };
    struct Probe<'a, 'b>(&'a mut BodyEngine<'b>);
    enum Found {
        Macro,
        Error(Error),
    }
    impl Hook for Probe<'_, '_> {
        type Error = Found;
        fn is_macro(
            &mut self,
            _: &ilang_syntax::ast::Expr,
            _: &[body_walk::Local],
            _: u32,
        ) -> Result<bool, Found> {
            Ok(false)
        }
        fn expand(
            &mut self,
            _: &ilang_syntax::ast::Expr,
            _: &[body_walk::Local],
            _: u32,
        ) -> Result<ilang_syntax::ast::Expr, Found> {
            unreachable!("type probe does not expand expressions")
        }
        fn is_type_macro(
            &mut self,
            ty: &ilang_syntax::ast::TypeExpr,
            locals: &[body_walk::Local],
            depth: u32,
        ) -> Result<bool, Found> {
            match self.0.is_type_macro(ty, locals, depth) {
                Ok(true) => Err(Found::Macro),
                Ok(false) => Ok(false),
                Err(error) => Err(Found::Error(error)),
            }
        }
        fn expand_type(
            &mut self,
            _: &ilang_syntax::ast::TypeExpr,
            _: &[body_walk::Local],
            _: u32,
        ) -> Result<ilang_syntax::ast::TypeExpr, Found> {
            unreachable!("type probe stops before expansion")
        }
        fn depth_error(&mut self, span: Span) -> Found {
            Found::Error(Error::Depth(span))
        }
        fn is_direct(
            &mut self,
            _: &ilang_syntax::ast::Expr,
            _: &[body_walk::Local],
            _: u32,
        ) -> bool {
            false
        }
    }
    match body_walk::type_definition(&mut definition.clone(), &mut Probe(engine)) {
        Ok(()) => Ok(true),
        Err(Found::Macro) => Ok(false),
        Err(Found::Error(error)) => Err(PhaseError::Step(error)),
    }
}

fn item_is_finished_without_run(engine: &BodyEngine<'_>, item: &Item, ready: bool) -> bool {
    if matches!(item.kind, ItemKind::MacroItem { .. }) {
        return false;
    }
    if matches!(item.kind, ItemKind::TypeDef(_)) && !ready {
        return false;
    }
    !item.attributes.iter().any(|a| {
        !builtin(a)
            && engine
                .resolve(a.module.as_ref(), &a.name, &engine.item_locals, 0)
                .is_some()
    })
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
    fn binder(pattern: &Pattern, out: &mut Vec<body_walk::Local>, scoped: &ScopedSource) {
        let mut add = |name: &str, span: Span| {
            if scoped.is_generated(span) {
                out.push(body_walk::Local {
                    name: name.to_owned(),
                    span,
                    fn_only: None,
                });
            }
        };
        match &pattern.kind {
            PatternKind::Var(name) => add(name, pattern.span),
            PatternKind::Typed { binder, .. } if !binder.discard => add(&binder.name, binder.span),
            PatternKind::Named { name, pattern } => {
                add(&name.name, name.span);
                binder(pattern, out, scoped);
            }
            PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
                for arg in args {
                    binder(arg, out, scoped);
                }
            }
            PatternKind::List { items, rest } => {
                for item in items {
                    binder(item, out, scoped);
                }
                if let Some(Some(b)) = rest
                    && !b.discard
                    && scoped.is_generated(b.span)
                {
                    out.push(body_walk::Local {
                        name: b.name.clone(),
                        span: b.span,
                        fn_only: None,
                    });
                }
            }
            PatternKind::Prefix { rest, .. } if !rest.discard => add(&rest.name, rest.span),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for slot in slots {
        match &slot.item.kind {
            ItemKind::Binding(binding) => binder(&binding.pattern, &mut out, &engine.scoped),
            ItemKind::Declaration { name, .. } if engine.scoped.is_generated(name.span) => out
                .push(body_walk::Local {
                    name: name.name.clone(),
                    span: name.span,
                    fn_only: None,
                }),
            _ => {}
        }
    }
    out
}

/// Export requests are checked only against the completed item phase. They do
/// not mutate the import graph and cannot publish a marked or foreign name.
fn commit_exports(
    module: &mut Module,
    transactions: &[Transaction],
    source: u32,
    scoped: &ScopedSource,
) -> Result<(), PhaseError> {
    for request in transactions
        .iter()
        .flat_map(|transaction| &transaction.requests)
    {
        let ilang_host::syntax::ItemRequest::Expose { name } = request else {
            continue;
        };
        let Node::Name(text) = name.node() else {
            return Err(PhaseError::Step(Error::InvalidItem(
                name.span(),
                "an export request needs a value name".into(),
            )));
        };
        if name.scope().module != source
            || !name.scope().marks.is_empty()
            || name.scope_identity().is_some()
        {
            return Err(PhaseError::Step(Error::InvalidItem(
                name.span(),
                "an export request needs an unmarked name in this module".into(),
            )));
        }
        let exists = module.items.iter().any(|item| {
            item_names(item).iter().any(|candidate| candidate == text)
                && match &item.kind {
                    ItemKind::Binding(binding) => {
                        unmarked_binder(&binding.pattern, text, source, scoped)
                    }
                    ItemKind::Declaration { name, .. } => {
                        name.name == *text
                            && scoped.origin(name.span).is_none_or(|origin| {
                                origin.syntax.scope().module == source
                                    && origin.syntax.scope().marks.is_empty()
                            })
                    }
                    _ => false,
                }
        });
        if !exists {
            return Err(PhaseError::Step(Error::InvalidItem(
                name.span(),
                format!("the requested export `{text}` is not defined in this module"),
            )));
        }
        if !module
            .exports
            .iter()
            .any(|export| export.name.name == *text)
        {
            module.exports.push(ilang_syntax::ast::ExportItem {
                name: ilang_syntax::ast::Ident {
                    name: text.clone(),
                    span: name.span(),
                },
                constructors: false,
                span: name.span(),
            });
        }
    }
    Ok(())
}
fn unmarked_binder(pattern: &Pattern, wanted: &str, source: u32, scoped: &ScopedSource) -> bool {
    let matches = |text: &str, span: Span| {
        text == wanted
            && scoped.origin(span).is_none_or(|origin| {
                origin.syntax.scope().module == source && origin.syntax.scope().marks.is_empty()
            })
    };
    match &pattern.kind {
        PatternKind::Var(name) => matches(name, pattern.span),
        PatternKind::Typed { binder, .. } => !binder.discard && matches(&binder.name, binder.span),
        PatternKind::Named { name, pattern } => {
            matches(&name.name, name.span) || unmarked_binder(pattern, wanted, source, scoped)
        }
        PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => args
            .iter()
            .any(|arg| unmarked_binder(arg, wanted, source, scoped)),
        PatternKind::List { items, rest } => {
            items
                .iter()
                .any(|arg| unmarked_binder(arg, wanted, source, scoped))
                || rest
                    .as_ref()
                    .and_then(|x| x.as_ref())
                    .is_some_and(|binder| !binder.discard && matches(&binder.name, binder.span))
        }
        PatternKind::Prefix { rest, .. } => !rest.discard && matches(&rest.name, rest.span),
        _ => false,
    }
}
