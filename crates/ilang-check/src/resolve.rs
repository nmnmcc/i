//! Name resolution (§2, §5, §6.10, §6.11, §7.2, §10): builds the HIR and
//! reports every name rule as a compile error.
//!
//! The work is done in phases, each over all modules:
//! 1. collect the top-level definitions, attributes, and exports;
//! 2. resolve the imports, and find import circles;
//! 3. resolve the type definitions, and find alias circles;
//! 4. resolve the declarations and bindings;
//! 5. check that exported definitions use only exported types (§2.4);
//! 6. check the rules about the whole program (entries, host services,
//!    `[wasm] exports`).

type Parsed = crate::Parsed;
use crate::code;
use crate::hir::*;
use ilang_syntax::syn3::ast::resolved::Ident;
use crate::types::{Head, HostType, ShapeId, Ty};
use ilang_package::{Diagnostic, ModuleKind, PackageGraph, PackageId};

use ilang_syntax::lisp::syntax::{Provenance, Scope as SyntaxScope, ScopeIdentity};
use ilang_syntax::span::Span;
use std::collections::BTreeSet;
use std::collections::{HashMap, HashSet};


/// Names and spans needed by shared import/export rules; no syntax nodes.
struct SourceImport { alias: Option<Ident>, names: Vec<Ident>, path: Vec<Ident>, span: Span }
struct SourceExport { name: Ident, constructors: bool, span: Span }
#[cfg(test)]
#[allow(dead_code)]
mod legacy;
#[cfg(test)]
mod differential;
mod three;
pub use three::lowering_errors;
pub(crate) use three::source_facts;

/// Resolve directly from the native tree. Source identity and origins are
/// shared with the transition resolver; no old syntax nodes are constructed.
pub fn resolve(graph: &PackageGraph, parsed: &[crate::Parsed]) -> (Program, Vec<Diagnostic>) {
    ilang_syntax::syn3::deep::deep(|| {
        let modules: Vec<_> = parsed.iter().map(|p| p.ast.clone()).collect();
        resolve_native(graph, parsed, &modules, false)
    })
}

pub fn resolve3(
    graph: &PackageGraph,
    parsed: &[crate::Parsed],
    modules: &[ilang_syntax::syn3::ast::Module],
) -> (Program, Vec<Diagnostic>) {
    resolve_native(graph, parsed, modules, true)
}
fn resolve_native(
    graph: &PackageGraph,
    parsed: &[crate::Parsed],
    modules: &[ilang_syntax::syn3::ast::Module],
    report_errors: bool,
) -> (Program, Vec<Diagnostic>) {
    ilang_syntax::syn3::deep::deep(|| {
        let mut transitions: Vec<_> = parsed
            .iter()
            .map(|p| {
                let mut t = p.lowering.clone();
                if let Some(scoped) = &p.scoped {
                    t.extend(scoped.transition());
                }
                t
            })
            .collect();
        let modules: Vec<_> = modules
            .iter()
            .zip(&mut transitions)
            .map(|(m, t)| three::normalize_module(m, t))
            .collect();
        three::resolve3(graph, parsed, &modules, &transitions, report_errors)
    })
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Def(DefId),
    /// A compile-time macro occupies the value namespace but is not a value.
    Macro {
        exported: bool,
    },
    Ctor(CtorRef),
    Module(ModuleId),
    /// A name from an import that failed; the problem has been reported,
    /// so its uses are not reported again.
    Broken,
}

/// The top-level names of a module: its own and the imported ones. The
/// `bool` says that the name is imported.
#[derive(Clone, Debug, Default)]
struct Scope {
    values: HashMap<String, (Entry, bool)>,
    types: HashMap<String, (TypeRef, bool)>,
    /// Type names from imports that failed.
    broken_types: HashSet<String>,
}

/// What an item of a module became in phase 1.
#[derive(Clone, Copy, Debug)]
enum Slot {
    Type(TypeId),
    Pattern(ProjectionId),
    Binding(u32),
    Decl(u32),
    None,
}

/// Local names bound by one binding, parameter list, or clause.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NameKey {
    text: String,
    scope: SyntaxScope,
    identity: Option<ScopeIdentity>,
}

#[derive(Clone, Debug)]
struct Frame {
    names: HashMap<NameKey, VarId>,
    /// `Some(d)`: the names may only be used inside function bodies that
    /// are deeper than `d` (§6.10, the value of a direct binding).
    fn_only: Option<u32>,
}

#[derive(Clone, Debug)]
struct TyFrame {
    names: HashMap<NameKey, TyVarId>,
}

struct TyDefMode {
    params: HashMap<NameKey, TyVarId>,
    used: HashSet<TyVarId>,
}

enum Found {
    Private,
    Local(VarId),
    Entry(Entry),
}

/// The attributes of one item, after they are checked.
#[derive(Default)]
struct Attrs {
    external: Option<(String, String)>,
    host: Option<Host>,
    main: bool,
    deprecated: Option<String>,
}

struct Resolver<'a> {
    graph: &'a PackageGraph,
    parsed: &'a [Parsed],
    p: Program,
    diags: Vec<Diagnostic>,
    generated_values: Vec<HashMap<NameKey, Entry>>,
    generated_types: Vec<HashMap<NameKey, TypeId>>,
    defs_by_name: Vec<HashMap<String, u32>>,
    macros_by_name: Vec<HashMap<String, bool>>,
    types_by_name: Vec<HashMap<String, TypeId>>,
    ctors_by_name: Vec<HashMap<String, CtorRef>>,
    slots: Vec<Vec<Slot>>,
    scopes: Vec<Scope>,
    by_path: HashMap<(PackageId, String), ModuleId>,
    /// The built-in names (§12): the prelude and the built-in types.
    builtin_values: HashMap<String, Entry>,
    builtin_types: HashMap<String, TypeRef>,
    // The state of the module and body being resolved.
    cur: ModuleId,
    locals: Vec<Frame>,
    fn_depth: u32,
    ty_frames: Vec<TyFrame>,
    ty_def: Option<TyDefMode>,
    /// The aliases in a circle (§4.3); the circle has been reported, and
    /// they are not expanded.
    circular: HashSet<TypeId>,
    pending: Vec<HashSet<NameKey>>,
    reserved: Vec<(usize, HashSet<NameKey>)>,
    checked_inputs: Option<&'a CheckedInputs>,
    /// The selection tables (syntax three §2.6.1) of the top-level names
    /// and of the local names in scope: for each shape group, its function.
    def_tables: HashMap<DefId, Table>,
    local_tables: HashMap<VarId, Table>,
}

/// The shape groups of a named chain and the binding of each group.
type Table = std::rc::Rc<Vec<(ShapeId, ValueRef)>>;

/// The reserved callee of the transition form of a multi-shape value: a
/// call with one argument per shape group, labelled by the shape.
const MULTI: &str = "%multi";


/// The shape that a group label of the transition form writes: the number
/// of positions, then each label after a comma.
fn shape_of_text(text: &str) -> ShapeId {
    let mut parts = text.split(',');
    let positions: u32 = parts
        .next()
        .unwrap()
        .parse()
        .expect("a shape starts with its positions");
    ShapeId::new(positions, parts.map(str::to_string))
}

/// A shape as it is written in a parameter group, for messages.
fn shape_shown(shape: ShapeId) -> String {
    let mut parts = vec!["_".to_string(); shape.positions() as usize];
    parts.extend(shape.labels().iter().map(|l| format!("{l} = _")));
    format!("({})", parts.join(", "))
}

fn is_upper(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
}


impl<'a> Resolver<'a> {
    fn resolved_quote(
        &mut self,
        template: Option<u32>,
        captures: Vec<(bool, Pattern)>,
        span: Span,
    ) -> PatternKind {
        if let Some(template) = template {
            let module = Ident {
                name: "\0quote-syntax".into(),
                span,
            };
            let name = Ident {
                name: "Syntax".into(),
                span,
            };
            let syntax = self.lookup_type(Some(&module), &name);
            let functions: Option<Vec<_>> = [
                "_template",
                "_match",
                "_match_ok",
                "_match_one",
                "_match_many",
            ]
            .into_iter()
            .map(|name| {
                let name = Ident {
                    name: name.into(),
                    span,
                };
                match self.name(Some(&module), &name) {
                    ExprKind::Name(ValueRef::Def(def)) => Some(def),
                    _ => None,
                }
            })
            .collect();
            match (syntax, functions) {
                (Some(TypeRef::Custom(syntax)), Some(functions)) => PatternKind::Quote {
                    template,
                    syntax,
                    functions: functions.try_into().expect("five quote services"),
                    captures,
                },
                _ => PatternKind::Error(captures.into_iter().map(|(_, p)| p).collect()),
            }
        } else {
            self.error(
                code::MACRO_RESERVED,
                span,
                "a quote pattern must be evaluated inside a macro",
            );
            PatternKind::Error(captures.into_iter().map(|(_, p)| p).collect())
        }
    }

    // ---- Helpers ----

    fn error(&mut self, code: &'static str, span: Span, message: impl Into<String>) {
        let source = self.p.module(self.cur).source;
        let mut message = message.into();
        if matches!(code, code::UNKNOWN_NAME | code::MACRO_RESERVED)
            && let Some(hint) = ilang_syntax::old_syntax_hint(&self.graph.sources.get(source).text)
        {
            message.push_str("; ");
            message.push_str(hint);
        }
        self.diags
            .push(Diagnostic::error(code, Some(source), span, message));
    }

    fn module_name(&self, m: ModuleId) -> &str {
        &self.p.module(m).path
    }

    fn is_core(&self, m: ModuleId) -> bool {
        self.p.module(m).package == self.graph.library && self.module_name(m) == "i/core"
    }

    fn in_library(&self) -> bool {
        self.p.module(self.cur).package == self.graph.library
    }

    fn new_var(&mut self) -> VarId {
        self.p.var_count += 1;
        VarId(self.p.var_count - 1)
    }

    fn new_ty_var(&mut self, name: &str) -> TyVarId {
        self.p.ty_var_count += 1;
        self.p.ty_var_names.push(name.to_string());
        TyVarId(self.p.ty_var_count - 1)
    }

    fn deprecated(&mut self, module: ModuleId, note: Option<String>, what: &str, span: Span) {
        if module != self.cur
            && let Some(note) = note
        {
            let source = self.p.module(self.cur).source;
            self.diags.push(Diagnostic::warning(
                code::DEPRECATED,
                Some(source),
                span,
                format!("`{what}` is deprecated: {note}"),
            ));
        }
    }

    fn used_def(&mut self, d: DefId, span: Span) {
        let def = self.p.def(d);
        let (note, name) = (def.deprecated.clone(), def.name.clone());
        self.deprecated(d.module, note, &name, span);
    }

    fn used_type(&mut self, t: TypeId, span: Span) {
        let def = self.p.ty(t);
        let (m, note, name) = (def.module, def.deprecated.clone(), def.name.clone());
        self.deprecated(m, note, &name, span);
    }

    fn used_ctor(&mut self, c: CtorRef, span: Span) {
        let def = self.p.ty(c.ty);
        let TypeBody::Custom(ctors) = &def.body else {
            return;
        };
        let name = ctors[c.index as usize].name.clone();
        let (m, note) = (def.module, def.deprecated.clone());
        self.deprecated(m, note, &name, span);
    }

