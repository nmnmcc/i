//! Scope-aware index of written macro bindings and module imports.
use std::collections::HashMap;

use ilang_check::{Parsed, hygiene};
use ilang_package::{Diagnostic, ModuleKind, PackageGraph, SourceId};
use ilang_syntax::ast::{ItemKind, Pattern, PatternKind};
use ilang_syntax::lisp::{
    expansion::{self, MacroDefinition},
    syntax::{Node, Syntax},
};

#[derive(Clone, Debug)]
pub struct Macro {
    pub source: SourceId,
    pub definition: MacroDefinition,
}

#[derive(Clone)]
enum Value {
    Macro(Macro),
    Other,
    Module(SourceId),
}

#[derive(Default)]
struct Module {
    values: HashMap<String, (Value, bool)>,
}

/// Names are indexed in the value namespace, including non-macro bindings:
/// these may shadow a macro or a module alias at the call site.
pub struct Registry {
    modules: HashMap<u32, Module>,
}

impl Registry {
    pub fn new(graph: &PackageGraph, parsed: &[Parsed]) -> Result<Self, Vec<Diagnostic>> {
        let mut modules = HashMap::new();
        let mut errors = Vec::new();
        for p in parsed {
            let lisp = graph.sources.get(p.source).display.ends_with(".is");
            let defs = match expansion::definitions(&p.ast, lisp) {
                Ok(defs) => defs,
                Err(es) => {
                    errors.extend(es.into_iter().map(|e| {
                        Diagnostic::error(
                            ilang_check::code::ATTRIBUTE_PLACE,
                            Some(p.source),
                            e.span,
                            e.message,
                        )
                    }));
                    Vec::new()
                }
            };
            let mut module = Module::default();
            for item in &p.ast.items {
                if let ItemKind::Binding(binding) = &item.kind {
                    let mut names = Vec::new();
                    binding_names(&binding.pattern, &mut names);
                    for name in names {
                        let exported = p.ast.exports.iter().any(|e| e.name.name == name);
                        module
                            .values
                            .insert(name.to_owned(), (Value::Other, exported));
                    }
                }
            }
            for definition in defs {
                let exported = p.ast.exports.iter().any(|e| e.name.name == definition.name);
                module.values.insert(
                    definition.name.clone(),
                    (
                        Value::Macro(Macro {
                            source: p.source,
                            definition,
                        }),
                        exported,
                    ),
                );
            }
            modules.insert(p.source.0, module);
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        for p in parsed {
            let package = &graph.packages[p.package];
            for import in &p.ast.imports {
                let first = &import.path[0].name;
                let id = if first == "i" {
                    Some(graph.library)
                } else if *first == package.name {
                    Some(p.package)
                } else {
                    package
                        .dependencies
                        .iter()
                        .find(|(n, _)| n == first)
                        .and_then(|(_, id)| *id)
                        .or_else(|| {
                            (p.kind == ModuleKind::Test)
                                .then(|| {
                                    package
                                        .dev_dependencies
                                        .iter()
                                        .find(|(n, _)| n == first)
                                        .and_then(|(_, id)| *id)
                                })
                                .flatten()
                        })
                };
                let Some(id) = id else { continue };
                let tail = import
                    .path
                    .iter()
                    .skip(1)
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join("/");
                let path = format!("{}/{tail}", graph.packages[id].name);
                let Some(target) = parsed
                    .iter()
                    .find(|m| m.package == id && m.path == path && m.kind != ModuleKind::Prelude)
                else {
                    continue;
                };
                let alias = import
                    .alias
                    .as_ref()
                    .unwrap_or_else(|| import.path.last().unwrap());
                let imported = import
                    .names
                    .iter()
                    .filter_map(|n| {
                        modules[&target.source.0]
                            .values
                            .get(&n.name)
                            .filter(|(_, exported)| *exported)
                            .map(|(v, _)| (n.name.clone(), v.clone()))
                    })
                    .collect::<Vec<_>>();
                let module = modules.get_mut(&p.source.0).unwrap();
                module
                    .values
                    .entry(alias.name.clone())
                    .or_insert((Value::Module(target.source), false));
                for (name, value) in imported {
                    module.values.entry(name).or_insert((value, false));
                }
            }
        }
        Ok(Self { modules })
    }

    /// Whether this visible value is a module alias, after lexical shadowing.
    pub fn is_module(
        &self,
        caller: SourceId,
        atom: &Syntax,
        locals: &[hygiene::Binding<()>],
    ) -> bool {
        let locals = locals
            .iter()
            .map(|b| hygiene::Binding {
                text: b.text.clone(),
                scope: b.scope.clone(),
                identity: b.identity.clone(),
                target: Value::Other,
            })
            .collect::<Vec<_>>();
        matches!(
            hygiene::lookup(
                atom,
                caller.0,
                &locals,
                &[],
                |module, text| self.modules.get(&module)?.values.get(text).map(
                    |(target, exported)| hygiene::ModuleName {
                        target: target.clone(),
                        exported: *exported
                    }
                ),
                |_| None,
                |_| None
            ),
            hygiene::Lookup::Found(Value::Module(_))
        )
    }

    /// Resolve a name in its source scope, checking exact lexical scopes first.
    /// A module-qualified call resolves its alias by the same value lookup.
    pub fn resolve(
        &self,
        caller: SourceId,
        qualifier: Option<&Syntax>,
        name: &Syntax,
        locals: &[hygiene::Binding<()>],
    ) -> Option<Macro> {
        let locals = locals
            .iter()
            .map(|b| hygiene::Binding {
                text: b.text.clone(),
                scope: b.scope.clone(),
                identity: b.identity.clone(),
                target: Value::Other,
            })
            .collect::<Vec<_>>();
        let lookup = |atom: &Syntax| {
            hygiene::lookup(
                atom,
                caller.0,
                &locals,
                &[],
                |module, text| {
                    self.modules
                        .get(&module)?
                        .values
                        .get(text)
                        .map(|(target, exported)| hygiene::ModuleName {
                            target: target.clone(),
                            exported: *exported,
                        })
                },
                |_| None,
                |_| None,
            )
        };
        let value = if let Some(alias) = qualifier {
            match lookup(alias) {
                hygiene::Lookup::Found(Value::Module(target)) => {
                    let text = match name.node() {
                        Node::Name(text) | Node::Up(text) => text,
                        _ => return None,
                    };
                    let (value, exported) = self.modules.get(&target.0)?.values.get(text)?;
                    if !exported {
                        return None;
                    }
                    value.clone()
                }
                _ => return None,
            }
        } else {
            match lookup(name) {
                hygiene::Lookup::Found(v) => v,
                _ => return None,
            }
        };
        match value {
            Value::Macro(m) => Some(m),
            _ => None,
        }
    }
}

fn binding_names<'a>(p: &'a Pattern, out: &mut Vec<&'a str>) {
    match &p.kind {
        PatternKind::Var(n) => out.push(n),
        PatternKind::Typed { binder, .. } if !binder.discard => out.push(&binder.name),
        PatternKind::Named { name, pattern } => {
            out.push(&name.name);
            binding_names(pattern, out);
        }
        PatternKind::Constructor { args, .. } | PatternKind::Tuple(args) => {
            for p in args {
                binding_names(p, out)
            }
        }
        PatternKind::List { items, rest } => {
            for p in items {
                binding_names(p, out);
            }
            if let Some(Some(b)) = rest
                && !b.discard
            {
                out.push(&b.name);
            }
        }
        PatternKind::Prefix { rest, .. } if !rest.discard => out.push(&rest.name),
        PatternKind::BitArray(segments) => {
            for s in segments {
                binding_names(&s.value, out);
            }
        }
        _ => {}
    }
}
