//! Checked, hygienic collection manifests for tool-selected entry points.
use std::collections::{HashMap, HashSet};

use ilang_check::{
    Parsed,
    hir::{DefId, DefKind, ExprKind, ModuleId, Param, Program},
    hygiene,
    types::Typed,
};
use ilang_host::syntax::ItemRequest;
use ilang_package::{Diagnostic, SourceId};
use ilang_syntax::{
    lisp::syntax::{Node, Syntax},
    span::Span,
};

use crate::session::{ItemOrigin, Transaction};

#[derive(Clone, Debug)]
pub struct Entry {
    pub collection: DefId,
    pub target: DefId,
    pub data: Vec<(String, String)>,
    pub signature: String,
    pub target_span: Span,
    pub target_origin: Option<ilang_syntax::lisp::scoped::Origin>,
    pub call_chain: Vec<ilang_syntax::lisp::scoped::Invocation>,
    pub at: Syntax,
    pub origin: ItemOrigin,
    pub ordinal: u32,
}

/// Resolves after successful type checking, before consumers select roots.
pub fn resolve(
    parsed: &[Parsed],
    program: &Program,
    typed: &Typed,
    transactions: &[Transaction],
) -> Result<Vec<Entry>, Vec<Diagnostic>> {
    resolve_with_types(parsed, program, Some(typed), transactions)
}

pub fn resolve_with_types(
    parsed: &[Parsed],
    program: &Program,
    typed: Option<&Typed>,
    transactions: &[Transaction],
) -> Result<Vec<Entry>, Vec<Diagnostic>> {
    let by_source: HashMap<u32, ModuleId> = program
        .modules
        .iter()
        .enumerate()
        .map(|(i, m)| (m.source.0, ModuleId(i as u32)))
        .collect();
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for transaction in transactions {
        let Some(origin) = &transaction.item_origin else {
            continue;
        };
        let caller = by_source[&origin.source];
        for (ordinal, request) in transaction.requests.iter().enumerate() {
            let ItemRequest::Add {
                collection,
                target,
                data,
                at,
            } = request
            else {
                continue;
            };
            let report = |node: &Syntax, msg: &str| {
                let scoped = parsed
                    .iter()
                    .find(|p| p.source.0 == origin.source)
                    .and_then(|p| p.scoped.as_ref());
                let span = if node.scope().module == origin.source
                    && scoped.is_some_and(|s| node.span().end <= s.original_len())
                {
                    node.span()
                } else {
                    origin.call
                };
                Diagnostic::error(
                    ilang_check::code::MACRO_RESERVED,
                    Some(SourceId(origin.source)),
                    span,
                    msg,
                )
            };
            let Some(collection_id) = lookup(parsed, program, &by_source, caller, collection)
            else {
                errors.push(report(
                    collection,
                    "collection must resolve to an exported ordinary binding",
                ));
                continue;
            };
            if !program.def(collection_id).exported
                || !matches!(program.def(collection_id).kind, DefKind::Binding { .. })
            {
                errors.push(report(
                    collection,
                    "collection must resolve to an exported ordinary binding",
                ));
                continue;
            }
            let Some(target_id) = lookup(parsed, program, &by_source, caller, target) else {
                errors.push(report(
                    target,
                    "collection target must resolve to a function in this module",
                ));
                continue;
            };
            let DefKind::Binding { binding, .. } = program.def(target_id).kind else {
                errors.push(report(
                    target,
                    "collection target must be a direct function",
                ));
                continue;
            };
            let top = &program.module(target_id.module).bindings[binding as usize];
            let function = match &top.binding.value.kind {
                ExprKind::Fn(f) => Some(f),
                _ => None,
            };
            if target_id.module != caller
                || !top.binding.direct
                || top.defs.len() != 1
                || !function
                    .is_some_and(|f| f.params.iter().all(|p| matches!(p, Param::Env { .. })))
            {
                errors.push(report(target, "collection target must be a same-module direct function with no ordinary parameters"));
                continue;
            }
            if !seen.insert((collection_id, target_id)) {
                errors.push(report(target, "duplicate collection target"));
                continue;
            }
            entries.push(Entry {
                collection: collection_id,
                target: target_id,
                data: data.clone(),
                signature: typed
                    .map(|t| {
                        t.schemes
                            .get(&target_id)
                            .expect("checked target has a type scheme")
                            .clone()
                    })
                    .unwrap_or_default(),
                target_span: program.def(target_id).span,
                target_origin: parsed
                    .iter()
                    .find(|p| p.source.0 == origin.source)
                    .and_then(|p| p.scoped.as_ref())
                    .and_then(|s| s.origin(program.def(target_id).span))
                    .cloned(),
                call_chain: parsed
                    .iter()
                    .find(|p| p.source.0 == origin.source)
                    .and_then(|p| p.scoped.as_ref())
                    .and_then(|s| transaction.location.and_then(|(_, span)| s.origin(span)))
                    .map_or_else(Vec::new, |o| o.call_chain.clone()),
                at: at.clone(),
                origin: origin.clone(),
                ordinal: ordinal as u32,
            });
        }
    }
    entries.sort_by(|a, b| {
        program
            .module(a.target.module)
            .path
            .cmp(&program.module(b.target.module).path)
            .then(a.origin.call.start.cmp(&b.origin.call.start))
            .then(a.origin.path.cmp(&b.origin.path))
            .then(a.ordinal.cmp(&b.ordinal))
    });
    if errors.is_empty() {
        Ok(entries)
    } else {
        Err(errors)
    }
}

