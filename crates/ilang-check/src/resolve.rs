//! Name resolution (§2, §5, §6.10, §6.11, §7.2, §10): builds the HIR and
//! reports every name rule as a compile error.
//!
//! The work is done in phases, each over all modules:
//! 1. collect the top-level definitions, attributes, and exports;
//! 2. resolve the imports, and find import circles;
//! 3. resolve the type definitions, and find alias circles;
//! 4. resolve the declarations and bindings;
//! 5. check that exported definitions use only exported types (§2.4);
//! 6. check the rules about the whole program (`@main`, host services,
//!    `[wasm] exports`).

use crate::Parsed;
use crate::code;
use crate::hir::*;
use crate::types::HostType;
use ilang_package::{Diagnostic, ModuleKind, PackageGraph, PackageId};
use ilang_syntax::ast;
use ilang_syntax::span::Span;
use std::collections::{HashMap, HashSet};

pub fn resolve(graph: &PackageGraph, parsed: &[Parsed]) -> (Program, Vec<Diagnostic>) {
    let mut r = Resolver {
        graph,
        parsed,
        p: Program::default(),
        diags: Vec::new(),
        defs_by_name: Vec::new(),
        types_by_name: Vec::new(),
        ctors_by_name: Vec::new(),
        slots: Vec::new(),
        scopes: Vec::new(),
        by_path: HashMap::new(),
        builtin_values: HashMap::new(),
        builtin_types: HashMap::new(),
        mains: Vec::new(),
        cur: ModuleId(0),
        locals: Vec::new(),
        fn_depth: 0,
        ty_frames: Vec::new(),
        ty_def: None,
        circular: HashSet::new(),
    };
    r.create_modules();
    for m in 0..parsed.len() {
        r.collect(ModuleId(m as u32));
    }
    r.builtins();
    for m in 0..parsed.len() {
        r.imports(ModuleId(m as u32));
    }
    r.import_circles();
    for m in 0..parsed.len() {
        r.type_defs(ModuleId(m as u32));
    }
    r.alias_circles();
    for m in 0..parsed.len() {
        r.values(ModuleId(m as u32));
    }
    for m in 0..parsed.len() {
        r.exported_types(ModuleId(m as u32));
    }
    r.program_rules();
    (r.p, r.diags)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Def(DefId),
    Ctor(CtorRef),
    Module(ModuleId),
    /// A name from an import that failed; the problem has been reported,
    /// so its uses are not reported again.
    Broken,
}

/// The top-level names of a module: its own and the imported ones. The
/// `bool` says that the name is imported.
#[derive(Default)]
struct Scope {
    values: HashMap<String, (Entry, bool)>,
    types: HashMap<String, (TypeRef, bool)>,
    /// Type names from imports that failed.
    broken_types: HashSet<String>,
}

/// What an item of a module became in phase 1.
#[derive(Clone, Copy)]
enum Slot {
    Type(TypeId),
    Binding(u32),
    Decl(u32),
    None,
}

/// Local names bound by one binding, parameter list, or clause.
struct Frame {
    names: HashMap<String, VarId>,
    /// `Some(d)`: the names may only be used inside function bodies that
    /// are deeper than `d` (§6.10, the value of a direct binding).
    fn_only: Option<u32>,
}

struct TyFrame {
    names: HashMap<String, TyVarId>,
}

struct TyDefMode {
    params: HashMap<String, TyVarId>,
    used: HashSet<TyVarId>,
}

enum Found {
    Local(VarId),
    Entry(Entry),
}

/// The attributes of one item, after they are checked.
#[derive(Default)]
struct Attrs {
    external: Option<(String, String)>,
    host: Option<Host>,
    main: bool,
    test: bool,
    deprecated: Option<String>,
}

struct Resolver<'a> {
    graph: &'a PackageGraph,
    parsed: &'a [Parsed],
    p: Program,
    diags: Vec<Diagnostic>,
    defs_by_name: Vec<HashMap<String, u32>>,
    types_by_name: Vec<HashMap<String, TypeId>>,
    ctors_by_name: Vec<HashMap<String, CtorRef>>,
    slots: Vec<Vec<Slot>>,
    scopes: Vec<Scope>,
    by_path: HashMap<(PackageId, String), ModuleId>,
    /// The built-in names (§12): the prelude and the built-in types.
    builtin_values: HashMap<String, Entry>,
    builtin_types: HashMap<String, TypeRef>,
    /// `@main` bindings in the compiled package.
    mains: Vec<(ModuleId, Span)>,
    // The state of the module and body being resolved.
    cur: ModuleId,
    locals: Vec<Frame>,
    fn_depth: u32,
    ty_frames: Vec<TyFrame>,
    ty_def: Option<TyDefMode>,
    /// The aliases in a circle (§4.3); the circle has been reported, and
    /// they are not expanded.
    circular: HashSet<TypeId>,
}

fn is_upper(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
}

/// The names a pattern binds, in order, with repeats (§7.2).
fn ast_binders(p: &ast::Pattern, out: &mut Vec<(String, Span)>) {
    use ast::PatternKind as P;
    match &p.kind {
        P::Var(n) => out.push((n.clone(), p.span)),
        P::Typed { binder, .. } if !binder.discard => out.push((binder.name.clone(), binder.span)),
        P::Named { name, pattern } => {
            out.push((name.name.clone(), name.span));
            ast_binders(pattern, out);
        }
        P::Prefix { rest, .. } if !rest.discard => out.push((rest.name.clone(), rest.span)),
        P::Constructor { args, .. } | P::Tuple(args) => {
            args.iter().for_each(|a| ast_binders(a, out));
        }
        P::List { items, rest } => {
            items.iter().for_each(|a| ast_binders(a, out));
            if let Some(Some(b)) = rest
                && !b.discard
            {
                out.push((b.name.clone(), b.span));
            }
        }
        P::BitArray(segs) => segs.iter().for_each(|s| ast_binders(&s.value, out)),
        _ => {}
    }
}

/// The label of a parameter (§6.11): the last name in the `=` chain at the
/// top of its pattern.
fn label_of(p: &ast::Pattern) -> Option<String> {
    use ast::PatternKind as P;
    match &p.kind {
        P::Var(n) => Some(n.clone()),
        P::Typed { binder, .. } if !binder.discard => Some(binder.name.clone()),
        P::Named { name, pattern } => label_of(pattern).or_else(|| Some(name.name.clone())),
        _ => None,
    }
}

/// `{ e }` with one expression is a group (§6.10): it is `e`.
fn ungroup(e: &ast::Expr) -> &ast::Expr {
    match &e.kind {
        ast::ExprKind::Block(b) if b.stmts.len() == 1 => match &b.stmts[0] {
            ast::Stmt::Expr(inner) => ungroup(inner),
            _ => e,
        },
        _ => e,
    }
}