    // ---- Phase 1: top-level definitions ----

    fn create_modules(&mut self) {
        for (i, parsed) in self.parsed.iter().enumerate() {
            let id = ModuleId(i as u32);
            self.p.modules.push(Module {
                path: parsed.path.clone(),
                package: parsed.package,
                source: parsed.source,
                kind: parsed.kind,
                scoped: parsed.scoped.clone(),
                imports: Vec::new(),
                import_names: Vec::new(),
                imported_types: Vec::new(),
                defs: Vec::new(),
                types: Vec::new(),
                bindings: Vec::new(),
            });
            if parsed.kind == ModuleKind::Prelude {
                self.p.prelude = Some(id);
            }
            self.by_path
                .insert((parsed.package, parsed.path.clone()), id);
            self.generated_values.push(HashMap::new());
            self.generated_types.push(HashMap::new());
            self.defs_by_name.push(HashMap::new());
            self.macros_by_name.push(HashMap::new());
            self.types_by_name.push(HashMap::new());
            self.ctors_by_name.push(HashMap::new());
            self.slots.push(Vec::new());
            self.scopes.push(Scope::default());
        }
    }

    /// A marked top-level name must not shadow an ordinary name in its
    /// definition module: otherwise the two branches of §5 rule 3 compete.
    fn check_generated_collisions(&mut self) {
        let mut conflicts = Vec::new();
        for (m, values) in self.generated_values.iter().enumerate() {
            for (key, entry) in values {
                if let Some(owner) = self
                    .parsed
                    .iter()
                    .position(|p| p.source.0 == key.scope.module)
                    && (self.scopes[owner].values.contains_key(&key.text)
                        || self.scopes[owner].types.contains_key(&key.text))
                {
                    let span = match entry {
                        Entry::Def(d) => self.p.def(*d).span,
                        Entry::Ctor(c) => match &self.p.ty(c.ty).body {
                            TypeBody::Custom(cs) => cs[c.index as usize].span,
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    };
                    conflicts.push((ModuleId(m as u32), key.text.clone(), span));
                }
            }
        }
        for (m, types) in self.generated_types.iter().enumerate() {
            for (key, id) in types {
                if let Some(owner) = self
                    .parsed
                    .iter()
                    .position(|p| p.source.0 == key.scope.module)
                    && (self.scopes[owner].values.contains_key(&key.text)
                        || self.scopes[owner].types.contains_key(&key.text))
                {
                    conflicts.push((ModuleId(m as u32), key.text.clone(), self.p.ty(*id).span));
                }
            }
        }
        conflicts.sort_by(|a, b| {
            (a.0.0, a.2.start, a.2.end, &a.1).cmp(&(b.0.0, b.2.start, b.2.end, &b.1))
        });
        for (m, name, span) in conflicts {
            self.cur = m;
            self.error(
                code::DEFINED_TWICE,
                span,
                format!("marked `{name}` conflicts with a name in its scope module (§5)"),
            );
        }
    }

    /// A value name of this module may be defined only once (§2.2).
    fn value_free(&mut self, m: ModuleId, name: &str, span: Span) -> bool {
        let mi = m.0 as usize;
        let key = self.name_key(name, span);
        self.check_binder(name, span, true, false);
        let occupied = if key.scope.marks.is_empty() {
            self.defs_by_name[mi].contains_key(name)
                || self.macros_by_name[mi].contains_key(name)
                || self.ctors_by_name[mi].contains_key(name)
        } else {
            self.generated_values[mi].contains_key(&key)
        };
        if occupied {
            self.error(
                code::DEFINED_TWICE,
                span,
                format!("`{name}` is defined more than once in this module"),
            );
            return false;
        }
        true
    }

    fn add_def(&mut self, m: ModuleId, name: &str, span: Span, kind: DefKind, attrs: &Attrs) {
        if !self.value_free(m, name, span) {
            return;
        }
        let module = &mut self.p.modules[m.0 as usize];
        let index = module.defs.len() as u32;
        module.defs.push(Def {
            name: name.to_string(),
            span,
            kind,
            exported: false,
            deprecated: attrs.deprecated.clone(),
        });
        let key = self.name_key(name, span);
        if key.scope.marks.is_empty() {
            self.defs_by_name[m.0 as usize].insert(name.to_string(), index);
        } else {
            self.generated_values[m.0 as usize].insert(key, Entry::Def(DefId { module: m, index }));
        }
    }


    /// §2.4.
    fn exports(&mut self, m: ModuleId, items: &[SourceExport]) {
        let mi = m.0 as usize;
        for item in items {
            let name = &item.name.name;
            let key = self.name_key(name, item.name.span);
            if !key.scope.marks.is_empty() {
                self.error(
                    code::MACRO_BINDER,
                    item.name.span,
                    format!("marked `{name}` cannot be exported (§5)"),
                );
                continue;
            }
            if !is_upper(name) {
                if let Some(exported) = self.macros_by_name[mi].get_mut(name) {
                    *exported = true;
                    continue;
                }
                match self.defs_by_name[mi].get(name) {
                    Some(&d) => self.p.modules[mi].defs[d as usize].exported = true,
                    None => {
                        let mut message =
                            format!("`{name}` is exported but not defined in this module");
                        let package = &self.graph.packages[self.p.module(m).package];
                        let mut packages = vec![
                            ("i".to_string(), self.graph.library),
                            (package.name.clone(), self.p.module(m).package),
                        ];
                        packages.extend(
                            package
                                .dependencies
                                .iter()
                                .chain(&package.dev_dependencies)
                                .filter_map(|(n, p)| p.map(|p| (n.clone(), p))),
                        );
                        for (prefix, id) in packages {
                            let target = &self.graph.packages[id];
                            let own_path = format!("{}/{name}", target.name);
                            if target.modules.iter().any(|module| module.path == own_path) {
                                message.push_str(&format!(
                                    "; a module path has a `/`, as in `({prefix}/{name})`"
                                ));
                                break;
                            }
                        }
                        self.error(code::EXPORT_UNDEFINED, item.span, message);
                    }
                }
                continue;
            }
            let Some(&t) = self.types_by_name[mi].get(name) else {
                self.error(
                    code::EXPORT_UNDEFINED,
                    item.span,
                    format!("the type `{name}` is exported but not defined in this module"),
                );
                continue;
            };
            let def = &mut self.p.types[t.0 as usize];
            def.exported = true;
            // `Alias(..)` is the same as `Alias`: an alias has no
            // constructors of its own, so there is nothing more to export.
            if item.constructors && matches!(def.body, TypeBody::Custom(_)) {
                def.constructors_exported = true;
            }
        }
    }

    fn builtins(&mut self) {
        for b in BuiltinType::ALL {
            self.builtin_types
                .insert(b.name().to_string(), TypeRef::Builtin(b));
        }
        let Some(prelude) = self.p.prelude else {
            return;
        };
        let pi = prelude.0 as usize;
        for (name, &d) in &self.defs_by_name[pi] {
            self.builtin_values.insert(
                name.clone(),
                Entry::Def(DefId {
                    module: prelude,
                    index: d,
                }),
            );
        }
        for (name, &c) in &self.ctors_by_name[pi] {
            self.builtin_values.insert(name.clone(), Entry::Ctor(c));
        }
        for (name, &t) in &self.types_by_name[pi] {
            self.builtin_types.insert(name.clone(), TypeRef::Custom(t));
        }
    }

    // ---- Phase 2: imports (§2.3) ----


    /// The module that an import path names (§2.1).
    fn import_target(&mut self, item: &SourceImport) -> Option<ModuleId> {
        let me = self.p.module(self.cur);
        let pkg = &self.graph.packages[me.package];
        let first = item.path[0].name.as_str();
        let is_test = me.kind == ModuleKind::Test;
        let package = if first == "i" {
            Some(self.graph.library)
        } else if first == pkg.name {
            Some(me.package)
        } else if let Some((_, id)) = pkg.dependencies.iter().find(|(n, _)| n == first) {
            *id
        } else if let Some((_, id)) = pkg
            .dev_dependencies
            .iter()
            .find(|(n, _)| n == first && is_test)
        {
            *id
        } else {
            let dev = pkg.dev_dependencies.iter().any(|(n, _)| n == first);
            let msg = if dev {
                format!("`{first}` is a dev-dependency, so only modules under `test/` may use it")
            } else {
                format!(
                    "`{first}` is not `i`, the name of this package, or a dependency name (§2.1)"
                )
            };
            self.error(code::UNKNOWN_PACKAGE, item.path[0].span, msg);
            return None;
        };
        // A dependency that could not be loaded has been reported.
        let package = package?;
        let rest: Vec<&str> = item.path[1..].iter().map(|p| p.name.as_str()).collect();
        let path = format!("{}/{}", self.graph.packages[package].name, rest.join("/"));
        let written: Vec<&str> = item.path.iter().map(|p| p.name.as_str()).collect();
        let written = written.join("/");
        let found = self.by_path.get(&(package, path)).copied();
        let Some(target) = found else {
            self.error(
                code::NO_MODULE,
                item.span,
                format!("there is no module `{written}`"),
            );
            return None;
        };
        if self.p.module(target).kind == ModuleKind::Test && !is_test {
            self.error(
                code::SRC_IMPORTS_TEST,
                item.span,
                format!("`{written}` is a module under `test/`, which a module under `src/` may not import"),
            );
            return None;
        }
        Some(target)
    }

    fn put_value(&mut self, scope: &mut Scope, name: &str, span: Span, entry: Entry) {
        match scope.values.get(name) {
            Some((_, false)) => self.error(
                code::IMPORT_IS_TOP_LEVEL,
                span,
                format!("`{name}` is imported, but it is also a top-level name of this module"),
            ),
            Some((_, true)) => self.error(
                code::IMPORTED_TWICE,
                span,
                format!("`{name}` is put in by more than one import"),
            ),
            None => {
                scope.values.insert(name.to_string(), (entry, true));
            }
        }
    }

    fn put_type(&mut self, scope: &mut Scope, name: &str, span: Span, t: TypeRef) {
        match scope.types.get(name) {
            Some((TypeRef::Custom(id), false))
                if self.p.alias_patterns.iter().any(|p| p.binders.contains(id)) =>
            {
                self.error(
                    code::TYPE_PATTERN_BINDER_IS_TYPE,
                    self.p.ty(*id).span,
                    format!("the new alias `{name}` conflicts with an imported type"),
                );
            }
            Some((_, false)) => self.error(
                code::IMPORT_IS_TOP_LEVEL,
                span,
                format!("the type `{name}` is imported, but this module also defines it"),
            ),
            Some((_, true)) => self.error(
                code::IMPORTED_TWICE,
                span,
                format!("the type `{name}` is put in by more than one import"),
            ),
            None => {
                scope.types.insert(name.to_string(), (t, true));
                if let TypeRef::Custom(id) = t {
                    self.p.modules[self.cur.0 as usize].imported_types.push(id);
                }
            }
        }
    }

    /// `(A, b) = path` (§2.3). Only exported names may be imported.
    fn import_name(&mut self, scope: &mut Scope, target: ModuleId, n: &Ident) {
        let ti = target.0 as usize;
        let module = self.module_name(target).to_string();
        if !is_upper(&n.name) {
            if let Some(&exported) = self.macros_by_name[ti].get(&n.name) {
                if exported {
                    self.put_value(scope, &n.name, n.span, Entry::Macro { exported });
                } else {
                    self.error(
                        code::NOT_EXPORTED,
                        n.span,
                        format!("`{}` is not exported from `{module}`", n.name),
                    );
                }
                return;
            }
            match self.defs_by_name[ti].get(&n.name) {
                Some(&d) if self.p.modules[ti].defs[d as usize].exported => {
                    let id = DefId {
                        module: target,
                        index: d,
                    };
                    self.put_value(scope, &n.name, n.span, Entry::Def(id));
                }
                Some(_) => self.error(
                    code::NOT_EXPORTED,
                    n.span,
                    format!("`{}` is not exported from `{module}`", n.name),
                ),
                None => self.error(
                    code::NO_MEMBER,
                    n.span,
                    format!("`{module}` has no `{}`", n.name),
                ),
            }
            return;
        }
        if self.is_core(target)
            && let Some(b) = BuiltinType::ALL.into_iter().find(|b| b.name() == n.name)
        {
            self.put_type(scope, &n.name, n.span, TypeRef::Builtin(b));
            return;
        }
        let ty = self.types_by_name[ti].get(&n.name).copied();
        let ctor = self.ctors_by_name[ti].get(&n.name).copied();
        let ty_ok = ty.filter(|t| self.p.ty(*t).exported);
        let ctor_ok = ctor.filter(|c| self.p.ty(c.ty).constructors_exported);
        if let Some(t) = ty_ok {
            self.put_type(scope, &n.name, n.span, TypeRef::Custom(t));
        }
        if let Some(c) = ctor_ok {
            self.put_value(scope, &n.name, n.span, Entry::Ctor(c));
        }
        if ty_ok.is_none() && ctor_ok.is_none() {
            if ty.is_some() || ctor.is_some() {
                self.error(
                    code::NOT_EXPORTED,
                    n.span,
                    format!("`{}` is not exported from `{module}`", n.name),
                );
            } else {
                self.error(
                    code::NO_MEMBER,
                    n.span,
                    format!("`{module}` has no `{}`", n.name),
                );
            }
        }
    }


    /// `imports` gives the written imports of a module: the path and the
    /// span of each.
    fn import_circles_of(&mut self, imports: &dyn Fn(usize) -> Vec<(Vec<String>, Span)>) {
        let mut g = petgraph::graph::DiGraph::<u32, ()>::new();
        let nodes: Vec<_> = (0..self.p.modules.len() as u32)
            .map(|i| g.add_node(i))
            .collect();
        for (i, m) in self.p.modules.iter().enumerate() {
            for t in &m.imports {
                g.add_edge(nodes[i], nodes[t.0 as usize], ());
            }
        }
        for scc in petgraph::algo::tarjan_scc(&g) {
            let mut members: Vec<u32> = scc.iter().map(|n| g[*n]).collect();
            members.sort();
            let self_loop = members.len() == 1
                && self.p.modules[members[0] as usize]
                    .imports
                    .contains(&ModuleId(members[0]));
            if members.len() < 2 && !self_loop {
                continue;
            }
            let names: Vec<&str> = members
                .iter()
                .map(|&m| self.p.modules[m as usize].path.as_str())
                .collect();
            let names = names.join(", ");
            for &m in &members {
                self.cur = ModuleId(m);
                // Report at the first import that goes into the circle.
                let written = imports(m as usize);
                let item = written.iter().find(|(path, _)| {
                    let me = self.p.module(ModuleId(m));
                    let first = &path[0];
                    let pkg = &self.graph.packages[me.package];
                    let package = if first == "i" {
                        Some(self.graph.library)
                    } else if *first == pkg.name {
                        Some(me.package)
                    } else {
                        pkg.dependencies
                            .iter()
                            .chain(&pkg.dev_dependencies)
                            .find(|(n, _)| n == first)
                            .and_then(|(_, id)| *id)
                    };
                    let rest: Vec<&str> = path[1..].iter().map(|p| p.as_str()).collect();
                    package.is_some_and(|p| {
                        let path = format!("{}/{}", self.graph.packages[p].name, rest.join("/"));
                        self.by_path
                            .get(&(p, path))
                            .is_some_and(|t| members.contains(&t.0))
                    })
                });
                let span = item.map_or(Span::default(), |i| i.1);
                self.error(
                    code::IMPORT_CIRCLE,
                    span,
                    format!("these modules import one another in a circle: {names}"),
                );
            }
        }
    }

    // ---- Phase 3: type definitions (§4.3) ----


    /// The part of `early_types` that reads no tree.
    fn early_type_checks(&mut self) {
        for i in 0..self.p.alias_patterns.len() {
            let pattern = self.p.alias_patterns[i].clone();
            if !self.projectable_pattern(&pattern.pattern, &mut HashSet::new()) {
                self.cur = pattern.module;
                self.error(code::TYPE_PATTERN_FORM,pattern.head_span,"an alias-pattern head must expand to one named type or tuple; each formal occurs at most once and only through named arguments or tuple parts");
                self.p.alias_patterns[i].pattern = TypePattern::Error;
                for id in pattern.binders {
                    if let TypeBody::Alias(Type {
                        kind: TypeKind::Of { site, .. },
                        ..
                    }) = self.p.ty(id).body
                    {
                        self.p.failed_reads.insert(site);
                    }
                }
                if let TypeKind::Of { site, .. } = pattern.source.kind {
                    self.p.failed_reads.insert(site);
                }
            }
        }
        let mut checks = Vec::new();
        for def in &self.p.types {
            if let TypeBody::Custom(ctors) = &def.body {
                for c in ctors {
                    for f in &c.fields {
                        if value_dependent(&self.p, &f.ty) {
                            checks.push((def.module, f.ty.span));
                        }
                    }
                }
            }
        }
        for (m, span) in checks {
            self.cur = m;
            self.error(
                code::VALUE_TYPE_PLACE,
                span,
                "a value-dependent type is not allowed in a constructor field or host type",
            );
        }
        let host_aliases = self
            .p
            .types
            .iter()
            .filter_map(|d| match &d.body {
                TypeBody::Alias(t) if d.host.is_some() => Some((d.module, t.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        for (m, t) in host_aliases {
            self.cur = m;
            self.reject_value_type(&t, "a host declaration");
        }
    }

    fn projectable_pattern(&self, pattern: &TypePattern, seen: &mut HashSet<TypeId>) -> bool {
        match pattern {
            TypePattern::Bind(_) | TypePattern::Skip | TypePattern::Error => true,
            TypePattern::Exact(t) => !value_dependent(&self.p, t),
            TypePattern::Tuple(parts) => parts.iter().all(|p| self.projectable_pattern(p, seen)),
            TypePattern::Named(TypeRef::Custom(id), parts)
                if matches!(self.p.ty(*id).body, TypeBody::Alias(_)) =>
            {
                if self.circular.contains(id) {
                    return true;
                }
                if !seen.insert(*id) {
                    return false;
                }
                let def = self.p.ty(*id);
                let TypeBody::Alias(body) = &def.body else {
                    unreachable!()
                };
                if value_dependent(&self.p, body) {
                    return false;
                }
                let map = def
                    .params
                    .iter()
                    .map(|(_, v)| *v)
                    .zip(parts.iter().cloned())
                    .collect();
                let expanded = projectable_body(body, &map, &mut HashSet::new());
                let valid = expanded.is_some_and(|p| {
                    let shape = match &p {
                        TypePattern::Exact(t) => matches!(
                            self.expand(t).kind,
                            TypeKind::Named { .. } | TypeKind::Tuple(_)
                        ),
                        TypePattern::Named(..) | TypePattern::Tuple(_) => true,
                        _ => false,
                    };
                    shape && self.projectable_pattern(&p, seen)
                });
                seen.remove(id);
                valid
            }
            TypePattern::Named(_, parts) => parts.iter().all(|p| self.projectable_pattern(p, seen)),
        }
    }

    fn environment_reads(&mut self) {
        if self.p.of_sites.is_empty() {
            return;
        }
        let mut errors = Vec::new();
        let mut visit = |module, t: &Type| {
            walk_type(t, &mut |t| {
                if let TypeKind::Fn { env: Some(env), .. } = &t.kind
                    && value_dependent(&self.p, env)
                {
                    errors.push((module, env.span));
                }
            });
        };
        for def in &self.p.types {
            if let TypeBody::Alias(t) = &def.body {
                visit(def.module, t);
            }
        }
        for (i, m) in self.p.modules.iter().enumerate() {
            let module = ModuleId(i as u32);
            for b in &m.bindings {
                binding_types(&b.binding, &mut |t| visit(module, t));
            }
        }
        errors.sort_by_key(|(m, s)| (m.0, s.start, s.end));
        errors.dedup();
        for (m, span) in errors {
            self.cur = m;
            self.error(
                code::VALUE_TYPE_PLACE,
                span,
                "a value-dependent type is not allowed inside an environment type",
            );
        }
    }


    /// §4.3: an alias may not use itself, directly or through other aliases.
    fn alias_circles(&mut self) {
        let mut g = petgraph::graph::DiGraph::<u32, ()>::new();
        let nodes: Vec<_> = (0..self.p.types.len() as u32)
            .map(|i| g.add_node(i))
            .collect();
        let mut self_loops = HashSet::new();
        for (i, t) in self.p.types.iter().enumerate() {
            if let TypeBody::Alias(body) = &t.body {
                let mut used = Vec::new();
                named_types(body, &mut used);
                if let TypeKind::Of {
                    projection: Some(pid),
                    ..
                } = body.kind
                {
                    pattern_type_ids(&self.p.alias_patterns[pid.0 as usize].pattern, &mut used);
                }
                for u in used {
                    if matches!(self.p.ty(u).body, TypeBody::Alias(_)) {
                        g.add_edge(nodes[i], nodes[u.0 as usize], ());
                        if u.0 as usize == i {
                            self_loops.insert(i);
                        }
                    }
                }
            }
        }
        for scc in petgraph::algo::tarjan_scc(&g) {
            if scc.len() < 2 && !self_loops.contains(&(g[scc[0]] as usize)) {
                continue;
            }
            for n in scc {
                self.circular.insert(TypeId(g[n]));
                let t = self.p.ty(TypeId(g[n]));
                let (m, span, name) = (t.module, t.span, t.name.clone());
                self.cur = m;
                self.error(
                    code::ALIAS_CIRCLE,
                    span,
                    format!("the alias `{name}` uses itself, directly or through other aliases"),
                );
            }
        }
    }

    // ---- Types ----

    fn lookup_type(&mut self, module: Option<&Ident>, name: &Ident) -> Option<TypeRef> {
        // §2.3: a local name may cover a module name, in types too.
        if let Some(m) = module
            && let Some(Found::Local(_)) = self.find(&m.name, m.span)
        {
            self.error(
                code::NOT_A_MODULE,
                m.span,
                format!(
                    "`{}` is a local value here, not a module; a local name covers a module name (§2.3)",
                    m.name
                ),
            );
            return None;
        }
        let key = self.name_key(&name.name, name.span);
        let broken = match module {
            Some(m) => matches!(
                self.find(&m.name, m.span),
                Some(Found::Entry(Entry::Broken))
            ),
            None if key.identity.is_some()
                || self.generated_types[self.cur.0 as usize].contains_key(&key) =>
            {
                false
            }
            None => self
                .parsed
                .iter()
                .position(|p| p.source.0 == key.scope.module)
                .is_some_and(|owner| {
                    self.scopes[owner].broken_types.contains(&name.name)
                        && !self.scopes[owner].types.contains_key(&name.name)
                }),
        };
        if broken {
            return None;
        }
        if let Some(m) = module {
            let target = self.module_of(m)?;
            let ti = target.0 as usize;
            let path = self.module_name(target).to_string();
            if self.is_core(target)
                && let Some(b) = BuiltinType::ALL.into_iter().find(|b| b.name() == name.name)
            {
                return Some(TypeRef::Builtin(b));
            }
            return match self.types_by_name[ti].get(&name.name).copied() {
                Some(t) if self.p.ty(t).exported || self.compiler_reference(m) => {
                    self.used_type(t, name.span);
                    Some(TypeRef::Custom(t))
                }
                Some(_) => {
                    self.error(
                        code::NOT_EXPORTED,
                        name.span,
                        format!("the type `{}` is not exported from `{path}`", name.name),
                    );
                    None
                }
                None => {
                    self.error(
                        code::NO_MEMBER,
                        name.span,
                        format!("`{path}` has no type `{}`", name.name),
                    );
                    None
                }
            };
        }
        let origin = self.parsed[self.cur.0 as usize]
            .scoped
            .as_ref()
            .and_then(|s| s.origin(name.span));
        let provenance = origin.map(|o| o.syntax.provenance());
        let from_answer = matches!(provenance, Some(Provenance::Made { from_answer: true }));
        let generated = if !key.scope.marks.is_empty() && !from_answer {
            self.generated_types[self.cur.0 as usize]
                .get(&key)
                .copied()
                .map(TypeRef::Custom)
        } else {
            None
        };
        let found = if matches!(key.identity, Some(ScopeIdentity::Builtin { .. })) {
            self.builtin_types.get(&name.name).copied()
        } else if key.identity.is_some() {
            None
        } else if let Some(t) = generated {
            Some(t)
        } else {
            let owner = self
                .parsed
                .iter()
                .position(|p| p.source.0 == key.scope.module);
            let found = owner.and_then(|owner| {
                self.scopes[owner]
                    .types
                    .get(&name.name)
                    .map(|(t, _)| (owner, *t))
            });
            if let Some((owner, TypeRef::Custom(id))) = found {
                let made = matches!(provenance, Some(Provenance::Made { .. }));
                if made
                    && (!key.scope.marks.is_empty() || owner != self.cur.0 as usize)
                    && !self.p.ty(id).exported
                {
                    self.error(
                        code::NOT_EXPORTED,
                        name.span,
                        format!(
                            "the type `{}` is not exported from its scope module",
                            name.name
                        ),
                    );
                    return None;
                }
            }
            found
                .map(|(_, t)| t)
                .or_else(|| self.builtin_types.get(&name.name).copied())
        };
        match found {
            Some(t) => {
                if let TypeRef::Custom(id) = t {
                    self.used_type(id, name.span);
                }
                Some(t)
            }
            None => {
                self.error(
                    code::UNKNOWN_TYPE,
                    name.span,
                    format!("the type `{}` is not defined", name.name),
                );
                None
            }
        }
    }

    /// The module named by the top-level name `m` in `m.T`. Local names
    /// that cover it are found by the caller.
    fn module_of(&mut self, m: &Ident) -> Option<ModuleId> {
        match self.find(&m.name, m.span) {
            Some(Found::Entry(Entry::Module(t))) => Some(t),
            Some(Found::Private) => {
                self.error(
                    code::NOT_EXPORTED,
                    m.span,
                    format!("`{}` is not exported from its scope module", m.name),
                );
                None
            }
            Some(_) => {
                self.error(
                    code::NOT_A_MODULE,
                    m.span,
                    format!("`{}` is not a module", m.name),
                );
                None
            }
            None => {
                self.error(
                    code::UNKNOWN_NAME,
                    m.span,
                    format!("there is no module `{}`", m.name),
                );
                None
            }
        }
    }

    fn arity(&self, t: TypeRef) -> usize {
        match t {
            TypeRef::Builtin(b) => b.arity(),
            TypeRef::Custom(id) => self.p.ty(id).params.len(),
        }
    }

    fn type_name(&self, t: TypeRef) -> String {
        match t {
            TypeRef::Builtin(b) => b.name().to_string(),
            TypeRef::Custom(id) => self.p.ty(id).name.clone(),
        }
    }

    fn new_of(
        &mut self,
        value: ValueRef,
        span: Span,
        permanent: bool,
        projection: Option<ProjectionId>,
    ) -> Type {
        let site = OfId(self.p.of_sites.len() as u32);
        self.p.of_sites.push(OfSite {
            module: self.cur,
            span,
            value,
            permanent,
            projection,
        });
        Type {
            kind: TypeKind::Of {
                value,
                site,
                projection,
            },
            span,
        }
    }

    fn value_type(
        &mut self,
        module: Option<&Ident>,
        name: &Ident,
        span: Span,
    ) -> TypeKind {
        let pending = |n: &str, span: Span| {
            self.pending
                .iter()
                .rev()
                .any(|p| p.contains(&self.name_key(n, span)))
        };
        if let Some(m) = module {
            if pending(&m.name, m.span)
                || self.reserved.iter().rev().any(|(depth, names)| {
                    names.contains(&self.name_key(&m.name, m.span))
                        && !self
                            .locals
                            .iter()
                            .skip(*depth)
                            .any(|f| f.names.contains_key(&self.name_key(&m.name, m.span)))
                })
            {
                self.error(
                    code::NOT_A_MODULE,
                    m.span,
                    format!("`{}` is a pending value, not a module", m.name),
                );
                return TypeKind::Error;
            }
        } else {
            let reserved = self.reserved.iter().rev().any(|(depth, names)| {
                names.contains(&self.name_key(&name.name, name.span))
                    && !self
                        .locals
                        .iter()
                        .skip(*depth)
                        .any(|f| f.names.contains_key(&self.name_key(&name.name, name.span)))
            });
            if pending(&name.name, name.span) || reserved {
                self.error(code::VALUE_TYPE_OWN_SCOPE, span,
                    format!("the type of `{}` cannot be read in its own pattern, function header, or binding value, even if an outer name exists", name.name));
                return TypeKind::Error;
            }
        }
        match self.name(module, name) {
            ExprKind::Name(value @ (ValueRef::Local(_) | ValueRef::Def(_))) => {
                self.new_of(value, span, self.ty_def.is_some(), None).kind
            }
            _ => TypeKind::Error,
        }
    }


    /// A label of data is a `name` (syntax-3 §2.3): not a string that is
    /// not a name, an `UpName`, or a label parameter.
    fn check_data_label(&mut self, label: &Ident, parameter: bool) {
        let name = label.name.as_str();
        let mut chars = name.chars();
        let first = chars.next();
        let is_name = matches!(first, Some('a'..='z' | '_'))
            && name != "_"
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if parameter || !is_name {
            self.error(
                code::INVALID_RECORD_KEY,
                label.span,
                format!(
                    "the label `{name}` is not a name; a label of data is a `name` (syntax-3 §2.3)"
                ),
            );
        }
    }

    /// The type that the alias `t` stands for, with its arguments put in
    /// the places of its parameters (§4.3). `None` when `t` is not a use of
    /// an alias, or of an alias in a circle.
    fn expand_alias(&self, t: &Type) -> Option<Type> {
        let TypeKind::Named {
            ty: TypeRef::Custom(id),
            args,
        } = &t.kind
        else {
            return None;
        };
        let def = self.p.ty(*id);
        let TypeBody::Alias(body) = &def.body else {
            return None;
        };
        if self.circular.contains(id) || def.params.len() != args.len() {
            return None;
        }
        let map: HashMap<TyVarId, &Type> = def.params.iter().map(|p| p.1).zip(args).collect();
        Some(subst(body, &map))
    }

    /// `t`, or the type it stands for when it is a use of an alias, with
    /// every alias at its top expanded.
    fn expand(&self, t: &Type) -> Type {
        let mut cur = t.clone();
        while let Some(next) = self.expand_alias(&cur) {
            cur = next;
        }
        cur
    }

    /// Shared early-context gate. It follows alias provenance, even when a
    /// completed alias would have an otherwise permitted host/service shape.
    fn reject_value_type(&mut self, t: &Type, context: &str) -> bool {
        if !value_dependent(&self.p, t) {
            return false;
        }
        self.error(
            code::VALUE_TYPE_PLACE,
            t.span,
            format!("a value-dependent type is not allowed in {context}"),
        );
        true
    }

    /// §9.2: the type of an environment parameter or statement is one named
    /// type: not a type variable alone, a union, or `Never`. An alias is
    /// expanded with its arguments. Gives back the type constructor, or
    /// `None` when there is none or a problem has been reported.
    fn env_type(&mut self, t: &Type) -> Option<TypeRef> {
        if self.reject_value_type(t, "a service/environment type") {
            return None;
        }
        let head = match &self.expand(t).kind {
            TypeKind::Error => return None,
            // An alias in a circle; the circle has been reported.
            TypeKind::Named {
                ty: TypeRef::Custom(id),
                ..
            } if self.circular.contains(id) => return None,
            TypeKind::Named {
                ty: TypeRef::Builtin(BuiltinType::Never),
                ..
            } => None,
            TypeKind::Named { ty, .. } => Some(*ty),
            _ => None,
        };
        if head.is_none() {
            self.error(
                code::ENV_TYPE,
                t.span,
                "a service type is one named type `T(…)`: not a type variable, a union, or `Never` (§9.2)",
            );
        }
        head
    }

    /// §9.7: the parameters, values, and errors of the functions of a host
    /// service are types that the host can take (§10.3), and they have no
    /// environment. Each such function is put in `host_fns`.
    fn host_fns(&mut self) {
        let request_module = self
            .p
            .modules
            .iter()
            .position(|m| m.path == "i/request")
            .map(|i| ModuleId(i as u32));
        let service = |name: &str| {
            self.p
                .types
                .iter()
                .position(|t| Some(t.module) == request_module && t.name == name)
                .map(|i| TypeId(i as u32))
        };
        let await_type = service("Await");
        let owner_type = service("Owner");
        for t in 0..self.p.types.len() {
            let def = &self.p.types[t];
            let (Some(host), TypeBody::Custom(ctors)) = (&def.host, &def.body) else {
                continue;
            };
            let [ctor] = ctors.as_slice() else { continue };
            let (module, asynchronous, raw) = (def.module, host.is_async, host.is_request);
            let fields = ctor.fields.clone();
            for (i, f) in fields.iter().enumerate() {
                if value_dependent(&self.p, &f.ty) {
                    continue;
                }
                let TypeKind::Fn {
                    params,
                    ret,
                    err,
                    env,
                } = &f.ty.kind
                else {
                    continue;
                };
                let mut wire = f.ty.clone();
                let mut wrapped = false;
                let mut valid = true;
                if raw {
                    valid = err.is_none() && env.is_none();
                    let expanded = self.expand(ret);
                    if let TypeKind::Named {
                        ty: TypeRef::Builtin(BuiltinType::Request),
                        args,
                    } = expanded.kind
                        && args.len() == 1
                    {
                        let answer = self.expand(&args[0]);
                        let (value, error) = if let TypeKind::Named {
                            ty: TypeRef::Custom(id),
                            args,
                        } = &answer.kind
                            && self.p.ty(*id).name == "Result"
                            && Some(self.p.ty(*id).module) == self.p.prelude
                            && args.len() == 2
                        {
                            wrapped = true;
                            (args[0].clone(), Some(Box::new(args[1].clone())))
                        } else {
                            (answer, None)
                        };
                        wire.kind = TypeKind::Fn {
                            params: params.clone(),
                            ret: Box::new(value),
                            err: error,
                            env: None,
                        };
                    } else {
                        valid = false;
                    }
                }
                let signature = valid.then(|| self.host_fn(&wire, true)).flatten();
                if let Some(mut hf) = signature {
                    hf.request_result = wrapped;
                    self.p.host_fns.insert((TypeId(t as u32), i as u32), hf);
                    if asynchronous {
                        let mut needs = Vec::new();
                        if !raw && let Some(id) = await_type {
                            needs.push(Type {
                                kind: TypeKind::Named {
                                    ty: TypeRef::Custom(id),
                                    args: vec![],
                                },
                                span: f.ty.span,
                            });
                        }
                        if let Some(id) = owner_type {
                            needs.push(Type {
                                kind: TypeKind::Named {
                                    ty: TypeRef::Custom(id),
                                    args: vec![],
                                },
                                span: f.ty.span,
                            });
                        }
                        if let TypeBody::Custom(cs) = &mut self.p.types[t].body
                            && let TypeKind::Fn { env, .. } = &mut cs[0].fields[i].ty.kind
                        {
                            *env = Some(Box::new(Type {
                                kind: TypeKind::Union(needs),
                                span: f.ty.span,
                            }));
                        }
                        if let Some(dependency) = request_module
                            && dependency != module
                            && !self.p.modules[module.0 as usize]
                                .imports
                                .contains(&dependency)
                        {
                            self.p.modules[module.0 as usize].imports.push(dependency);
                        }
                    }
                } else {
                    self.cur = module;
                    let input_note = match &wire.kind {
                        TypeKind::Fn { ret, err, .. } => [Some(&**ret), err.as_deref()]
                            .into_iter()
                            .flatten()
                            .find_map(|ty| {
                                let note = self.host_input_note(ty);
                                (!note.is_empty()).then_some(note)
                            })
                            .unwrap_or_default(),
                        _ => String::new(),
                    };
                    self.error(code::HOST_TYPE, f.ty.span,
                        format!("a host field has ABI parameter, result and error types, and no written environment; request fields return Request(A) or Request(Result(A,E)), with no written errors (§9.7, §10.3){input_note}"));
                }
            }
        }
    }

    /// The parts of B.2 of a written function type at the host boundary,
    /// or `None` when a part is not a type that the host can take. Errors
    /// are only allowed with `errors`.
    fn host_fn(&self, t: &Type, errors: bool) -> Option<HostFn> {
        let TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } = &self.expand(t).kind
        else {
            return None;
        };
        if let Some(env) = env
            && !self.is_never(env)
        {
            return None;
        }
        let params = params
            .iter()
            .map(|p| self.host_part(&p.ty, true))
            .collect::<Option<Vec<_>>>()?;
        let result = self.host_part(ret, false)?;
        let mut members = Vec::new();
        if let Some(err) = err {
            if !errors && !self.is_never(err) {
                return None;
            }
            let err = self.boundary_type(err)?;
            let parts = match err {
                Ty::Union(ms) => ms,
                Ty::Never => Vec::new(),
                t => vec![t],
            };
            for m in parts {
                let h = self.host_member(&m, false)?;
                if matches!(h, HostType::Opaque(_)) {
                    return None;
                }
                members.push(h);
            }
        }
        members.sort();
        Some(HostFn {
            params,
            result,
            errors: members,
            request_result: false,
        })
    }

    fn is_never(&self, t: &Type) -> bool {
        self.boundary_type(t) == Some(Ty::Never)
    }

    /// The part of B.2 for a type that the host can take (§10.3).
    fn host_part(&self, t: &Type, outgoing: bool) -> Option<HostType> {
        if let Some(t) = self.boundary_type(t) {
            return self.host_member(&t, outgoing);
        }
        // Opaque custom types may have parameters; they have no scalar set.
        if let TypeKind::Named {
            ty: TypeRef::Custom(id),
            ..
        } = self.expand(t).kind
            && matches!(&self.p.ty(id).body, TypeBody::Custom(cs) if cs.is_empty())
        {
            return Some(HostType::Opaque(id));
        }
        None
    }

    fn host_member(&self, t: &Ty, outgoing: bool) -> Option<HostType> {
        if t.scalar().is_some() && !outgoing && !t.values().is_all() {
            return None;
        }
        match t {
            Ty::Con(Head::Named(TypeRef::Builtin(b)), _, args) if args.is_empty() => match b {
                BuiltinType::Int => Some(HostType::Int),
                BuiltinType::Float => Some(HostType::Float),
                BuiltinType::String => Some(HostType::String),
                BuiltinType::BitArray => Some(HostType::BitArray),
                _ => None,
            },
            Ty::Con(Head::Named(TypeRef::Custom(id)), _, _) => {
                let def = self.p.ty(*id);
                let prelude = Some(def.module) == self.p.prelude;
                match &def.body {
                    TypeBody::Custom(ctors) if ctors.is_empty() => Some(HostType::Opaque(*id)),
                    TypeBody::Custom(_) if prelude && def.name == "Nil" => Some(HostType::Nil),
                    TypeBody::Custom(_) if prelude && def.name == "Bool" => Some(HostType::Bool),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// §10.1: the written type of an external function outside the package
    /// `i`. It is a function type with no `_` and no type variables, whose
    /// parameters and value are types that the host can take (§10.3), with
    /// no errors and no environment.
    fn external_type(&mut self, t: &Type) {
        if self.reject_value_type(t, "an external declaration") {
            return;
        }
        let TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } = &t.kind
        else {
            if !matches!(t.kind, TypeKind::Error) {
                self.error(
                    code::EXTERNAL_NOT_FN,
                    t.span,
                    "the type of an external function is a function type (§10.1)",
                );
            }
            return;
        };
        for (part, what) in [(err, "errors"), (env, "an environment")] {
            if let Some(part) = part {
                self.error(
                    code::EXTERNAL_EFFECTS,
                    part.span,
                    format!("an external function may not have {what} (§10.1)"),
                );
            }
        }
        for (part, outgoing) in params
            .iter()
            .map(|p| (&p.ty, true))
            .chain([(&**ret, false)])
        {
            if let Some(span) = find_in_type(part, &|k| matches!(k, TypeKind::Hole)) {
                self.error(
                    code::EXTERNAL_HOLE,
                    span,
                    "the full type of an external function must be written, with no `_` (§10.1)",
                );
            } else if let Some(span) = find_in_type(part, &|k| matches!(k, TypeKind::Var(_))) {
                self.error(
                    code::EXTERNAL_TYPE_VAR,
                    span,
                    "the type of an external function may not have type variables (§10.1)",
                );
            } else if !self.host_type(part, outgoing) {
                self.error(
                    code::EXTERNAL_HOST_TYPE,
                    part.span,
                    format!("the parameters and the value of an external function are types that the host can take: `Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray`, or a custom type with no constructors (§10.3){}", if outgoing { String::new() } else { self.host_input_note(part) }),
                );
            }
        }
    }

    /// Aliases and scalar expressions use inference's finite/cofinite algebra.
    fn boundary_type(&self, t: &Type) -> Option<Ty> {
        crate::types::boundary_type(t, &self.p, &|t| self.expand(t))
    }

    fn host_input_note(&self, t: &Type) -> String {
        let Some(t) = self.boundary_type(t) else {
            return String::new();
        };
        let Some(b) = t.scalar().filter(|_| !t.values().is_all()) else {
            return String::new();
        };
        let complement = !t.values().is_finite();
        let mut texts: Vec<_> = t
            .values()
            .items()
            .map(|v| {
                if b == BuiltinType::String {
                    ilang_syntax::syn3::ast::LitValue::Str(v.to_string()).text()
                } else {
                    v.to_string()
                }
            })
            .collect();
        let count = texts.len();
        if count > 12 {
            texts.truncate(10);
            texts.push(format!("… ({} more)", count - 10));
        }
        let shown = if complement {
            format!("{} - {}", b.name(), texts.join(" - "))
        } else {
            texts.join(" | ")
        };
        format!(
            "; the host gives this value, so it must be all of `{}`, not `{shown}`",
            b.name()
        )
    }

    /// §10.3: a type that the host can take. Aliases are expanded.
    fn host_type(&self, t: &Type, outgoing: bool) -> bool {
        if self.host_part(t, outgoing).is_some() {
            return true;
        }
        match &self.expand(t).kind {
            TypeKind::Error => true,
            TypeKind::Named {
                ty: TypeRef::Custom(id),
                ..
            } => {
                // An alias in a circle; the circle has been reported.
                matches!(self.p.ty(*id).body, TypeBody::Alias(_))
            }
            _ => false,
        }
    }

    // ---- Phase 4: values ----


    /// A quiet lookup, after imports are known (§3.10 and §3.11).
    fn unqualified_type(&self, name: &str) -> Option<TypeRef> {
        self.scopes[self.cur.0 as usize]
            .types
            .get(name)
            .map(|(t, _)| *t)
            .or_else(|| self.builtin_types.get(name).copied())
    }

    fn conflicts_with_type(&self, key: &NameKey) -> bool {
        key.identity.is_none()
            && if key.scope.marks.is_empty() {
                self.unqualified_type(&key.text).is_some()
            } else {
                self.generated_types[self.cur.0 as usize].contains_key(key)
            }
    }

    fn push_declared_frame(&mut self, params: &[Ident]) -> Vec<TyVarId> {
        let mut names = HashMap::new();
        let mut vars = Vec::new();
        for param in params {
            let key = self.name_key(&param.name, param.span);
            self.check_binder(&param.name, param.span, false, false);
            if names.contains_key(&key) {
                self.error(
                    code::TYPE_PARAM_TWICE,
                    param.span,
                    format!(
                        "the type variable `{}` is declared more than once here",
                        param.name
                    ),
                );
                continue;
            }
            if self.conflicts_with_type(&key) {
                self.error(
                    code::TYPE_VAR_IS_TYPE,
                    param.span,
                    "a type variable may not have the name of a type",
                );
            }
            let var = self.new_ty_var(&param.name);
            names.insert(key, var);
            vars.push(var);
        }
        self.ty_frames.push(TyFrame { names });
        vars
    }


    fn check_binder(&mut self, name: &str, span: Span, top: bool, type_parameter: bool) {
        let parsed = &self.parsed[self.cur.0 as usize];
        let Some(origin) = parsed.scoped.as_ref().and_then(|s| s.origin(span)) else {
            return;
        };
        let syntax = &origin.syntax;
        let answer = matches!(
            syntax.provenance(),
            Provenance::Answer | Provenance::Made { from_answer: true }
        );
        let allowed_parameter = type_parameter
            && matches!(syntax.scope_identity(),
                Some(ScopeIdentity::DefinitionParameter { answer, .. })
                    if origin.definition_head == Some(*answer));
        let bad_answer = answer && !allowed_parameter;
        let forbidden_identity = matches!(
            syntax.scope_identity(),
            Some(ScopeIdentity::Builtin { .. } | ScopeIdentity::TypeVariable { .. })
        );
        let foreign_unmarked =
            top && syntax.scope().marks.is_empty() && syntax.scope().module != parsed.source.0;
        if bad_answer || forbidden_identity || foreign_unmarked || !syntax.valid_scope_identity() {
            self.error(
                code::MACRO_BINDER,
                span,
                format!("`{name}` cannot bind a name with this macro origin (§5)"),
            );
        }
    }

    /// Exact lexical identity of an occurrence. Written code has its source
    /// module and no marks; expanded atoms retain their syntax object identity.
    fn name_key(&self, text: &str, span: Span) -> NameKey {
        let parsed = &self.parsed[self.cur.0 as usize];
        let syntax = parsed.scoped.as_ref().and_then(|s| s.origin(span));
        let (scope, identity) = match syntax {
            Some(origin) => (
                origin.syntax.scope().clone(),
                origin.syntax.scope_identity().cloned(),
            ),
            None => (
                SyntaxScope {
                    module: parsed.source.0,
                    marks: BTreeSet::new(),
                },
                None,
            ),
        };
        let scope = if identity.is_some() {
            SyntaxScope {
                module: 0,
                marks: BTreeSet::new(),
            }
        } else {
            scope
        };
        NameKey {
            text: text.to_owned(),
            scope,
            identity,
        }
    }

    /// Looks up a value name with no module before it (§5).
    fn find(&self, name: &str, span: Span) -> Option<Found> {
        if name.starts_with('\0') {
            return self.scopes[self.cur.0 as usize]
                .values
                .get(name)
                .map(|(e, _)| Found::Entry(*e));
        }
        let key = self.name_key(name, span);
        if !is_upper(name) {
            for f in self.locals.iter().rev() {
                if f.fn_only.is_some_and(|d| self.fn_depth <= d) {
                    continue;
                }
                if let Some(v) = f.names.get(&key) {
                    return Some(Found::Local(*v));
                }
            }
        }
        if matches!(key.identity, Some(ScopeIdentity::Builtin { .. })) {
            return self.builtin_values.get(name).map(|e| Found::Entry(*e));
        }
        if key.identity.is_some() {
            return None;
        }
        let origin = self.parsed[self.cur.0 as usize]
            .scoped
            .as_ref()
            .and_then(|s| s.origin(span));
        let provenance = origin.map(|o| o.syntax.provenance());
        let from_answer = matches!(provenance, Some(Provenance::Made { from_answer: true }));
        if !key.scope.marks.is_empty()
            && !from_answer
            && let Some(e) = self.generated_values[self.cur.0 as usize].get(&key)
        {
            return Some(Found::Entry(*e));
        }
        let module = self
            .parsed
            .iter()
            .position(|p| p.source.0 == key.scope.module)?;
        let scope = &self.scopes[module];
        if let Some((e, imported)) = scope.values.get(name) {
            let needs_export = matches!(provenance, Some(Provenance::Made { .. }))
                && (!key.scope.marks.is_empty() || module != self.cur.0 as usize);
            let exported = match e {
                Entry::Def(d) => self.p.def(*d).exported,
                Entry::Macro { exported } => *exported,
                Entry::Ctor(c) => self.p.ty(c.ty).constructors_exported,
                Entry::Module(_) => *imported,
                Entry::Broken => true,
            };
            if needs_export && !exported {
                return Some(Found::Private);
            }
            return Some(Found::Entry(*e));
        }
        self.builtin_values.get(name).map(|e| Found::Entry(*e))
    }

    /// Only compiler-created modules and unspellable compiler aliases may
    /// refer directly to private implementation members.
    fn compiler_reference(&self, module: &Ident) -> bool {
        self.parsed[self.cur.0 as usize].compiler_generated || module.name.starts_with('\0')
    }

    /// A member `m.x` or `m.C` of another module.
    fn member(&mut self, target: ModuleId, name: &Ident, trusted: bool) -> ExprKind {
        let ti = target.0 as usize;
        let path = self.module_name(target).to_string();
        if let Some(&exported) = self.macros_by_name[ti].get(&name.name) {
            if !exported && !trusted {
                self.error(
                    code::NOT_EXPORTED,
                    name.span,
                    format!("`{}` is not exported from `{path}`", name.name),
                );
            } else {
                self.error(
                    code::MACRO_RESERVED,
                    name.span,
                    format!(
                        "macro `{}` can only be used as a macro call (§4.1)",
                        name.name
                    ),
                );
            }
            return ExprKind::Error;
        }
        let (found, exported) = if is_upper(&name.name) {
            match self.ctors_by_name[ti].get(&name.name) {
                Some(&c) => (
                    Some(ValueRef::Ctor(c)),
                    self.p.ty(c.ty).constructors_exported,
                ),
                None => (None, false),
            }
        } else {
            match self.defs_by_name[ti].get(&name.name) {
                Some(&d) => {
                    let id = DefId {
                        module: target,
                        index: d,
                    };
                    (Some(ValueRef::Def(id)), self.p.def(id).exported)
                }
                None => (None, false),
            }
        };
        match found {
            Some(v) if exported || trusted => {
                match v {
                    ValueRef::Def(d) => self.used_def(d, name.span),
                    ValueRef::Ctor(c) => self.used_ctor(c, name.span),
                    ValueRef::Local(_) => {}
                }
                ExprKind::Name(v)
            }
            Some(_) => {
                self.error(
                    code::NOT_EXPORTED,
                    name.span,
                    format!("`{}` is not exported from `{path}`", name.name),
                );
                ExprKind::Error
            }
            None => {
                self.error(
                    code::NO_MEMBER,
                    name.span,
                    format!("`{path}` has no `{}`", name.name),
                );
                ExprKind::Error
            }
        }
    }

    fn name(&mut self, module: Option<&Ident>, name: &Ident) -> ExprKind {
        if let Some(m) = module {
            return match self.find(&m.name, m.span) {
                Some(Found::Private) => {
                    self.error(
                        code::NOT_EXPORTED,
                        m.span,
                        format!("`{}` is not exported from its scope module", m.name),
                    );
                    ExprKind::Error
                }
                Some(Found::Entry(Entry::Module(t))) => {
                    self.member(t, name, self.compiler_reference(m))
                }
                Some(Found::Entry(Entry::Broken)) => ExprKind::Error,
                Some(_) => {
                    self.error(
                        code::NOT_A_MODULE,
                        m.span,
                        format!("`{}` is a value, not a module", m.name),
                    );
                    ExprKind::Error
                }
                None => {
                    self.error(
                        code::UNKNOWN_NAME,
                        m.span,
                        format!("`{}` is not defined", m.name),
                    );
                    ExprKind::Error
                }
            };
        }
        match self.find(&name.name, name.span) {
            Some(Found::Private) => {
                self.error(
                    code::NOT_EXPORTED,
                    name.span,
                    format!("`{}` is not exported from its scope module", name.name),
                );
                ExprKind::Error
            }
            Some(Found::Local(v)) => ExprKind::Name(ValueRef::Local(v)),
            Some(Found::Entry(Entry::Def(d))) => {
                self.used_def(d, name.span);
                ExprKind::Name(ValueRef::Def(d))
            }
            Some(Found::Entry(Entry::Macro { .. })) => {
                self.error(
                    code::MACRO_RESERVED,
                    name.span,
                    format!(
                        "macro `{}` can only be used as a macro call (§4.1)",
                        name.name
                    ),
                );
                ExprKind::Error
            }
            Some(Found::Entry(Entry::Ctor(c))) => {
                self.used_ctor(c, name.span);
                ExprKind::Name(ValueRef::Ctor(c))
            }
            Some(Found::Entry(Entry::Broken)) => ExprKind::Error,
            Some(Found::Entry(Entry::Module(_))) => {
                self.error(
                    code::MODULE_AS_VALUE,
                    name.span,
                    format!(
                        "`{}` is a module, not a value; use a member, as in `{}.name`",
                        name.name, name.name
                    ),
                );
                ExprKind::Error
            }
            None => {
                self.error(
                    code::UNKNOWN_NAME,
                    name.span,
                    format!("`{}` is not defined", name.name),
                );
                ExprKind::Error
            }
        }
    }

    /// A constructor in a pattern or a record update.
    fn ctor(&mut self, module: Option<&Ident>, name: &Ident) -> Option<CtorRef> {
        if !is_upper(&name.name) {
            self.error(
                code::NOT_A_CONSTRUCTOR,
                name.span,
                format!("`{}` is not a constructor", name.name),
            );
            return None;
        }
        match self.name(module, name) {
            ExprKind::Name(ValueRef::Ctor(c)) => Some(c),
            ExprKind::Error => None,
            _ => {
                self.error(
                    code::NOT_A_CONSTRUCTOR,
                    name.span,
                    format!("`{}` is not a constructor", name.name),
                );
                None
            }
        }
    }


    // ---- Shape groups (syntax three §2.6) ----


    /// The selection table of a resolved callee, when it is a name with one.
    fn table_of(&self, callee: &Expr) -> Option<Table> {
        match &callee.kind {
            ExprKind::Name(ValueRef::Def(d)) => self.def_tables.get(d).cloned(),
            ExprKind::Name(ValueRef::Local(v)) => self.local_tables.get(v).cloned(),
            _ => None,
        }
    }


    /// The table that a local binding gives its one name: the binding is
    /// a multi chain over local group bindings, or a direct alias.
    fn local_table(&mut self, b: &Binding) {
        let PatternKind::Var(binder) = &b.pattern.kind else {
            return;
        };
        let table = match &b.value.kind {
            ExprKind::Multi(groups) => groups
                .iter()
                .map(|g| match &g.value.kind {
                    ExprKind::Name(v @ (ValueRef::Local(_) | ValueRef::Def(_))) => {
                        Some((g.shape, *v))
                    }
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .map(std::rc::Rc::new),
            _ => self.table_of(&b.value),
        };
        if let Some(table) = table {
            self.local_tables.insert(binder.var, table);
        }
    }


    /// Provider contracts are classified after aliases and imports resolve.
    /// A syntactically bare alias can still hide an inference hole.
    fn open_provider_contract(&self, t: &Type) -> bool {
        let mut pending = vec![t.clone()];
        while let Some(t) = pending.pop() {
            if let Some(expanded) = self.expand_alias(&t) {
                pending.push(expanded);
                continue;
            }
            match t.kind {
                TypeKind::Hole | TypeKind::Var(_) | TypeKind::Of { .. } | TypeKind::Error => {
                    return true;
                }
                TypeKind::Named {
                    ty: TypeRef::Custom(id),
                    ..
                } if self.circular.contains(&id) => return true,
                TypeKind::Named { args, .. }
                | TypeKind::Tuple(args)
                | TypeKind::Union(args)
                | TypeKind::Inter(args)
                | TypeKind::Diff(args) => pending.extend(args),
                TypeKind::Record { fields } => pending.extend(fields.into_iter().map(|(_, t)| t)),
                TypeKind::Fn {
                    params,
                    ret,
                    err,
                    env,
                } => {
                    pending.extend(params.into_iter().map(|p| p.ty));
                    pending.push(*ret);
                    pending.extend(err.into_iter().map(|t| *t));
                    pending.extend(env.into_iter().map(|t| *t));
                }
                TypeKind::Lit(_) => {}
            }
        }
        false
    }


    fn param_name(&mut self, all: &[Binder], name: &str, span: Span) {
        if all
            .iter()
            .any(|b| self.name_key(&b.name, b.span) == self.name_key(name, span))
        {
            self.error(
                code::PARAMS_BIND_TWICE,
                span,
                format!("the parameters of this function bind `{name}` more than once"),
            );
        }
    }


    fn bind(
        &mut self,
        name: &str,
        span: Span,
        binders: &mut Vec<Binder>,
        shared: Option<&HashMap<NameKey, VarId>>,
    ) -> Binder {
        let key = self.name_key(name, span);
        self.check_binder(name, span, false, false);
        if binders
            .iter()
            .any(|b| self.name_key(&b.name, b.span) == key)
        {
            self.error(
                code::PATTERN_BINDS_TWICE,
                span,
                format!("this pattern binds `{name}` more than once"),
            );
        }
        let var = match shared.and_then(|s| s.get(&key)) {
            Some(v) => *v,
            None => self
                .checked_inputs
                .and_then(|c| c.binders.get(&span))
                .copied()
                .unwrap_or_else(|| self.new_var()),
        };
        let b = Binder {
            var,
            name: name.to_string(),
            span,
        };
        binders.push(b.clone());
        b
    }


    // ---- Phase 5: exported types (§2.4) ----

    /// §2.4: if the type of an exported definition uses a type defined in
    /// this module, that type is exported too. This checks the types that
    /// are written: the fields of a type exported with its constructors
    /// (`T(..)`; a type exported alone does not show its fields), the body
    /// of an exported alias, the type of an exported declaration, and the
    /// annotation at the top of the pattern of an exported binding. An
    /// alias of this module that is not exported is looked through.
    ///
    /// The inferred type of an exported binding with no written type at
    /// the top of its pattern is checked by type inference.
    fn exported_types(&mut self, m: ModuleId) {
        self.cur = m;
        let module = self.p.module(m);
        let mut checks: Vec<(String, Type)> = Vec::new();
        for &t in &module.types {
            let def = self.p.ty(t);
            if !def.exported {
                continue;
            }
            match &def.body {
                TypeBody::Alias(body) => {
                    checks.push((format!("the alias `{}`", def.name), body.clone()))
                }
                TypeBody::Custom(ctors) if def.constructors_exported => {
                    for c in ctors {
                        for f in &c.fields {
                            checks.push((format!("the constructor `{}`", c.name), f.ty.clone()));
                        }
                    }
                }
                TypeBody::Custom(_) => {}
            }
        }
        let mut exported_vars = HashMap::new();
        for d in &module.defs {
            if !d.exported {
                continue;
            }
            match &d.kind {
                DefKind::External { ty, .. } => checks.push((format!("`{}`", d.name), ty.clone())),
                DefKind::Binding { var, .. } => {
                    exported_vars.insert(*var, d.name.clone());
                }
            }
        }
        for b in &module.bindings {
            top_annotations(&b.binding.pattern, &exported_vars, &mut checks);
        }
        for (what, t) in checks {
            if value_dependent(&self.p, &t) {
                continue;
            }
            let mut found = Vec::new();
            self.hidden_types(m, &t, None, &mut HashSet::new(), &mut found);
            for (span, name, foreign) in found {
                self.error(
                    code::EXPORT_HIDDEN_TYPE,
                    span,
                    format!(
                        "{what} is exported, but its type uses the type `{name}` {}, which is not exported (§2.4)", if foreign { "of its defining module" } else { "of this module" }
                    ),
                );
            }
        }
    }

    /// The types of module `m` that are not exported and that `t` uses,
    /// with the place of their use. `at` is the place of the use of an
    /// alias that is being looked through.
    fn hidden_types(
        &self,
        m: ModuleId,
        t: &Type,
        at: Option<Span>,
        seen: &mut HashSet<TypeId>,
        out: &mut Vec<(Span, String, bool)>,
    ) {
        if let TypeKind::Named {
            ty: TypeRef::Custom(id),
            ..
        } = &t.kind
        {
            let def = self.p.ty(*id);
            if !def.exported {
                match &def.body {
                    TypeBody::Alias(body) => {
                        if !self.circular.contains(id) && seen.insert(*id) {
                            self.hidden_types(m, body, Some(at.unwrap_or(t.span)), seen, out);
                            seen.remove(id);
                        }
                    }
                    TypeBody::Custom(_) => {
                        out.push((at.unwrap_or(t.span), def.name.clone(), def.module != m))
                    }
                }
            }
        }
        for_each_child(t, &mut |c| self.hidden_types(m, c, at, seen, out));
    }

    // ---- Phase 6: rules about the whole program ----

    fn program_rules(&mut self) {
        if let Some(root) = self.graph.root {
            let pkg = &self.graph.packages[root];
            let mut bad = Vec::new();
            if let Some(m) = self.by_path.get(&(root, format!("{}/main", pkg.name)))
                && let Some(&index) = self.defs_by_name[m.0 as usize].get("main")
            {
                self.p.declared_entries.push(DefId { module: *m, index });
            }
            for (name, e) in &pkg.entries {
                let valid = self
                    .by_path
                    .get(&(root, e.module.clone()))
                    .is_some_and(|m| {
                        self.defs_by_name[m.0 as usize]
                            .get(&e.function)
                            .is_some_and(|i| {
                                matches!(
                                    self.p.module(*m).defs[*i as usize].kind,
                                    DefKind::Binding { .. }
                                )
                            })
                    });
                if valid {
                    let m = self.by_path[&(root, e.module.clone())];
                    let index = self.defs_by_name[m.0 as usize][&e.function];
                    self.p.declared_entries.push(DefId { module: m, index });
                } else {
                    bad.push((
                        e.span,
                        format!("entry `{name}` must name a top-level function of this package"),
                    ));
                }
            }
            if let Some(name) = &self.graph.entry
                && !pkg.entries.iter().any(|(n, _)| n == name)
            {
                let names = pkg
                    .entries
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                bad.push((
                    Span::default(),
                    format!(
                        "unknown entry `{name}`; declared entries: {}",
                        if names.is_empty() { "(none)" } else { &names }
                    ),
                ));
            }
            for (span, message) in bad {
                self.diags.push(Diagnostic::error(
                    code::ENTRY_FORM,
                    pkg.manifest_source,
                    span,
                    message,
                ));
            }
        }
        self.host_fns();
        // §9.7: two host services may not have the same namespace and a
        // field with the same label.
        // The error names the service that has the label first.
        let mut seen: HashMap<(String, String), usize> = HashMap::new();
        for t in 0..self.p.types.len() {
            let def = &self.p.types[t];
            let (Some(host), TypeBody::Custom(ctors)) = (&def.host, &def.body) else {
                continue;
            };
            let mut twice = Vec::new();
            for c in ctors {
                for f in &c.fields {
                    let Some(label) = &f.label else { continue };
                    let key = (host.namespace.clone(), label.name.clone());
                    match seen.get(&key) {
                        Some(&other) => twice.push((label.span, label.name.clone(), other)),
                        None => {
                            seen.insert(key, t);
                        }
                    }
                }
            }
            let (m, ns) = (def.module, host.namespace.clone());
            for (span, label, other) in twice {
                let other = &self.p.types[other];
                let message = format!(
                    "the host service `{}` of `{}` also has the namespace `{ns}` and a field `{label}`",
                    other.name,
                    self.module_name(other.module)
                );
                self.cur = m;
                self.error(code::HOST_TWICE, span, message);
            }
        }
        self.wasm_exports();
    }

    /// §10.2: a host export names an exported function of a module.
    fn wasm_exports(&mut self) {
        let Some(root) = self.graph.root else {
            return;
        };
        self.p.root = Some(root);
        let pkg = &self.graph.packages[root];
        let Some(source) = pkg.manifest_source else {
            return;
        };
        let mut names = HashSet::new();
        for e in &pkg.wasm_exports {
            let (first, rest) = e.module.split_once('/').unwrap_or((&e.module, ""));
            let package = if first == pkg.name {
                Some(root)
            } else {
                pkg.dependencies
                    .iter()
                    .find(|(n, _)| n == first)
                    .and_then(|(_, id)| *id)
            };
            let module = package.and_then(|p| {
                let path = format!("{}/{rest}", self.graph.packages[p].name);
                self.by_path
                    .get(&(p, path))
                    .copied()
                    .filter(|&m| self.p.module(m).kind == ModuleKind::Src)
            });
            let def = module.and_then(|m| {
                self.defs_by_name[m.0 as usize]
                    .get(&e.function)
                    .map(|&d| DefId {
                        module: m,
                        index: d,
                    })
            });
            let mut problem = None;
            match def {
                Some(d) if self.p.def(d).exported => {
                    let def = self.p.def(d);
                    // Only the entry and tests of the compiled package
                    // are entry points; the others are not used (§10.2).
                    if let DefKind::Binding { binding, .. } = def.kind
                        && self.p.module(d.module).package == root
                    {
                        let b = &self.p.module(d.module).bindings[binding as usize];
                        if b.main {
                            problem = Some((
                                code::WASM_EXPORT_ENTRY,
                                format!(
                                    "`{}.{}` is a host export and also the program entry, so their export names would be the same",
                                    e.module, e.function
                                ),
                            ));
                        }
                    }
                }
                _ => {
                    problem = Some((
                        code::WASM_EXPORT_UNKNOWN,
                        format!(
                            "the host export `{}.{}` is not a function in the exports of a module `{}`",
                            e.module, e.function, e.module
                        ),
                    ))
                }
            }
            // B.5: a function named two times has two exports with one
            // name.
            if problem.is_none() && !names.insert((e.module.as_str(), e.function.as_str())) {
                problem = Some((
                    code::EXPORT_NAME_TWICE,
                    format!(
                        "`{}.{}` is named more than once in `[wasm].exports`, so their export names would be the same (B.5)",
                        e.module, e.function
                    ),
                ));
            }
            match (problem, def) {
                (Some((code, message)), _) => {
                    self.diags
                        .push(Diagnostic::error(code, Some(source), e.span, message));
                }
                (None, Some(d)) => self.p.host_exports.push(d),
                (None, None) => {}
            }
        }
    }
}

/// The custom types and aliases named in a type.
fn named_types(t: &Type, out: &mut Vec<TypeId>) {
    match &t.kind {
        TypeKind::Named { ty, args } => {
            if let TypeRef::Custom(id) = ty {
                out.push(*id);
            }
            args.iter().for_each(|a| named_types(a, out));
        }
        TypeKind::Tuple(ts) | TypeKind::Union(ts) | TypeKind::Inter(ts) | TypeKind::Diff(ts) => {
            ts.iter().for_each(|a| named_types(a, out))
        }
        TypeKind::Record { fields } => {
            for (_, field) in fields {
                named_types(field, out);
            }
        }
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => {
            params.iter().for_each(|p| named_types(&p.ty, out));
            named_types(ret, out);
            err.iter().chain(env).for_each(|a| named_types(a, out));
        }
        TypeKind::Lit(_)
        | TypeKind::Of { .. }
        | TypeKind::Var(_)
        | TypeKind::Hole
        | TypeKind::Error => {}
    }
}


/// Calls `f` on each type directly inside `t`.
fn for_each_child(t: &Type, f: &mut dyn FnMut(&Type)) {
    match &t.kind {
        TypeKind::Named { args, .. } => args.iter().for_each(f),
        TypeKind::Record { fields } => {
            for (_, field) in fields {
                f(field);
            }
        }
        TypeKind::Tuple(ts) | TypeKind::Union(ts) | TypeKind::Inter(ts) | TypeKind::Diff(ts) => {
            ts.iter().for_each(f)
        }
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => {
            params.iter().for_each(|p| f(&p.ty));
            f(ret);
            err.iter().chain(env).for_each(|a| f(a));
        }
        TypeKind::Lit(_)
        | TypeKind::Of { .. }
        | TypeKind::Var(_)
        | TypeKind::Hole
        | TypeKind::Error => {}
    }
}

/// The place of the first part of `t`, `t` itself included, whose kind
/// `is` says yes to.
fn find_in_type(t: &Type, is: &dyn Fn(&TypeKind) -> bool) -> Option<Span> {
    if is(&t.kind) {
        return Some(t.span);
    }
    let mut found = None;
    for_each_child(t, &mut |c| {
        if found.is_none() {
            found = find_in_type(c, is);
        }
    });
    found
}

/// `t` with the type variables in `map` replaced.
pub(crate) fn subst(t: &Type, map: &HashMap<TyVarId, &Type>) -> Type {
    let kind = match &t.kind {
        TypeKind::Lit(v) => TypeKind::Lit(v.clone()),
        TypeKind::Var(v) => match map.get(v) {
            Some(r) => return (*r).clone(),
            None => TypeKind::Var(*v),
        },
        TypeKind::Named { ty, args } => TypeKind::Named {
            ty: *ty,
            args: args.iter().map(|a| subst(a, map)).collect(),
        },
        TypeKind::Tuple(ts) => TypeKind::Tuple(ts.iter().map(|a| subst(a, map)).collect()),
        TypeKind::Record { fields } => TypeKind::Record {
            fields: fields
                .iter()
                .map(|(label, field)| (label.clone(), subst(field, map)))
                .collect(),
        },
        TypeKind::Union(ts) => TypeKind::Union(ts.iter().map(|a| subst(a, map)).collect()),
        TypeKind::Inter(ts) => TypeKind::Inter(ts.iter().map(|a| subst(a, map)).collect()),
        TypeKind::Diff(ts) => TypeKind::Diff(ts.iter().map(|a| subst(a, map)).collect()),
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => TypeKind::Fn {
            params: params
                .iter()
                .map(|p| FnParamType {
                    label: p.label.clone(),
                    ty: subst(&p.ty, map),
                })
                .collect(),
            ret: Box::new(subst(ret, map)),
            err: err.as_ref().map(|e| Box::new(subst(e, map))),
            env: env.as_ref().map(|e| Box::new(subst(e, map))),
        },
        TypeKind::Of {
            value,
            site,
            projection,
        } => TypeKind::Of {
            value: *value,
            site: *site,
            projection: *projection,
        },
        TypeKind::Hole => TypeKind::Hole,
        TypeKind::Error => TypeKind::Error,
    };
    Type { kind, span: t.span }
}

/// §2.4: the annotations at the top of the pattern of a top-level binding
/// that give the type of an exported name: `x: T`, and `x = y: T`.
fn top_annotations(p: &Pattern, exported: &HashMap<VarId, String>, out: &mut Vec<(String, Type)>) {
    fn top_type(p: &Pattern) -> Option<&Type> {
        match &p.kind {
            PatternKind::Typed { ty, .. } => Some(ty),
            PatternKind::Named { pattern, .. } => top_type(pattern),
            _ => None,
        }
    }
    let mut check = |b: &Binder, ty: Option<&Type>| {
        if let (Some(name), Some(ty)) = (exported.get(&b.var), ty) {
            out.push((format!("`{name}`"), ty.clone()));
        }
    };
    match &p.kind {
        PatternKind::Typed {
            binder: Some(b),
            ty,
        } => check(b, Some(ty)),
        PatternKind::Named { binder, pattern } => {
            check(binder, top_type(pattern));
            top_annotations(pattern, exported, out);
        }
        PatternKind::Constructor { args: ps, .. }
        | PatternKind::Tuple(ps)
        | PatternKind::Error(ps)
        | PatternKind::List { items: ps, .. } => {
            ps.iter().for_each(|a| top_annotations(a, exported, out))
        }
        PatternKind::BitArray(segs) => segs
            .iter()
            .for_each(|s| top_annotations(&s.value, exported, out)),
        _ => {}
    }
}

fn pattern_type_ids(p: &TypePattern, out: &mut Vec<TypeId>) {
    match p {
        TypePattern::Named(ty, parts) => {
            if let TypeRef::Custom(id) = ty {
                out.push(*id);
            }
            for p in parts {
                pattern_type_ids(p, out);
            }
        }
        TypePattern::Tuple(parts) => {
            for p in parts {
                pattern_type_ids(p, out);
            }
        }
        TypePattern::Exact(t) => named_types(t, out),
        TypePattern::Bind(_) | TypePattern::Skip | TypePattern::Error => {}
    }
}

fn projectable_body(
    t: &Type,
    map: &HashMap<TyVarId, TypePattern>,
    used: &mut HashSet<TyVarId>,
) -> Option<TypePattern> {
    let mut variable = false;
    let mut invalid = false;
    walk_type(t, &mut |t| match t.kind {
        TypeKind::Var(_) => variable = true,
        TypeKind::Hole | TypeKind::Error | TypeKind::Of { .. } => invalid = true,
        _ => {}
    });
    if invalid {
        return None;
    }
    if !variable {
        return Some(TypePattern::Exact(t.clone()));
    }
    match &t.kind {
        TypeKind::Var(v) => {
            if !used.insert(*v) {
                return None;
            }
            map.get(v).cloned()
        }
        TypeKind::Named { ty, args } => Some(TypePattern::Named(
            *ty,
            args.iter()
                .map(|t| projectable_body(t, map, used))
                .collect::<Option<_>>()?,
        )),
        TypeKind::Tuple(ts) => Some(TypePattern::Tuple(
            ts.iter()
                .map(|t| projectable_body(t, map, used))
                .collect::<Option<_>>()?,
        )),
        TypeKind::Lit(_)
        | TypeKind::Record { .. }
        | TypeKind::Fn { .. }
        | TypeKind::Union(_)
        | TypeKind::Inter(_)
        | TypeKind::Diff(_)
        | TypeKind::Hole
        | TypeKind::Error
        | TypeKind::Of { .. } => None,
    }
}


#[derive(Clone, Debug)]
pub(crate) struct ExpansionScope {
    locals: Vec<Frame>,
    ty_frames: Vec<TyFrame>,
    fn_depth: u32,
    pending: Vec<HashSet<NameKey>>,
    reserved: Vec<(usize, HashSet<NameKey>)>,
}
#[derive(Clone, Debug)]
pub(crate) struct ExpansionNames {
    generated_values: Vec<HashMap<NameKey, Entry>>,
    generated_types: Vec<HashMap<NameKey, TypeId>>,
    defs_by_name: Vec<HashMap<String, u32>>,
    macros_by_name: Vec<HashMap<String, bool>>,
    types_by_name: Vec<HashMap<String, TypeId>>,
    ctors_by_name: Vec<HashMap<String, CtorRef>>,
    slots: Vec<Vec<Slot>>,
    scopes: Vec<Scope>,
    by_path: HashMap<(PackageId, String), ModuleId>,
    builtin_values: HashMap<String, Entry>,
    builtin_types: HashMap<String, TypeRef>,
}
#[derive(Default)]
pub struct CheckedInputs {
    pub expressions: HashMap<Span, Expr>,
    pub binders: HashMap<Span, VarId>,
}
/// Resolve a typed macro result directly from its native expression.
#[allow(clippy::too_many_arguments)]
pub fn typed_output(
    graph: &PackageGraph,
    parsed: &[crate::Parsed],
    original: &Program,
    module: ModuleId,
    call: Span,
    expression: &ilang_syntax::syn3::ast::Expr,
    var_count: u32,
    ty_var_count: u32,
    checked: &CheckedInputs,
) -> Result<(Expr, Program), Vec<Diagnostic>> {
    // A multi-shape chain has a synthetic one-byte root in the transition
    // tree. Keep checked inputs at that root as well as their native span.
    struct Inputs<'a>(&'a mut CheckedInputs);
    impl ilang_syntax::syn3::walk::Visitor for Inputs<'_> {
        fn expr(&mut self, e: &mut ilang_syntax::syn3::ast::Expr, enter: bool) {
            use ilang_syntax::syn3::ast::ExprKind;
            // Only a multi-shape chain changes its root span. Computing
            // this from its head avoids lowering every forwarded subtree.
            if enter
                && let ExprKind::Chain(branches) = &e.kind
                && branches.iter().any(|branch| {
                    branch
                        .params
                        .iter()
                        .any(|parameter| parameter.key.is_some())
                        || branches
                            .first()
                            .is_some_and(|first| first.params.len() != branch.params.len())
                })
                && let Some(old) = self.0.expressions.get(&e.span.0).cloned()
            {
                let span = e.span.0;
                self.0
                    .expressions
                    .insert(Span::new(span.end.saturating_sub(1), span.end), old);
            }
        }
    }
    let mut forwarded = CheckedInputs {
        expressions: checked.expressions.clone(),
        binders: checked.binders.clone(),
    };
    ilang_syntax::syn3::deep::deep(|| {
        ilang_syntax::syn3::walk::expr(&mut expression.clone(), &mut Inputs(&mut forwarded))
    });
    three::typed_output3(
        graph,
        parsed,
        original,
        module,
        call,
        expression,
        var_count,
        ty_var_count,
        &forwarded,
        &parsed[module.0 as usize]
            .scoped
            .as_ref()
            .map(|s| s.transition().clone())
            .unwrap_or_default(),
    )
}


pub(crate) fn normalize_source(
    module: &ilang_syntax::syn3::ast::Module,
) -> ilang_syntax::syn3::ast::Module {
    three::normalize_module(module, &mut Default::default())
}