fn lookup(
    parsed: &[Parsed],
    program: &Program,
    by_source: &HashMap<u32, ModuleId>,
    caller: ModuleId,
    syntax: &Syntax,
) -> Option<DefId> {
    let (qualifier, name) = match syntax.node() {
        Node::Name(_) => (None, syntax),
        Node::Round(parts)
            if parts.len() == 3 && matches!(parts[0].node(), Node::Symbol(s) if s == ".") =>
        {
            (Some(&parts[1]), &parts[2])
        }
        _ => return None,
    };
    #[derive(Clone)]
    enum Target {
        Def(DefId),
        Module(ModuleId),
    }
    let mut generated = Vec::new();
    for (module_index, module) in program.modules.iter().enumerate() {
        let Some(p) = parsed.iter().find(|p| p.source == module.source) else {
            continue;
        };
        for (index, def) in module.defs.iter().enumerate() {
            if let Some(origin) = p.scoped.as_ref().and_then(|s| s.origin(def.span))
                && !origin.syntax.scope().marks.is_empty()
            {
                generated.push(hygiene::Binding {
                    text: def.name.clone(),
                    scope: origin.syntax.scope().clone(),
                    identity: origin.syntax.scope_identity().cloned(),
                    target: Target::Def(DefId {
                        module: ModuleId(module_index as u32),
                        index: index as u32,
                    }),
                });
            }
        }
    }
    let module_lookup = |scope: u32, text: &str| -> Option<hygiene::ModuleName<Target>> {
        let id = *by_source.get(&scope)?;
        let module = program.module(id);
        if let Some((index, def)) = module
            .defs
            .iter()
            .enumerate()
            .find(|(_, d)| d.name == text && !p_is_marked(parsed, module.source, d.span))
        {
            return Some(hygiene::ModuleName {
                target: Target::Def(DefId {
                    module: id,
                    index: index as u32,
                }),
                exported: def.exported,
            });
        }
        let p = parsed.iter().find(|p| p.source == module.source)?;
        for import in &p.ast.imports {
            if !import.names.iter().any(|n| n.name == text) {
                continue;
            }
            let alias = import
                .alias
                .as_ref()
                .unwrap_or_else(|| import.path.last().expect("import path"));
            let imported = module
                .import_names
                .iter()
                .find(|(name, _)| name == &alias.name)?
                .1;
            if let Some((index, _def)) = program
                .module(imported)
                .defs
                .iter()
                .enumerate()
                .find(|(_, d)| d.name == text && d.exported)
            {
                return Some(hygiene::ModuleName {
                    target: Target::Def(DefId {
                        module: imported,
                        index: index as u32,
                    }),
                    exported: true,
                });
            }
        }
        module
            .import_names
            .iter()
            .find(|(name, _)| name == text)
            .map(|(_, id)| hygiene::ModuleName {
                target: Target::Module(*id),
                exported: false,
            })
    };
    let resolve_name = |atom: &Syntax| {
        hygiene::lookup(
            atom,
            program.module(caller).source.0,
            &[],
            &generated,
            module_lookup,
            |_| None,
            |_| None,
        )
    };
    if let Some(qualifier) = qualifier {
        let hygiene::Lookup::Found(Target::Module(imported)) = resolve_name(qualifier) else {
            return None;
        };
        let Node::Name(text) = name.node() else {
            return None;
        };
        let (index, def) = program
            .module(imported)
            .defs
            .iter()
            .enumerate()
            .find(|(_, d)| d.name == *text && d.exported)?;
        let _ = def;
        return Some(DefId {
            module: imported,
            index: index as u32,
        });
    }
    match resolve_name(name) {
        hygiene::Lookup::Found(Target::Def(id)) => Some(id),
        _ => None,
    }
}
fn p_is_marked(parsed: &[Parsed], source: SourceId, span: Span) -> bool {
    parsed
        .iter()
        .find(|p| p.source == source)
        .and_then(|p| p.scoped.as_ref())
        .and_then(|s| s.origin(span))
        .is_some_and(|o| !o.syntax.scope().marks.is_empty())
}