impl<'a> Resolver<'a> {
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
            self.defs_by_name.push(HashMap::new());
            self.types_by_name.push(HashMap::new());
            self.ctors_by_name.push(HashMap::new());
            self.slots.push(Vec::new());
            self.scopes.push(Scope::default());
        }
    }

    /// A value name of this module may be defined only once (§2.2).
    fn value_free(&mut self, m: ModuleId, name: &str, span: Span) -> bool {
        let mi = m.0 as usize;
        if self.defs_by_name[mi].contains_key(name) || self.ctors_by_name[mi].contains_key(name) {
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
        self.defs_by_name[m.0 as usize].insert(name.to_string(), index);
    }

    fn collect(&mut self, m: ModuleId) {
        self.cur = m;
        let parsed = self.parsed;
        let ast = &parsed[m.0 as usize].ast;
        let placeholder_var = VarId(u32::MAX);
        for item in &ast.items {
            let attrs = self.attributes(item);
            let slot = match &item.kind {
                ast::ItemKind::TypeDef(def) => Slot::Type(self.collect_type(m, def, &attrs)),
                ast::ItemKind::Binding(b) => {
                    let index = self.p.module(m).bindings.len() as u32;
                    let mut names = Vec::new();
                    ast_binders(&b.pattern, &mut names);
                    let mut seen = HashSet::new();
                    let mut defs = Vec::new();
                    for (name, span) in names {
                        if !seen.insert(name.clone()) {
                            continue; // reported when the pattern is resolved
                        }
                        let kind = DefKind::Binding {
                            binding: index,
                            var: placeholder_var,
                        };
                        let before = self.p.module(m).defs.len();
                        self.add_def(m, &name, span, kind, &attrs);
                        if self.p.module(m).defs.len() > before {
                            defs.push(DefId {
                                module: m,
                                index: before as u32,
                            });
                        }
                    }
                    self.p.modules[m.0 as usize].bindings.push(TopBinding {
                        binding: Binding {
                            pattern: Pattern {
                                kind: PatternKind::Error(Vec::new()),
                                span: b.pattern.span,
                            },
                            value: Expr {
                                kind: ExprKind::Error,
                                span: b.value.span,
                            },
                            direct: false,
                            ty_vars: Vec::new(),
                            span: b.span,
                        },
                        defs,
                        main: attrs.main,
                        test: attrs.test,
                    });
                    Slot::Binding(index)
                }
                ast::ItemKind::MacroItem { name, args, .. } => {
                    let message = if args.is_empty()
                        && matches!(name.name.as_str(), "import" | "export")
                    {
                        "a call at the top level is kept for macros; if this was an old empty header, omit it"
                    } else {
                        "a call at the top level is kept for macros"
                    };
                    self.error(code::MACRO_RESERVED, item.span, message);
                    Slot::None
                }
                ast::ItemKind::Declaration {
                    name,
                    ty,
                    type_params: _,
                } => {
                    let (module, function) = attrs.external.clone().unwrap_or_default();
                    let kind = DefKind::External {
                        module,
                        function,
                        ty: Type {
                            kind: TypeKind::Error,
                            span: ty.span,
                        },
                        ty_vars: Vec::new(),
                    };
                    let before = self.p.module(m).defs.len();
                    self.add_def(m, &name.name, name.span, kind, &attrs);
                    if self.p.module(m).defs.len() > before {
                        Slot::Decl(before as u32)
                    } else {
                        Slot::None
                    }
                }
            };
            self.slots[m.0 as usize].push(slot);
        }
        self.exports(m, &ast.exports);
    }

    fn collect_type(&mut self, m: ModuleId, def: &ast::TypeDef, attrs: &Attrs) -> TypeId {
        let id = TypeId(self.p.types.len() as u32);
        let mi = m.0 as usize;
        if self.types_by_name[mi].contains_key(&def.name.name) {
            self.error(
                code::DEFINED_TWICE,
                def.name.span,
                format!(
                    "the type `{}` is defined more than once in this module",
                    def.name.name
                ),
            );
        } else {
            self.types_by_name[mi].insert(def.name.name.clone(), id);
        }
        // A repeated parameter is reported, but it keeps its place, so that
        // the parameters stay in line with the written ones and the number
        // of parameters is the written number. Its name refers to the first.
        let mut params: Vec<(String, TyVarId)> = Vec::new();
        for p in &def.params {
            if params.iter().any(|(n, _)| n == &p.name) {
                self.error(
                    code::TYPE_PARAM_TWICE,
                    p.span,
                    format!("the type parameter `{}` is given more than once", p.name),
                );
            }
            let v = self.new_ty_var(&p.name);
            params.push((p.name.clone(), v));
        }
        let body = match &def.body {
            ast::TypeBody::Alias(t) => TypeBody::Alias(Type {
                kind: TypeKind::Error,
                span: t.span,
            }),
            ast::TypeBody::Custom(ctors) => {
                let mut out = Vec::new();
                for c in ctors {
                    if !self.value_free(m, &c.name.name, c.name.span) {
                        continue;
                    }
                    let index = out.len() as u32;
                    self.ctors_by_name[mi].insert(c.name.name.clone(), CtorRef { ty: id, index });
                    out.push(CtorDef {
                        name: c.name.name.clone(),
                        span: c.name.span,
                        fields: Vec::new(),
                    });
                }
                TypeBody::Custom(out)
            }
        };
        self.p.types.push(TypeDef {
            name: def.name.name.clone(),
            span: def.name.span,
            module: m,
            params,
            body,
            exported: false,
            constructors_exported: false,
            host: attrs.host.clone(),
            deprecated: attrs.deprecated.clone(),
        });
        self.p.modules[mi].types.push(id);
        id
    }

    /// §2.2 and §10: the attributes of an item.
    fn attributes(&mut self, item: &ast::Item) -> Attrs {
        let mut out = Attrs::default();
        let mut seen = HashSet::new();
        let is_test_module = self.p.module(self.cur).kind == ModuleKind::Test;
        for a in &item.attributes {
            let name = a.name.name.as_str();
            let written = a
                .module
                .as_ref()
                .map_or_else(|| format!("@{name}"), |m| format!("@{}.{name}", m.name));
            if a.module.is_some()
                || !matches!(name, "external" | "host" | "main" | "test" | "deprecated")
            {
                self.error(code::UNKNOWN_ATTRIBUTE, a.span, format!("the attribute `{written}` is kept for macros; the built-in attributes are `@external`, `@host`, `@main`, `@test`, and `@deprecated`"));
                continue;
            }
            if !seen.insert(name) {
                self.error(
                    code::ATTRIBUTE_TWICE,
                    a.span,
                    format!("`@{name}` is given more than once on this item"),
                );
                continue;
            }
            let args: Vec<&str> = a.args.iter().map(|(s, _)| s.as_str()).collect();
            let (counts, place_ok, place): (&[usize], bool, &str) = match name {
                "external" => (
                    &[2],
                    matches!(item.kind, ast::ItemKind::Declaration { .. }),
                    "a declaration `name: Type`",
                ),
                "host" => (
                    &[1, 2],
                    matches!(item.kind, ast::ItemKind::TypeDef(_)),
                    "a type definition",
                ),
                "main" | "test" => (
                    &[0],
                    matches!(item.kind, ast::ItemKind::Binding(_)),
                    "a binding",
                ),
                "deprecated" => (&[1], true, "any item"),
                _ => unreachable!(),
            };
            let form = match name {
                "external" => r#"`@external("module", "function")`"#,
                "host" => r#"`@host("namespace")` or `@host("namespace", "async")`"#,
                "main" => "`@main`, with no arguments",
                "test" => "`@test`, with no arguments",
                _ => r#"`@deprecated("note")`"#,
            };
            if let Some((args, _)) = &a.call {
                let message = if counts == [0] && args.is_empty() {
                    format!("`@{name}` takes no arguments and is written with no `( )`")
                } else {
                    format!(
                        "the arguments of `@{name}` are string literals given by position: {form}"
                    )
                };
                self.error(code::ATTRIBUTE_ARGS, a.span, message);
                continue;
            }
            if !counts.contains(&args.len())
                || (name == "host" && args.len() == 2 && !matches!(args[1], "async" | "request"))
            {
                self.error(
                    code::ATTRIBUTE_ARGS,
                    a.span,
                    format!("this attribute is written {form}"),
                );
                continue;
            }
            if !place_ok {
                self.error(
                    code::ATTRIBUTE_PLACE,
                    a.span,
                    format!("`@{name}` may only be on {place}"),
                );
                continue;
            }
            match name {
                "external" | "host" => {
                    if args[0].starts_with("ilang:") && !self.in_library() {
                        self.error(
                            code::RESERVED_ILANG,
                            a.args[0].1,
                            "names that start with `ilang:` are kept for the runtime",
                        );
                    }
                    if name == "external" {
                        out.external = Some((args[0].to_string(), args[1].to_string()));
                    } else {
                        out.host = Some(Host {
                            namespace: args[0].to_string(),
                            is_async: args.len() == 2,
                            is_request: args.get(1).is_some_and(|m| *m == "request"),
                        });
                    }
                }
                "main" | "test" => {
                    let ast::ItemKind::Binding(b) = &item.kind else {
                        unreachable!()
                    };
                    let compiled = Some(self.p.module(self.cur).package) == self.graph.root;
                    if !self.entry_form(name, a.span, b, compiled) {
                        continue;
                    }
                    if name == "test" && !is_test_module {
                        self.error(
                            code::TEST_OUTSIDE_TEST,
                            a.span,
                            "`@test` may only be in a module under `test/`",
                        );
                        continue;
                    }
                    if name == "main" {
                        out.main = true;
                        if compiled {
                            self.mains.push((self.cur, b.pattern.span));
                        }
                    } else {
                        out.test = true;
                    }
                }
                _ => out.deprecated = Some(args[0].to_string()),
            }
        }
        // B.5: the program entry and a test have the export name `module
        // path.function name`, so one function may not be both.
        if out.main
            && out.test
            && Some(self.p.module(self.cur).package) == self.graph.root
            && let Some(a) = item
                .attributes
                .iter()
                .rfind(|a| matches!(a.name.name.as_str(), "main" | "test"))
        {
            self.error(
                code::EXPORT_NAME_TWICE,
                a.span,
                "this function is the program entry and also a test, so their export names would be the same (B.5)",
            );
        }
        if let ast::ItemKind::Declaration { name, .. } = &item.kind
            && out.external.is_none()
            && !item
                .attributes
                .iter()
                .any(|a| a.module.is_none() && a.name.name == "external")
        {
            self.error(
                code::DECLARATION_WITHOUT_EXTERNAL,
                name.span,
                "a declaration with no `@external` is kept for macros",
            );
        }
        out
    }

    /// §2.2: `@main` and `@test` are on a binding whose pattern binds one
    /// name and whose value is a function expression. §10.2: in the
    /// compiled package, they are entry points, which have no parameters.
    /// `@main` in other packages is not used, so only §2.2 is checked there.
    fn entry_form(&mut self, name: &str, span: Span, b: &ast::Binding, compiled: bool) -> bool {
        let mut names = Vec::new();
        ast_binders(&b.pattern, &mut names);
        let ast::ExprKind::Fn(f) = &ungroup(&b.value).kind else {
            self.error(
                code::ENTRY_FORM,
                span,
                format!("`@{name}` is on a binding whose value is a function expression"),
            );
            return false;
        };
        if names.len() != 1 {
            self.error(
                code::ENTRY_FORM,
                span,
                format!("`@{name}` is on a binding whose pattern binds one name"),
            );
            return false;
        }
        if compiled && f.params.iter().any(|p| matches!(p, ast::Param::Pattern(_))) {
            self.error(
                code::ENTRY_PARAMS,
                span,
                format!("an entry point has no parameters but environment parameters, so `@{name}` may not be on this function"),
            );
            return false;
        }
        true
    }

    /// §2.4.
    fn exports(&mut self, m: ModuleId, items: &[ast::ExportItem]) {
        let mi = m.0 as usize;
        for item in items {
            let name = &item.name.name;
            if !is_upper(name) {
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

    fn imports(&mut self, m: ModuleId) {
        self.cur = m;
        let mi = m.0 as usize;
        let mut scope = Scope::default();
        for (name, &d) in &self.defs_by_name[mi] {
            let entry = Entry::Def(DefId {
                module: m,
                index: d,
            });
            scope.values.insert(name.clone(), (entry, false));
        }
        for (name, &c) in &self.ctors_by_name[mi] {
            scope.values.insert(name.clone(), (Entry::Ctor(c), false));
        }
        for (name, &t) in &self.types_by_name[mi] {
            scope
                .types
                .insert(name.clone(), (TypeRef::Custom(t), false));
        }
        let parsed = self.parsed;
        for item in &parsed[mi].ast.imports {
            let alias = item.alias.as_ref().unwrap_or(item.path.last().unwrap());
            let Some(target) = self.import_target(item) else {
                for n in item.names.iter().chain([alias]) {
                    scope
                        .values
                        .entry(n.name.clone())
                        .or_insert((Entry::Broken, true));
                    if is_upper(&n.name) {
                        scope.broken_types.insert(n.name.clone());
                    }
                }
                continue;
            };
            self.p.modules[mi].imports.push(target);
            self.p.modules[mi]
                .import_names
                .push((alias.name.clone(), target));
            self.put_value(&mut scope, &alias.name, alias.span, Entry::Module(target));
            for n in &item.names {
                self.import_name(&mut scope, target, n);
            }
        }
        self.scopes[mi] = scope;
    }

    /// The module that an import path names (§2.1).
    fn import_target(&mut self, item: &ast::ImportItem) -> Option<ModuleId> {
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
        let found = self
            .by_path
            .get(&(package, path))
            .copied()
            .filter(|&t| self.p.module(t).kind != ModuleKind::Prelude);
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

    fn put_type(&mut self, scope: &mut Scope, name: &str, span: Span, t: TypeId) {
        match scope.types.get(name) {
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
                scope
                    .types
                    .insert(name.to_string(), (TypeRef::Custom(t), true));
                self.p.modules[self.cur.0 as usize].imported_types.push(t);
            }
        }
    }

    /// `(A, b) = path` (§2.3). Only exported names may be imported.
    fn import_name(&mut self, scope: &mut Scope, target: ModuleId, n: &ast::Ident) {
        let ti = target.0 as usize;
        let module = self.module_name(target).to_string();
        if !is_upper(&n.name) {
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
        let ty = self.types_by_name[ti].get(&n.name).copied();
        let ctor = self.ctors_by_name[ti].get(&n.name).copied();
        let ty_ok = ty.filter(|t| self.p.ty(*t).exported);
        let ctor_ok = ctor.filter(|c| self.p.ty(c.ty).constructors_exported);
        if let Some(t) = ty_ok {
            self.put_type(scope, &n.name, n.span, t);
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

    /// §2.1: modules may not import one another in a circle.
    fn import_circles(&mut self) {
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
            let parsed = self.parsed;
            for &m in &members {
                self.cur = ModuleId(m);
                // Report at the first import that goes into the circle.
                let item = parsed[m as usize].ast.imports.iter().find(|item| {
                    let me = self.p.module(ModuleId(m));
                    let first = &item.path[0].name;
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
                    let rest: Vec<&str> = item.path[1..].iter().map(|p| p.name.as_str()).collect();
                    package.is_some_and(|p| {
                        let path = format!("{}/{}", self.graph.packages[p].name, rest.join("/"));
                        self.by_path
                            .get(&(p, path))
                            .is_some_and(|t| members.contains(&t.0))
                    })
                });
                let span = item.map_or(Span::default(), |i| i.span);
                self.error(
                    code::IMPORT_CIRCLE,
                    span,
                    format!("these modules import one another in a circle: {names}"),
                );
            }
        }
    }

    // ---- Phase 3: type definitions (§4.3) ----

    fn type_defs(&mut self, m: ModuleId) {
        self.cur = m;
        let parsed = self.parsed;
        let ast = &parsed[m.0 as usize].ast;
        for (item, slot) in ast.items.iter().zip(self.slots[m.0 as usize].clone()) {
            let (ast::ItemKind::TypeDef(def), Slot::Type(id)) = (&item.kind, slot) else {
                continue;
            };
            // §4.3: read a reserved head in full, but do not resolve its body.
            let alias = matches!(def.body, ast::TypeBody::Alias(_));
            if !def.head.is_empty() {
                self.error(
                    if alias { code::ALIAS_PATTERN_RESERVED } else { code::TYPE_PARAM_FORM },
                    def.head[0].span,
                    if alias { "the left side of an alias that is not a list of type parameters is kept for alias patterns" }
                    else { "a type parameter is an `UpName` with no module name and no arguments" },
                );
                if def
                    .head
                    .iter()
                    .any(|t| matches!(t.kind, ast::TypeKind::ValueOf { module: None, .. }))
                {
                    self.diags.last_mut().unwrap().message.push_str("; old lowercase type parameters are now declared as UpNames, as in `Box(A) { Box(A) }` or `Id(A) = A`");
                }
                continue;
            }
            let mut clashes = false;
            for param in &def.params {
                if is_upper(&param.name) && self.unqualified_type(&param.name).is_some() {
                    clashes = true;
                    self.error(
                        if alias { code::ALIAS_PATTERN_RESERVED } else { code::TYPE_VAR_IS_TYPE },
                        param.span,
                        if alias { "the left side of an alias that is not a list of type parameters is kept for alias patterns" }
                        else { "a type parameter may not have the name of a type" },
                    );
                }
            }
            if clashes {
                continue;
            }
            let params = self.p.ty(id).params.clone();
            let mut by_name = HashMap::new();
            for (n, v) in &params {
                by_name.entry(n.clone()).or_insert(*v);
            }
            self.ty_def = Some(TyDefMode {
                params: by_name,
                used: HashSet::new(),
            });
            match &def.body {
                ast::TypeBody::Alias(t) => {
                    let t = self.ty(t);
                    self.p.types[id.0 as usize].body = TypeBody::Alias(t);
                }
                ast::TypeBody::Custom(ctors) => {
                    for c in ctors {
                        let fields: Vec<FieldDef> = c
                            .fields
                            .iter()
                            .map(|f| FieldDef {
                                label: f.label.clone(),
                                ty: self.ty(&f.ty),
                            })
                            .collect();
                        if let TypeBody::Custom(defs) = &mut self.p.types[id.0 as usize].body
                            && let Some(d) = defs.iter_mut().find(|d| d.span == c.name.span)
                        {
                            d.fields = fields;
                        }
                    }
                }
            }
            let mode = self.ty_def.take().unwrap();
            for (i, (p, ident)) in params.iter().zip(&def.params).enumerate() {
                let repeated = params[..i].iter().any(|q| q.0 == p.0);
                if !repeated && !mode.used.contains(&p.1) {
                    self.error(
                        code::TYPE_PARAM_UNUSED,
                        ident.span,
                        format!("the type parameter `{}` is not used", p.0),
                    );
                }
            }
            if self.p.ty(id).host.is_some() {
                self.host_form(def);
            }
        }
    }

    /// §9.7: a host service has one constructor, and every field has a
    /// label and a function type.
    fn host_form(&mut self, def: &ast::TypeDef) {
        let ok = match &def.body {
            ast::TypeBody::Custom(ctors) if ctors.len() == 1 => ctors[0]
                .fields
                .iter()
                .all(|f| f.label.is_some() && matches!(f.ty.kind, ast::TypeKind::Fn { .. })),
            _ => false,
        };
        if !ok {
            self.error(
                code::HOST_FORM,
                def.name.span,
                "a host service has exactly one constructor, and every field has a label and a function type (§9.7)",
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

    fn lookup_type(&mut self, module: Option<&ast::Ident>, name: &ast::Ident) -> Option<TypeRef> {
        // §2.3: a local name may cover a module name, in types too.
        if let Some(m) = module
            && let Some(Found::Local(_)) = self.find(&m.name)
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
        let scope = &self.scopes[self.cur.0 as usize];
        let broken = match module {
            Some(m) => matches!(scope.values.get(&m.name), Some((Entry::Broken, _))),
            None => {
                scope.broken_types.contains(&name.name) && !scope.types.contains_key(&name.name)
            }
        };
        if broken {
            return None;
        }
        if let Some(m) = module {
            let target = self.module_of(m)?;
            let ti = target.0 as usize;
            let path = self.module_name(target).to_string();
            return match self.types_by_name[ti].get(&name.name).copied() {
                Some(t) if self.p.ty(t).exported => {
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
        let scope = &self.scopes[self.cur.0 as usize];
        let found = scope
            .types
            .get(&name.name)
            .map(|(t, _)| *t)
            .or_else(|| self.builtin_types.get(&name.name).copied());
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
    fn module_of(&mut self, m: &ast::Ident) -> Option<ModuleId> {
        match self.scopes[self.cur.0 as usize].values.get(&m.name) {
            Some((Entry::Module(t), _)) => Some(*t),
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

    fn ty(&mut self, t: &ast::TypeExpr) -> Type {
        use ast::TypeKind as T;
        let kind = match &t.kind {
            T::Named { module, name, args } => {
                let variable = if module.is_none() {
                    self.ty_frames
                        .iter()
                        .rev()
                        .find_map(|f| f.names.get(&name.name).copied())
                        .or_else(|| {
                            self.ty_def.as_mut().and_then(|mode| {
                                let var = mode.params.get(&name.name).copied()?;
                                mode.used.insert(var);
                                Some(var)
                            })
                        })
                } else {
                    None
                };
                if let Some(var) = variable {
                    let kind = if args.is_empty() {
                        TypeKind::Var(var)
                    } else {
                        self.error(
                            code::TYPE_VAR_ARGS,
                            t.span,
                            "a type variable takes no arguments",
                        );
                        TypeKind::Error
                    };
                    return Type { kind, span: t.span };
                }
                let before = self.diags.len();
                let found = self.lookup_type(module.as_ref(), name);
                if found.is_none() && module.is_none() && args.is_empty() {
                    let hint = if self.ty_def.is_some() {
                        "to use a type parameter, declare it in the head, as in `Box(A) { … }`"
                    } else {
                        "to use a type variable, declare it after the name of the binding, as in `f(A) = …`"
                    };
                    for d in &mut self.diags[before..] {
                        if d.code == code::UNKNOWN_TYPE {
                            d.message.push_str("; ");
                            d.message.push_str(hint);
                        }
                    }
                }
                let args: Vec<Type> = args.iter().map(|a| self.ty(a)).collect();
                match found {
                    Some(ty) if self.arity(ty) != args.len() => {
                        let want = self.arity(ty);
                        let tname = self.type_name(ty);
                        self.error(
                            code::TYPE_ARITY,
                            t.span,
                            format!(
                                "the type `{tname}` has {want} type parameter{}, but {} {} given",
                                if want == 1 { "" } else { "s" },
                                args.len(),
                                if args.len() == 1 { "is" } else { "are" }
                            ),
                        );
                        TypeKind::Error
                    }
                    Some(ty) => TypeKind::Named { ty, args },
                    None => TypeKind::Error,
                }
            }
            T::ValueOf { module, .. } => {
                self.error(
                    if module.is_some() { code::TYPE_OF_VALUE_RESERVED } else { code::LOWERCASE_TYPE_RESERVED },
                    t.span,
                    "a lowercase name in a type is kept for the type of a value; declare a type variable as an UpName, as in `f(A) = …`",
                );
                TypeKind::Error
            }
            T::Call { .. } => {
                self.error(
                    code::MACRO_RESERVED,
                    t.span,
                    "a call in a type is kept for macros",
                );
                TypeKind::Error
            }
            T::Hole => TypeKind::Hole,
            T::Tuple(ts) => TypeKind::Tuple(ts.iter().map(|x| self.ty(x)).collect()),
            T::Union(ts) => TypeKind::Union(ts.iter().map(|x| self.ty(x)).collect()),
            T::Inter(ts) => TypeKind::Inter(ts.iter().map(|x| self.ty(x)).collect()),
            T::Diff(ts) => TypeKind::Diff(ts.iter().map(|x| self.ty(x)).collect()),
            T::Fn {
                params,
                ret,
                err,
                env,
            } => TypeKind::Fn {
                params: params
                    .iter()
                    .map(|p| FnParamType {
                        label: p.label.clone(),
                        ty: self.ty(&p.ty),
                    })
                    .collect(),
                ret: Box::new(self.ty(ret)),
                err: err.as_ref().map(|e| Box::new(self.ty(e))),
                env: env.as_ref().map(|e| Box::new(self.ty(e))),
            },
        };
        Type { kind, span: t.span }
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

    /// §9.2: the type of an environment parameter or statement is one named
    /// type: not a type variable alone, a union, or `Never`. An alias is
    /// expanded with its arguments. Gives back the type constructor, or
    /// `None` when there is none or a problem has been reported.
    fn env_type(&mut self, t: &Type) -> Option<TypeRef> {
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
                    self.error(code::HOST_TYPE, f.ty.span,
                        "a host field has ABI parameter, result and error types, and no written environment; request fields return Request(A) or Request(Result(A,E)), with no written errors (§9.7, §10.3)");
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
            .map(|p| self.host_part(&p.ty))
            .collect::<Option<Vec<_>>>()?;
        let result = self.host_part(ret)?;
        let mut members = Vec::new();
        if let Some(err) = err {
            if !errors && !self.is_never(err) {
                return None;
            }
            let err = self.expand(err);
            let parts: Vec<Type> = match &err.kind {
                TypeKind::Union(ms) => ms.clone(),
                _ => vec![err.clone()],
            };
            for m in parts {
                if self.is_never(&m) {
                    continue;
                }
                let h = self.host_part(&m)?;
                if matches!(h, HostType::Opaque(_)) || members.contains(&h) {
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
        matches!(
            self.expand(t).kind,
            TypeKind::Named {
                ty: TypeRef::Builtin(BuiltinType::Never),
                ..
            }
        )
    }

    /// The part of B.2 for a type that the host can take (§10.3).
    fn host_part(&self, t: &Type) -> Option<HostType> {
        match &self.expand(t).kind {
            TypeKind::Named {
                ty: TypeRef::Builtin(b),
                ..
            } => match b {
                BuiltinType::Int => Some(HostType::Int),
                BuiltinType::Float => Some(HostType::Float),
                BuiltinType::String => Some(HostType::String),
                BuiltinType::BitArray => Some(HostType::BitArray),
                _ => None,
            },
            TypeKind::Named {
                ty: TypeRef::Custom(id),
                ..
            } => {
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
        for part in params.iter().map(|p| &p.ty).chain([&**ret]) {
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
            } else if !self.host_type(part) {
                self.error(
                    code::EXTERNAL_HOST_TYPE,
                    part.span,
                    "the parameters and the value of an external function are types that the host can take: `Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray`, or a custom type with no constructors (§10.3)",
                );
            }
        }
    }

    /// §10.3: a type that the host can take. Aliases are expanded.
    fn host_type(&self, t: &Type) -> bool {
        match &self.expand(t).kind {
            TypeKind::Error => true,
            TypeKind::Named {
                ty: TypeRef::Builtin(b),
                ..
            } => matches!(
                b,
                BuiltinType::Int | BuiltinType::Float | BuiltinType::String | BuiltinType::BitArray
            ),
            TypeKind::Named {
                ty: TypeRef::Custom(id),
                ..
            } => {
                let def = self.p.ty(*id);
                match &def.body {
                    // An alias in a circle; the circle has been reported.
                    TypeBody::Alias(_) => true,
                    TypeBody::Custom(ctors) => {
                        ctors.is_empty()
                            || (Some(def.module) == self.p.prelude
                                && matches!(def.name.as_str(), "Nil" | "Bool"))
                    }
                }
            }
            _ => false,
        }
    }

    // ---- Phase 4: values ----

    fn values(&mut self, m: ModuleId) {
        self.cur = m;
        let parsed = self.parsed;
        let ast = &parsed[m.0 as usize].ast;
        for (item, slot) in ast.items.iter().zip(self.slots[m.0 as usize].clone()) {
            match (&item.kind, slot) {
                (
                    ast::ItemKind::Declaration {
                        ty, type_params, ..
                    },
                    Slot::Decl(d),
                ) => {
                    let vars = self.push_declared_frame(type_params);
                    let t = self.ty(ty);
                    self.ty_frames.pop();
                    // The operations of the library are not host imports
                    // (§2.1), so their types are not limited.
                    if !self.in_library() {
                        let before = self.diags.len();
                        self.external_type(&t);
                        if self.diags.len() == before
                            && let Some(f) = self.host_fn(&t, false)
                        {
                            self.p.external_fns.insert(
                                DefId {
                                    module: m,
                                    index: d,
                                },
                                f,
                            );
                        }
                    }
                    if let DefKind::External {
                        ty: slot_ty,
                        ty_vars,
                        ..
                    } = &mut self.p.modules[m.0 as usize].defs[d as usize].kind
                    {
                        *slot_ty = t;
                        *ty_vars = vars;
                    }
                }
                (ast::ItemKind::Binding(b), Slot::Binding(index)) => {
                    let (binding, binders) = self.binding(b, true);
                    let mi = m.0 as usize;
                    for binder in binders {
                        if let Some(&d) = self.defs_by_name[mi].get(&binder.name)
                            && let DefKind::Binding { binding: bi, var } =
                                &mut self.p.modules[mi].defs[d as usize].kind
                            && *bi == index
                        {
                            *var = binder.var;
                        }
                    }
                    self.p.modules[mi].bindings[index as usize].binding = binding;
                }
                _ => {}
            }
        }
    }

    /// A quiet lookup, after imports are known (§3.10 and §3.11).
    fn unqualified_type(&self, name: &str) -> Option<TypeRef> {
        self.scopes[self.cur.0 as usize]
            .types
            .get(name)
            .map(|(t, _)| *t)
            .or_else(|| self.builtin_types.get(name).copied())
    }

    fn push_declared_frame(&mut self, params: &[ast::Ident]) -> Vec<TyVarId> {
        let mut names = HashMap::new();
        let mut vars = Vec::new();
        for param in params {
            if names.contains_key(&param.name) {
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
            if self.unqualified_type(&param.name).is_some() {
                self.error(
                    code::TYPE_VAR_IS_TYPE,
                    param.span,
                    "a type variable may not have the name of a type",
                );
            }
            let var = self.new_ty_var(&param.name);
            names.insert(param.name.clone(), var);
            vars.push(var);
        }
        self.ty_frames.push(TyFrame { names });
        vars
    }

    /// A binding, at the top level or in a block. Gives back the binders of
    /// its pattern; in a block, the caller makes them visible from the next
    /// statement (§6.10).
    fn binding(&mut self, b: &ast::Binding, top: bool) -> (Binding, Vec<Binder>) {
        let ty_vars = self.push_declared_frame(&b.type_params);
        let direct = self.is_direct(&b.value);
        let mut binders = Vec::new();
        let pattern = self.pattern(&b.pattern, &mut binders, None);
        let value = if direct && !top {
            // §6.10: the names may be used inside the function bodies of a
            // direct value.
            self.locals.push(Frame {
                names: binders.iter().map(|b| (b.name.clone(), b.var)).collect(),
                fn_only: Some(self.fn_depth),
            });
            let v = self.expr(&b.value);
            self.locals.pop();
            v
        } else {
            self.expr(&b.value)
        };
        self.ty_frames.pop();
        let binding = Binding {
            pattern,
            value,
            direct,
            ty_vars,
            span: b.span,
        };
        (binding, binders)
    }

    /// §6.10: a direct value is made without running code.
    fn is_direct(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as E;
        match &ungroup(e).kind {
            E::Fn(_) | E::Clauses(_) | E::Int(_) | E::Float(_) | E::String(_) | E::Name { .. } => {
                true
            }
            E::Unary { expr, .. } => {
                matches!(expr.kind, E::Int(_) | E::Float(_) | E::String(_))
            }
            E::Field { target, .. } => match &target.kind {
                // `m.x` is a name when `m` is a module.
                E::Name { module: None, name } => {
                    matches!(self.find(&name.name), Some(Found::Entry(Entry::Module(_))))
                }
                _ => false,
            },
            E::Tuple(items) => items.iter().all(|x| self.is_direct(x)),
            E::List { items, rest } => items
                .iter()
                .chain(rest.as_deref())
                .all(|x| self.is_direct(x)),
            // Constructors are the only values with an `UpName`.
            E::Call { callee, args } => {
                is_constructor(callee) && args.iter().all(|a| self.is_direct(&a.value))
            }
            // §6.9: `a |> C` is `C(a)`, and `a |> C(b)` is `C(a, b)`.
            E::Binary {
                op: ast::BinOp::Pipe,
                lhs,
                rhs,
            } => {
                self.is_direct(lhs)
                    && match &rhs.kind {
                        E::Name { .. } => is_constructor(rhs),
                        E::Call { callee, args } => {
                            is_constructor(callee) && args.iter().all(|a| self.is_direct(&a.value))
                        }
                        _ => false,
                    }
            }
            _ => false,
        }
    }

    /// Looks up a value name with no module before it (§5).
    fn find(&self, name: &str) -> Option<Found> {
        if !is_upper(name) {
            for f in self.locals.iter().rev() {
                if f.fn_only.is_some_and(|d| self.fn_depth <= d) {
                    continue;
                }
                if let Some(v) = f.names.get(name) {
                    return Some(Found::Local(*v));
                }
            }
        }
        let scope = &self.scopes[self.cur.0 as usize];
        if let Some((e, _)) = scope.values.get(name) {
            return Some(Found::Entry(*e));
        }
        self.builtin_values.get(name).map(|e| Found::Entry(*e))
    }

    /// A member `m.x` or `m.C` of another module.
    fn member(&mut self, target: ModuleId, name: &ast::Ident) -> ExprKind {
        let ti = target.0 as usize;
        let path = self.module_name(target).to_string();
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
            Some(v) if exported => {
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

    fn name(&mut self, module: Option<&ast::Ident>, name: &ast::Ident) -> ExprKind {
        if let Some(m) = module {
            return match self.find(&m.name) {
                Some(Found::Entry(Entry::Module(t))) => self.member(t, name),
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
        match self.find(&name.name) {
            Some(Found::Local(v)) => ExprKind::Name(ValueRef::Local(v)),
            Some(Found::Entry(Entry::Def(d))) => {
                self.used_def(d, name.span);
                ExprKind::Name(ValueRef::Def(d))
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
    fn ctor(&mut self, module: Option<&ast::Ident>, name: &ast::Ident) -> Option<CtorRef> {
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

    fn expr(&mut self, e: &ast::Expr) -> Expr {
        use ast::ExprKind as E;
        let kind = match &e.kind {
            E::Int(n) => ExprKind::Int(n.clone()),
            E::Float(f) => ExprKind::Float(*f),
            E::String(s) => ExprKind::String(s.clone()),
            E::Name { module, name } => self.name(module.as_ref(), name),
            E::Tuple(items) => ExprKind::Tuple(items.iter().map(|x| self.expr(x)).collect()),
            E::List { items, rest } => ExprKind::List {
                items: items.iter().map(|x| self.expr(x)).collect(),
                rest: rest.as_ref().map(|r| Box::new(self.expr(r))),
            },
            E::BitArray(segs) => ExprKind::BitArray(
                segs.iter()
                    .map(|s| Segment {
                        value: self.expr(&s.value),
                        format: self.format(s.format.as_ref()),
                        span: s.span,
                    })
                    .collect(),
            ),
            E::Block(b) => ExprKind::Block(self.block(b)),
            E::Fn(f) => ExprKind::Fn(Box::new(self.function(f, e.span))),
            E::Clauses(clauses) => self.clause_function(clauses, e.span),
            E::Case { subject, clauses } if clauses.len() == 1 && clauses[0].guard.is_none() => {
                let clause = &clauses[0];
                if clause.patterns.len() != 1 {
                    self.one_clause_choices(e.span)
                } else {
                    // §3.9: the subject precedes the new scope; no type frame,
                    // no generalization, and the binding stops at the pattern.
                    let value = self.expr(subject);
                    let mut binders = Vec::new();
                    let pattern = self.pattern(&clause.patterns[0], &mut binders, None);
                    self.locals.push(Frame {
                        names: binders.into_iter().map(|b| (b.name, b.var)).collect(),
                        fn_only: None,
                    });
                    let body = self.expr(&clause.body);
                    self.locals.pop();
                    let span = pattern.span;
                    ExprKind::Block(Block {
                        stmts: vec![
                            Stmt::Bind(Binding {
                                pattern,
                                value,
                                direct: false,
                                ty_vars: vec![],
                                span,
                            }),
                            Stmt::Expr(body),
                        ],
                        span: e.span,
                    })
                }
            }
            E::Case {
                subject, clauses, ..
            } => ExprKind::Case {
                subject: Box::new(self.expr(subject)),
                clauses: clauses.iter().map(|c| self.clause(c)).collect(),
            },
            E::Call { callee, args } => ExprKind::Call {
                callee: Box::new(self.expr(callee)),
                args: args
                    .iter()
                    .map(|a| Arg {
                        label: a.label.clone(),
                        value: self.expr(&a.value),
                    })
                    .collect(),
            },
            E::Field { target, name } => {
                let module = match &target.kind {
                    E::Name { module: None, name } => match self.find(&name.name) {
                        Some(Found::Entry(Entry::Module(t))) => Some(Some(t)),
                        Some(Found::Entry(Entry::Broken)) => Some(None),
                        _ => None,
                    },
                    _ => None,
                };
                match module {
                    Some(Some(t)) => self.member(t, name),
                    Some(None) => ExprKind::Error,
                    None => ExprKind::Field {
                        target: Box::new(self.expr(target)),
                        name: name.clone(),
                    },
                }
            }
            E::TupleIndex {
                target,
                index,
                index_span,
            } => ExprKind::TupleIndex {
                target: Box::new(self.expr(target)),
                index: index.clone(),
                index_span: *index_span,
            },
            E::RecordUpdate {
                constructor,
                base,
                fields,
            } => {
                let ctor = match &constructor.kind {
                    E::Name { module, name } => self.ctor(module.as_ref(), name),
                    _ => {
                        self.error(
                            code::NOT_A_CONSTRUCTOR,
                            constructor.span,
                            "a record update starts with a constructor",
                        );
                        None
                    }
                };
                let base = Box::new(self.expr(base));
                let fields = fields
                    .iter()
                    .map(|(n, x)| (n.clone(), self.expr(x)))
                    .collect();
                match ctor {
                    Some(ctor) => ExprKind::RecordUpdate {
                        ctor,
                        ctor_span: constructor.span,
                        base,
                        fields,
                    },
                    None => ExprKind::Error,
                }
            }
            E::Unary { op, expr } => ExprKind::Unary {
                op: *op,
                expr: Box::new(self.expr(expr)),
            },
            E::Binary { op, lhs, rhs } => ExprKind::Binary {
                op: *op,
                lhs: Box::new(self.expr(lhs)),
                rhs: Box::new(self.expr(rhs)),
            },
        };
        Expr { kind, span: e.span }
    }

    fn one_clause_choices(&mut self, span: Span) -> ExprKind {
        self.error(code::CHOICES_ONE_CLAUSE, span, "a clause block with one clause and choices is not supported yet; write one clause for each choice");
        ExprKind::Error
    }

    fn clause_function(&mut self, clauses: &[ast::Clause], span: Span) -> ExprKind {
        let single = clauses.len() == 1 && clauses[0].guard.is_none();
        if single && clauses[0].patterns.len() != 1 {
            return self.one_clause_choices(span);
        }
        let synthetic = Span::new(span.start, span.start);
        let (param, body) = if single {
            (clauses[0].patterns[0].clone(), clauses[0].body.clone())
        } else {
            let case_span = match (clauses.first(), clauses.last()) {
                (Some(first), Some(last)) => first.span.to(last.span),
                _ => Span::new(span.start + 1, span.start + 1),
            };
            (
                ast::Pattern {
                    kind: ast::PatternKind::Var("%arg".into()),
                    span: synthetic,
                },
                ast::Expr {
                    kind: ast::ExprKind::Case {
                        subject: Box::new(ast::Expr {
                            kind: ast::ExprKind::Name {
                                module: None,
                                name: ast::Ident {
                                    name: "%arg".into(),
                                    span: synthetic,
                                },
                            },
                            span: synthetic,
                        }),
                        clauses: clauses.to_vec(),
                    },
                    span: case_span,
                },
            )
        };
        let mut function = self.function(
            &ast::Function {
                params: vec![ast::Param::Pattern(param)],
                ret: None,
                body: ast::Block {
                    stmts: vec![ast::Stmt::Expr(body)],
                    span,
                },
            },
            span,
        );
        if !single && let Param::Pattern { label, .. } = &mut function.params[0] {
            *label = None;
        }
        ExprKind::Fn(Box::new(function))
    }

    fn format(&mut self, f: Option<&ast::Format>) -> Option<Format> {
        f.map(|f| Format {
            name: f.name.clone(),
            arg: f.arg.as_ref().map(|a| Box::new(self.expr(a))),
        })
    }

    fn block(&mut self, b: &ast::Block) -> Block {
        let mark = self.locals.len();
        let mut stmts = Vec::new();
        for s in &b.stmts {
            stmts.push(match s {
                ast::Stmt::Bind(binding) => {
                    let (binding, binders) = self.binding(binding, false);
                    self.locals.push(Frame {
                        names: binders.into_iter().map(|b| (b.name, b.var)).collect(),
                        fn_only: None,
                    });
                    Stmt::Bind(binding)
                }
                ast::Stmt::Env { ty, value, span } => {
                    let ty = self.ty(ty);
                    self.env_type(&ty);
                    let value = self.expr(value);
                    Stmt::Env {
                        ty,
                        value,
                        span: *span,
                    }
                }
                ast::Stmt::Expr(e) => Stmt::Expr(self.expr(e)),
            });
        }
        self.locals.truncate(mark);
        Block {
            stmts,
            span: b.span,
        }
    }

    /// §6.8 and §6.11.
    fn function(&mut self, f: &ast::Function, span: Span) -> Function {
        let mut params = Vec::new();
        let mut env_heads: Vec<TypeRef> = Vec::new();
        let mut all: Vec<Binder> = Vec::new();
        let mut labels: Vec<String> = Vec::new();
        for p in &f.params {
            match p {
                ast::Param::Pattern(pat) => {
                    let mut binders = Vec::new();
                    let pattern = self.pattern(pat, &mut binders, None);
                    for b in binders {
                        self.param_name(&all, &b.name, b.span);
                        all.push(b);
                    }
                    let label = label_of(pat);
                    if let Some(l) = &label {
                        if labels.contains(l) {
                            self.error(
                                code::PARAMS_SAME_LABEL,
                                pat.span,
                                format!("two parameters of this function have the label `{l}`"),
                            );
                        }
                        labels.push(l.clone());
                    }
                    params.push(Param::Pattern { pattern, label });
                }
                ast::Param::Env { name, ty } => {
                    let ty = self.ty(ty);
                    // §9.2: two environment parameters of one function may
                    // not have the same type constructor.
                    if let Some(head) = self.env_type(&ty) {
                        if env_heads.contains(&head) {
                            let tname = self.type_name(head);
                            self.error(
                                code::ENV_PARAMS_SAME,
                                ty.span,
                                format!("two environment parameters of this function have the type constructor `{tname}` (§9.2)"),
                            );
                        }
                        env_heads.push(head);
                    }
                    let var = self.new_var();
                    self.param_name(&all, &name.name, name.span);
                    all.push(Binder {
                        var,
                        name: name.name.clone(),
                        span: name.span,
                    });
                    params.push(Param::Env {
                        var,
                        name: name.clone(),
                        ty,
                    });
                }
            }
        }
        let ret = f.ret.as_ref().map(|r| RetType {
            value: self.ty(&r.value),
            err: r.err.as_ref().map(|t| self.ty(t)),
            env: r.env.as_ref().map(|t| self.ty(t)),
        });
        self.fn_depth += 1;
        self.locals.push(Frame {
            names: all.into_iter().map(|b| (b.name, b.var)).collect(),
            fn_only: None,
        });
        let body = self.block(&f.body);
        self.locals.pop();
        self.fn_depth -= 1;
        Function {
            params,
            ret,
            body,
            span,
        }
    }

    fn param_name(&mut self, all: &[Binder], name: &str, span: Span) {
        if all.iter().any(|b| b.name == name) {
            self.error(
                code::PARAMS_BIND_TWICE,
                span,
                format!("the parameters of this function bind `{name}` more than once"),
            );
        }
    }

    fn clause(&mut self, c: &ast::Clause) -> Clause {
        let mut first: Vec<Binder> = Vec::new();
        let mut patterns = Vec::new();
        for (i, p) in c.patterns.iter().enumerate() {
            if i == 0 {
                patterns.push(self.pattern(p, &mut first, None));
                continue;
            }
            let shared: HashMap<String, VarId> =
                first.iter().map(|b| (b.name.clone(), b.var)).collect();
            let mut binders = Vec::new();
            patterns.push(self.pattern(p, &mut binders, Some(&shared)));
            let a: HashSet<&str> = first.iter().map(|b| b.name.as_str()).collect();
            let b: HashSet<&str> = binders.iter().map(|b| b.name.as_str()).collect();
            if a != b {
                let mut diff: Vec<&str> = a.symmetric_difference(&b).copied().collect();
                diff.sort();
                self.error(
                    code::CHOICES_DIFFER,
                    p.span,
                    format!(
                        "every choice of a clause binds the same names, but not all bind {}",
                        diff.iter()
                            .map(|n| format!("`{n}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                );
            }
        }
        self.locals.push(Frame {
            names: first.into_iter().map(|b| (b.name, b.var)).collect(),
            fn_only: None,
        });
        let guard = c.guard.as_ref().map(|g| self.expr(g));
        let body = self.expr(&c.body);
        self.locals.pop();
        Clause {
            patterns,
            guard,
            body,
            span: c.span,
        }
    }

    fn bind(
        &mut self,
        name: &str,
        span: Span,
        binders: &mut Vec<Binder>,
        shared: Option<&HashMap<String, VarId>>,
    ) -> Binder {
        if binders.iter().any(|b| b.name == name) {
            self.error(
                code::PATTERN_BINDS_TWICE,
                span,
                format!("this pattern binds `{name}` more than once"),
            );
        }
        let var = match shared.and_then(|s| s.get(name)) {
            Some(v) => *v,
            None => self.new_var(),
        };
        let b = Binder {
            var,
            name: name.to_string(),
            span,
        };
        binders.push(b.clone());
        b
    }

    fn pattern(
        &mut self,
        p: &ast::Pattern,
        binders: &mut Vec<Binder>,
        shared: Option<&HashMap<String, VarId>>,
    ) -> Pattern {
        use ast::PatternKind as P;
        let kind = match &p.kind {
            P::Var(n) => PatternKind::Var(self.bind(n, p.span, binders, shared)),
            P::Discard(_) => PatternKind::Discard,
            P::Typed { binder, ty } => {
                let binder = (!binder.discard)
                    .then(|| self.bind(&binder.name, binder.span, binders, shared));
                PatternKind::Typed {
                    binder,
                    ty: self.ty(ty),
                }
            }
            P::Named { name, pattern } => {
                let binder = self.bind(&name.name, name.span, binders, shared);
                PatternKind::Named {
                    binder,
                    pattern: Box::new(self.pattern(pattern, binders, shared)),
                }
            }
            P::Int { negative, value } => PatternKind::Int {
                negative: *negative,
                value: value.clone(),
            },
            P::Float { negative, value } => PatternKind::Float {
                negative: *negative,
                value: *value,
            },
            P::String(s) => PatternKind::String(s.clone()),
            P::Prefix { prefix, rest } => PatternKind::Prefix {
                prefix: prefix.clone(),
                rest: (!rest.discard).then(|| self.bind(&rest.name, rest.span, binders, shared)),
            },
            P::Constructor {
                module,
                name,
                args,
                spread,
            } => {
                let ctor = self.ctor(module.as_ref(), name);
                let args: Vec<Pattern> = args
                    .iter()
                    .map(|a| self.pattern(a, binders, shared))
                    .collect();
                match ctor {
                    Some(ctor) => PatternKind::Constructor {
                        ctor,
                        args,
                        spread: *spread,
                    },
                    None => PatternKind::Error(args),
                }
            }
            P::Tuple(items) => PatternKind::Tuple(
                items
                    .iter()
                    .map(|a| self.pattern(a, binders, shared))
                    .collect(),
            ),
            P::List { items, rest } => {
                let items = items
                    .iter()
                    .map(|a| self.pattern(a, binders, shared))
                    .collect();
                let rest = rest.as_ref().map(|r| {
                    r.as_ref()
                        .filter(|b| !b.discard)
                        .map(|b| self.bind(&b.name, b.span, binders, shared))
                });
                PatternKind::List { items, rest }
            }
            P::BitArray(segs) => {
                let mut out = Vec::new();
                for s in segs {
                    let value = self.pattern(&s.value, binders, shared);
                    // §8.4: a length may use names of earlier segments.
                    self.locals.push(Frame {
                        names: binders.iter().map(|b| (b.name.clone(), b.var)).collect(),
                        fn_only: None,
                    });
                    let format = self.format(s.format.as_ref());
                    self.locals.pop();
                    out.push(Segment {
                        value,
                        format,
                        span: s.span,
                    });
                }
                PatternKind::BitArray(out)
            }
        };
        Pattern { kind, span: p.span }
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
            let mut found = Vec::new();
            self.hidden_types(m, &t, None, &mut HashSet::new(), &mut found);
            for (span, name) in found {
                self.error(
                    code::EXPORT_HIDDEN_TYPE,
                    span,
                    format!(
                        "{what} is exported, but its type uses the type `{name}` of this module, which is not exported (§2.4)"
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
        out: &mut Vec<(Span, String)>,
    ) {
        if let TypeKind::Named {
            ty: TypeRef::Custom(id),
            ..
        } = &t.kind
        {
            let def = self.p.ty(*id);
            if def.module == m && !def.exported {
                match &def.body {
                    TypeBody::Alias(body) => {
                        if !self.circular.contains(id) && seen.insert(*id) {
                            self.hidden_types(m, body, Some(at.unwrap_or(t.span)), seen, out);
                            seen.remove(id);
                        }
                    }
                    TypeBody::Custom(_) => out.push((at.unwrap_or(t.span), def.name.clone())),
                }
            }
        }
        for_each_child(t, &mut |c| self.hidden_types(m, c, at, seen, out));
    }

    // ---- Phase 6: rules about the whole program ----

    fn program_rules(&mut self) {
        // §10.2: one `@main` in the compiled package.
        for &(m, span) in self.mains.clone().iter().skip(1) {
            self.cur = m;
            self.error(
                code::MAIN_TWICE,
                span,
                "the compiled package has more than one `@main`",
            );
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
                    // Only the `@main` and `@test` of the compiled package
                    // are entry points; the others are not used (§10.2).
                    if let DefKind::Binding { binding, .. } = def.kind
                        && self.p.module(d.module).package == root
                    {
                        let b = &self.p.module(d.module).bindings[binding as usize];
                        if b.main || b.test {
                            problem = Some((
                                code::WASM_EXPORT_ENTRY,
                                format!(
                                    "`{}.{}` is a host export and also the program entry or a test, so their export names would be the same",
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
        TypeKind::Var(_) | TypeKind::Hole | TypeKind::Error => {}
    }
}

/// `C` or `m.C`: constructors are the only values with an `UpName`.
fn is_constructor(e: &ast::Expr) -> bool {
    matches!(&e.kind, ast::ExprKind::Name { name, .. } if is_upper(&name.name))
}

/// Calls `f` on each type directly inside `t`.
fn for_each_child(t: &Type, f: &mut dyn FnMut(&Type)) {
    match &t.kind {
        TypeKind::Named { args, .. } => args.iter().for_each(f),
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
        TypeKind::Var(_) | TypeKind::Hole | TypeKind::Error => {}
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
        TypeKind::Var(v) => match map.get(v) {
            Some(r) => return (*r).clone(),
            None => TypeKind::Var(*v),
        },
        TypeKind::Named { ty, args } => TypeKind::Named {
            ty: *ty,
            args: args.iter().map(|a| subst(a, map)).collect(),
        },
        TypeKind::Tuple(ts) => TypeKind::Tuple(ts.iter().map(|a| subst(a, map)).collect()),
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
