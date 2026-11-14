//! Scope-aware index of written macro bindings and module imports.
use std::collections::HashMap;

use ilang_check::{Parsed, hygiene};
use ilang_package::{Diagnostic, ModuleKind, PackageGraph, SourceId};
use ilang_syntax::lisp::{
    expansion::MacroDefinition,
    expansion3 as expansion,
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
    /// The module's own bindings. They take precedence over imported names.
    values: HashMap<String, (Value, bool)>,
    generated: Vec<hygiene::Binding<Value>>,
    /// Import aliases and imported names, in import order; the first import of a
    /// name wins, and an own binding shadows all of them.
    imports: HashMap<String, (Value, bool)>,
}

impl Module {
    fn value(&self, text: &str) -> Option<&(Value, bool)> {
        self.values.get(text).or_else(|| self.imports.get(text))
    }
}

/// Names are indexed in the value namespace, including non-macro bindings:
/// these may shadow a macro or a module alias at the call site.
///
/// A module's own entry depends only on that module; its import layer depends
/// on its imports and on the exported own values of their targets. Expansion
/// changes one module at a time, so `update` recomputes that module's own entry
/// and every import layer, which gives exactly the index `new` would build.
pub struct Registry {
    modules: HashMap<u32, Module>,
}

impl Registry {
    pub fn new(graph: &PackageGraph, parsed: &[Parsed]) -> Result<Self, Vec<Diagnostic>> {
        let mut modules = HashMap::new();
        let mut errors = Vec::new();
        for p in parsed {
            match own(graph, p) {
                Ok(module) => {
                    modules.insert(p.source.0, module);
                }
                Err(es) => {
                    errors.extend(es);
                    modules.insert(p.source.0, Module::default());
                }
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut registry = Self { modules };
        registry.link(graph, parsed);
        Ok(registry)
    }

    /// Re-indexes `parsed[index]` after it changed, as if the whole registry
    /// were rebuilt from `parsed`. Other modules must be unchanged since this
    /// registry was built or last updated.
    pub fn update(
        &mut self,
        graph: &PackageGraph,
        parsed: &[Parsed],
        index: usize,
    ) -> Result<(), Vec<Diagnostic>> {
        let p = &parsed[index];
        let module = own(graph, p)?;
        self.modules.insert(p.source.0, module);
        self.link(graph, parsed);
        Ok(())
    }

    /// Rebuilds every module's import layer from the current own entries.
    fn link(&mut self, graph: &PackageGraph, parsed: &[Parsed]) {
        for p in parsed {
            let package = &graph.packages[p.package];
            let mut layer: HashMap<String, (Value, bool)> = HashMap::new();
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
                layer
                    .entry(alias.name.clone())
                    .or_insert((Value::Module(target.source), false));
                let target = &self.modules[&target.source.0];
                for n in &import.names {
                    if let Some((value, _)) =
                        target.values.get(&n.name).filter(|(_, exported)| *exported)
                    {
                        layer
                            .entry(n.name.clone())
                            .or_insert((value.clone(), false));
                    }
                }
            }
            self.modules
                .get_mut(&p.source.0)
                .expect("every parsed module is indexed")
                .imports = layer;
        }
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
                self.modules
                    .get(&caller.0)
                    .map_or(&[], |module| module.generated.as_slice()),
                |module, text| self
                    .modules
                    .get(&module)?
                    .value(text)
                    .map(|(target, exported)| hygiene::ModuleName {
                        target: target.clone(),
                        exported: *exported
                    }),
                |_| None,
                |_| None
            ),
            hygiene::Lookup::Found(Value::Module(_))
        )
    }

    /// Resolves a name in its source scope, checking exact lexical scopes first.
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
                self.modules
                    .get(&caller.0)
                    .map_or(&[], |module| module.generated.as_slice()),
                |module, text| {
                    self.modules
                        .get(&module)?
                        .value(text)
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
                    let (value, exported) = self.modules.get(&target.0)?.value(text)?;
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

/// The bindings and macros written in one module, without its imports.
fn own(graph: &PackageGraph, p: &Parsed) -> Result<Module, Vec<Diagnostic>> {
    let lisp = graph.sources.get(p.source).display.ends_with(".is");
    let defs = expansion::definitions(&p.ast, lisp).map_err(|es| {
        es.into_iter()
            .map(|e| {
                Diagnostic::error(
                    ilang_check::code::ATTRIBUTE_PLACE,
                    Some(p.source),
                    e.span,
                    e.message,
                )
            })
            .collect::<Vec<_>>()
    })?;
    let mut module = Module::default();
    for (name, span) in crate::registry3::value_names(&p.ast) {
        if let Some(origin) = p.scoped.as_ref().and_then(|scoped| scoped.origin(span))
            && !origin.syntax.scope().marks.is_empty()
        {
            module.generated.push(hygiene::Binding {
                text: name.to_owned(),
                scope: origin.syntax.scope().clone(),
                identity: origin.syntax.scope_identity().cloned(),
                target: Value::Other,
            });
            continue;
        }
        let exported = p.ast.exports.iter().any(|e| e.name.name == name);
        module
            .values
            .insert(name.to_owned(), (Value::Other, exported));
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
    Ok(module)
}
