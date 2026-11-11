//! Queries for editors on a checked program: the type of the name at a
//! position, where that name is defined, the names that may be written at a
//! position, and the top-level definitions of a module.
//!
//! A position is a module and a byte offset in its file. The queries read
//! the HIR, which resolves every name, and `Typed`, which has the shown type
//! schemes of the top-level names and, from [`check_for_queries`], the types
//! of the local names. They also read the source text and its syntax tree,
//! for what the HIR does not keep: imports by name, documentation comments,
//! and the spans of whole items.

use crate::hir::*;
use crate::types::{Access, Typed};
use crate::{Checked, order, parse, resolve, types};
use ilang_package::{Diagnostic, PackageGraph, SourceId};
use ilang_syntax::ast;
use ilang_syntax::span::Span;
use std::collections::{HashMap, HashSet};

/// [`crate::check`] for editors. The diagnostics are the same. But the
/// program is resolved even when a module has a syntax error, from the
/// syntax tree that the parser recovered, and its types are inferred even
/// when it has errors, so that names and types can still be found; the
/// problems found after the first errors are not reported. The types of
/// the local names are kept (`Typed::locals`).
pub fn check_for_queries(graph: &PackageGraph) -> Checked {
    let (parsed, diags) = parse(graph);
    check_parsed_for_queries(graph, &parsed, diags)
}

/// Resolve and infer query information from the syntax-expanded modules.
pub fn check_parsed_for_queries(
    graph: &PackageGraph,
    parsed: &[crate::Parsed],
    mut diags: Vec<Diagnostic>,
) -> Checked {
    let syntax_errors = diags.iter().any(Diagnostic::is_error);
    let (mut program, more) = resolve::resolve(graph, parsed);
    let mut ordered = Vec::new();
    order::order(&mut program, &mut ordered);
    if !syntax_errors {
        diags.extend(more);
        diags.extend(ordered);
    }
    if diags.iter().any(Diagnostic::is_error) {
        // Inference is only a best effort on a program with errors.
        let types = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            types::check_keeping_locals_with_errors(&program, &graph.sources, true).0
        }))
        .ok();
        return Checked {
            program: Some(program),
            types,
            diags,
        };
    }
    let (typed, more) = types::check_keeping_locals(&program, &graph.sources);
    diags.extend(more);
    Checked {
        program: Some(program),
        types: Some(typed),
        diags,
    }
}

/// What [`Query::type_at`] shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hover {
    /// The span of the name or literal, in the file of the module asked.
    pub span: Span,
    /// The name and its type in the syntax of §3, as `name: Type`, or the
    /// definition of a type.
    pub code: String,
    /// The documentation comment (`///`) of the definition, if any.
    pub doc: Option<String>,
}

/// A place in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub module: ModuleId,
    pub source: SourceId,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    /// A name bound by a pattern or a parameter.
    Local,
    /// A top-level name whose value is a function.
    Function,
    /// Another top-level name.
    Value,
    Constructor,
    Type,
    Module,
    Field,
    Keyword,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub label: String,
    pub kind: CompletionKind,
    /// The type, when it is known.
    pub detail: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Function,
    Value,
    Type,
    Constructor,
}

/// A top-level definition, or a constructor of one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    /// The whole item.
    pub span: Span,
    /// The name.
    pub name_span: Span,
    pub detail: Option<String>,
    pub children: Vec<Symbol>,
}

/// What a name or a literal in the source is.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    Local(VarId),
    Def(DefId),
    Ctor(CtorRef),
    Type(TypeRef),
    TyVar(TyVarId),
    /// A field with a label, of each constructor that has it.
    Field(Vec<(CtorRef, u32)>),
    Module(ModuleId),
    Literal(BuiltinType),
}

/// The queries on one checked program.
pub struct Query<'a> {
    graph: &'a PackageGraph,
    program: &'a Program,
    types: Option<&'a Typed>,
}

impl<'a> Query<'a> {
    pub fn new(graph: &'a PackageGraph, program: &'a Program, types: Option<&'a Typed>) -> Self {
        Query {
            graph,
            program,
            types,
        }
    }

    /// The module whose file is `source`.
    pub fn module_of_source(&self, source: SourceId) -> Option<ModuleId> {
        self.program
            .modules
            .iter()
            .position(|m| m.source == source)
            .map(|i| ModuleId(i as u32))
    }

    /// The file of a module.
    pub fn source_of(&self, m: ModuleId) -> SourceId {
        self.program.module(m).source
    }

    fn text(&self, m: ModuleId) -> &'a str {
        &self.graph.sources.get(self.program.module(m).source).text
    }

    fn ast(&self, m: ModuleId) -> ast::Module {
        let source = self.graph.sources.get(self.program.module(m).source);
        if source.display.ends_with(".is") {
            ilang_syntax::lisp::parse_source(&source.text).0
        } else {
            ilang_syntax::parse_module(self.text(m)).0
        }
    }

    // ---- type_at ----

    /// The type of the name or the literal at `offset`: for a top-level
    /// name its type scheme, for a local name its inferred type, for a
    /// constructor its type as a function, for a field its written type,
    /// and for a type its definition.
    pub fn type_at(&self, m: ModuleId, offset: u32) -> Option<Hover> {
        let (span, target) = self.target_at(m, offset)?;
        let (code, doc_at) = self.describe(m, span.start, &target)?;
        Some(Hover {
            span,
            code,
            doc: doc_at.and_then(|(module, span)| self.doc(module, span)),
        })
    }

    /// The text for a target at `at`, and the place of the definition
    /// whose documentation goes with it.
    fn describe(
        &self,
        m: ModuleId,
        at: u32,
        target: &Target,
    ) -> Option<(String, Option<(ModuleId, Span)>)> {
        let p = self.program;
        Some(match target {
            Target::Local(v) => {
                if let Some(d) = self.def_of_var(m, *v) {
                    return self.describe(m, at, &Target::Def(d));
                }
                let name = self.local_name(m, *v)?;
                let ty = self.local_type(m, *v, at);
                (with_type(&name, ty.as_deref()), None)
            }
            Target::Def(d) => {
                let def = p.def(*d);
                (
                    with_type(&def.name, self.def_type(*d).as_deref()),
                    Some((d.module, def.span)),
                )
            }
            Target::Ctor(c) => {
                let ty = p.ty(c.ty);
                let ctor = self.ctor_def(*c)?;
                (
                    format!("{}: {}", ctor.name, self.ctor_type(*c)),
                    Some((ty.module, ty.span)),
                )
            }
            Target::Type(TypeRef::Custom(t)) => {
                let ty = p.ty(*t);
                (self.type_definition(*t), Some((ty.module, ty.span)))
            }
            Target::Type(TypeRef::Builtin(b)) => (builtin_head(*b), None),
            Target::TyVar(v) => (self.ty_var_name(*v), None),
            Target::Field(fields) => {
                let (c, i) = *fields.first()?;
                let f = self.ctor_def(c)?.fields.get(i as usize)?;
                let label = f.label.as_ref()?;
                let ty = p.ty(c.ty);
                (
                    format!("{}: {}", label.name, self.show_type(&f.ty)),
                    Some((ty.module, ty.span)),
                )
            }
            Target::Module(t) => (format!("module {}", p.module(*t).path), None),
            Target::Literal(b) => (b.name().to_string(), None),
        })
    }

    fn def_type(&self, d: DefId) -> Option<String> {
        if let Some(s) = self.types.and_then(|t| t.scheme_hovers.get(&d)) {
            return Some(s.clone());
        }
        match &self.program.def(d).kind {
            DefKind::External { ty, .. } => Some(self.show_type(ty)),
            DefKind::Binding { .. } => None,
        }
    }

    /// The type of a local name used at `at`, in the context of the
    /// top-level binding there, whose type scheme names the type variables
    /// that the name shares with the other names of the binding.
    fn local_type(&self, m: ModuleId, v: VarId, at: u32) -> Option<String> {
        let mut context = Vec::new();
        if let Some(b) = self
            .program
            .module(m)
            .bindings
            .iter()
            .find(|b| contains(b.binding.span, at))
        {
            pattern_binders(&b.binding.pattern, &mut context);
        }
        let context: Vec<VarId> = context.iter().map(|b| b.var).collect();
        self.types?
            .locals
            .as_ref()?
            .show_in(self.program, m, v, &context)
    }

    /// The top-level name that a variable of a top-level pattern binds.
    fn def_of_var(&self, m: ModuleId, v: VarId) -> Option<DefId> {
        // The variable may be bound in any module, but it is nearly always
        // this one.
        let modules =
            std::iter::once(m).chain((0..self.program.modules.len() as u32).map(ModuleId));
        for mi in modules {
            for (i, d) in self.program.module(mi).defs.iter().enumerate() {
                if matches!(d.kind, DefKind::Binding { var, .. } if var == v) {
                    return Some(DefId {
                        module: mi,
                        index: i as u32,
                    });
                }
            }
        }
        None
    }

    fn local_name(&self, m: ModuleId, v: VarId) -> Option<String> {
        let w = self.walk(m);
        let span = *w.binders.get(&v)?;
        Some(
            self.text(m)
                .get(span.start as usize..span.end as usize)?
                .to_string(),
        )
    }

    fn ctor_def(&self, c: CtorRef) -> Option<&'a CtorDef> {
        match &self.program.ty(c.ty).body {
            TypeBody::Custom(cs) => cs.get(c.index as usize),
            TypeBody::Alias(_) => None,
        }
    }

    /// The type of a constructor as a value (§6.3).
    fn ctor_type(&self, c: CtorRef) -> String {
        let result = self.type_head(c.ty);
        let Some(def) = self.ctor_def(c) else {
            return result;
        };
        if def.fields.is_empty() {
            return result;
        }
        let fields: Vec<String> = def
            .fields
            .iter()
            .map(|f| match &f.label {
                Some(l) => format!("{}: {}", l.name, self.show_type(&f.ty)),
                None => self.show_type(&f.ty),
            })
            .collect();
        format!("({}) -> {result}", fields.join(", "))
    }

    /// `T(a, b)`.
    fn type_head(&self, t: TypeId) -> String {
        let def = self.program.ty(t);
        if def.params.is_empty() {
            def.name.clone()
        } else {
            let ps: Vec<&str> = def.params.iter().map(|(n, _)| n.as_str()).collect();
            format!("{}({})", def.name, ps.join(", "))
        }
    }

    /// `T(a) { C1(a) C2 }` or `T(a) = Type` (§4.3).
    fn type_definition(&self, t: TypeId) -> String {
        let def = self.program.ty(t);
        let head = self.type_head(t);
        if let Some(shown) = self.types.and_then(|types| types.alias_types.get(&t)) {
            return format!(
                "{head} = {}",
                shown.text(&types::type_names(self.program, def.module))
            );
        }
        if let Some(pattern) = self
            .program
            .alias_patterns
            .iter()
            .find(|p| p.binders.contains(&t))
        {
            let source = &self
                .graph
                .sources
                .get(self.program.module(pattern.module).source)
                .text;
            return source[pattern.span.start as usize..pattern.span.end as usize].to_string();
        }
        match &def.body {
            TypeBody::Alias(ty) => format!("{head} = {}", self.show_type(ty)),
            TypeBody::Custom(cs) if cs.is_empty() => format!("{head} {{}}"),
            TypeBody::Custom(cs) => {
                let cs: Vec<String> = cs
                    .iter()
                    .map(|c| {
                        if c.fields.is_empty() {
                            return c.name.clone();
                        }
                        let fs: Vec<String> = c
                            .fields
                            .iter()
                            .map(|f| match &f.label {
                                Some(l) => format!("{}: {}", l.name, self.show_type(&f.ty)),
                                None => self.show_type(&f.ty),
                            })
                            .collect();
                        format!("{}({})", c.name, fs.join(", "))
                    })
                    .collect();
                if cs.iter().map(String::len).sum::<usize>() < 60 {
                    format!("{head} {{ {} }}", cs.join("  "))
                } else {
                    format!("{head} {{\n  {}\n}}", cs.join("\n  "))
                }
            }
        }
    }

    fn ty_var_name(&self, v: TyVarId) -> String {
        self.program
            .ty_var_names
            .get(v.0 as usize)
            .cloned()
            .unwrap_or_else(|| "_".into())
    }

    fn type_ref_name(&self, r: TypeRef) -> String {
        match r {
            TypeRef::Custom(t) => self.program.ty(t).name.clone(),
            TypeRef::Builtin(b) => b.name().to_string(),
        }
    }

    /// A written type in the syntax of §3.
    pub fn show_type(&self, t: &Type) -> String {
        self.show_type_prec(t, 0)
    }

    /// `prec`: 0 any type, 1 no union, 2 no intersection, 3 no difference
    /// and no function type.
    fn show_type_prec(&self, t: &Type, prec: u8) -> String {
        let wrap = |s: String, need: u8| if prec > need { format!("{{ {s} }}") } else { s };
        match &t.kind {
            TypeKind::Named { ty, args } => {
                let base = self.type_ref_name(*ty);
                if args.is_empty() {
                    base
                } else {
                    let args: Vec<String> = args.iter().map(|a| self.show_type(a)).collect();
                    format!("{base}({})", args.join(", "))
                }
            }
            TypeKind::Lit(v) => v.text(),
            TypeKind::Var(v) => self.ty_var_name(*v),
            TypeKind::Of { site, .. } => {
                let s = &self.program.of_sites[site.0 as usize];
                self.graph
                    .sources
                    .get(self.program.module(s.module).source)
                    .text[s.span.start as usize..s.span.end as usize]
                    .to_string()
            }
            TypeKind::Hole | TypeKind::Error => "_".into(),
            TypeKind::Tuple(parts) => {
                let parts: Vec<String> = parts.iter().map(|a| self.show_type(a)).collect();
                format!("({})", parts.join(", "))
            }
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                let params: Vec<String> = params
                    .iter()
                    .map(|p| match &p.label {
                        Some(l) => format!("{}: {}", l.name, self.show_type(&p.ty)),
                        None => self.show_type(&p.ty),
                    })
                    .collect();
                let part = |x: &Type, followed: bool| {
                    let s = self.show_type(x);
                    if followed && matches!(x.kind, TypeKind::Fn { .. }) {
                        format!("{{ {s} }}")
                    } else {
                        s
                    }
                };
                let mut s = format!(
                    "({}) -> {}",
                    params.join(", "),
                    part(ret, err.is_some() || env.is_some())
                );
                if let Some(e) = err {
                    s.push_str(&format!(" ? {}", part(e, env.is_some())));
                }
                if let Some(n) = env {
                    s.push_str(&format!(" ! {}", part(n, false)));
                }
                wrap(s, 0)
            }
            TypeKind::Union(ms) => {
                let ms = self.shown_type_parts(ms, 1);
                wrap(ms.join(" | "), 0)
            }
            TypeKind::Inter(ms) => {
                let ms: Vec<String> = ms.iter().map(|m| self.show_type_prec(m, 2)).collect();
                wrap(ms.join(" & "), 1)
            }
            TypeKind::Diff(ms) => {
                let ms = self.shown_type_parts(ms, 3);
                wrap(ms.join(" - "), 2)
            }
        }
    }

    /// Written alias hovers follow the same per-scalar limit as inferred hovers.
    fn shown_type_parts(&self, parts: &[Type], prec: u8) -> Vec<String> {
        let kind = |t: &Type| match &t.kind {
            TypeKind::Lit(crate::types::LitValue::Int(_)) => Some(0usize),
            TypeKind::Lit(crate::types::LitValue::Str(_)) => Some(1usize),
            _ => None,
        };
        let mut counts = [0usize; 2];
        for t in parts {
            if let Some(k) = kind(t) {
                counts[k] += 1;
            }
        }
        let mut seen = [0usize; 2];
        let mut result = Vec::new();
        for t in parts {
            if let Some(k) = kind(t) {
                seen[k] += 1;
                if counts[k] > 12 && seen[k] > 10 {
                    if seen[k] == 11 {
                        result.push(format!("… ({} more)", counts[k] - 10));
                    }
                    continue;
                }
            }
            result.push(self.show_type_prec(t, prec));
        }
        result
    }

    /// The `///` comment of the item whose span holds `span`.
    fn doc(&self, m: ModuleId, span: Span) -> Option<String> {
        let ast = self.ast(m);
        let item = ast
            .items
            .iter()
            .find(|i| i.span.start <= span.start && span.end <= i.span.end)?;
        if item.doc.is_empty() {
            return None;
        }
        let lines: Vec<&str> = item
            .doc
            .iter()
            .map(|l| l.strip_prefix(' ').unwrap_or(l))
            .collect();
        Some(lines.join("\n"))
    }

    // ---- definition_at ----

    /// Where the name at `offset` is defined: a local name at its binder,
    /// a top-level name, a constructor, a field, or a type at its name, and
    /// a module at the start of its file.
    pub fn definition_at(&self, m: ModuleId, offset: u32) -> Option<Location> {
        let (_, target) = self.target_at(m, offset)?;
        self.definition(m, &target)
    }

    fn definition(&self, m: ModuleId, target: &Target) -> Option<Location> {
        let p = self.program;
        let at = |module: ModuleId, span: Span| Location {
            module,
            source: p.module(module).source,
            span,
        };
        match target {
            Target::Local(v) => {
                if let Some(d) = self.def_of_var(m, *v) {
                    return self.definition(m, &Target::Def(d));
                }
                let span = *self.walk(m).binders.get(v)?;
                Some(at(m, span))
            }
            Target::Def(d) => Some(at(d.module, p.def(*d).span)),
            Target::Ctor(c) => {
                let module = p.ty(c.ty).module;
                Some(at(module, self.ctor_def(*c)?.span))
            }
            Target::Type(TypeRef::Custom(t)) => {
                let ty = p.ty(*t);
                Some(at(ty.module, ty.span))
            }
            Target::Field(fields) => {
                let (c, i) = *fields.first()?;
                let f = self.ctor_def(c)?.fields.get(i as usize)?;
                Some(at(p.ty(c.ty).module, f.label.as_ref()?.span))
            }
            Target::Module(t) => Some(at(*t, Span::default())),
            Target::Type(TypeRef::Builtin(_)) | Target::TyVar(_) | Target::Literal(_) => None,
        }
    }

    /// The smallest name or literal at `offset`, with its span.
    fn target_at(&self, m: ModuleId, offset: u32) -> Option<(Span, Target)> {
        let text = self.text(m);
        let word = word_at(text, offset);
        let w = self.walk(m);
        let found = w
            .occ
            .iter()
            .filter(|(s, _)| s.start < s.end && s.start <= offset && offset <= s.end)
            .min_by_key(|(s, _)| s.end - s.start)
            .cloned();
        // `m` in `m.x`: the HIR has one name for `m.x`.
        if let Some(word) = word
            && found.as_ref().is_none_or(|(s, _)| *s != word)
            && text.as_bytes().get(word.end as usize) == Some(&b'.')
        {
            let name = &text[word.start as usize..word.end as usize];
            if let Some(t) = self.import_module(m, name) {
                return Some((word, Target::Module(t)));
            }
        }
        found
    }

    fn import_module(&self, m: ModuleId, name: &str) -> Option<ModuleId> {
        self.program
            .module(m)
            .import_names
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| *t)
    }

    /// Every name and literal of a module, and the binder of every local
    /// name.
    fn walk(&self, m: ModuleId) -> Walk<'_> {
        let mut w = Walk {
            q: self,
            m,
            text: self.text(m),
            occ: Vec::new(),
            binders: HashMap::new(),
            fields: Vec::new(),
        };
        let p = self.program;
        let module = p.module(m);
        for b in &module.bindings {
            w.pattern(&b.binding.pattern);
            w.expr(&b.binding.value);
        }
        for pattern in &p.alias_patterns {
            if pattern.module != m {
                continue;
            }
            for (span, head) in &pattern.heads {
                w.occ.push((*span, Target::Type(*head)));
            }
            w.ty(&pattern.source);
        }
        for (i, d) in module.defs.iter().enumerate() {
            let id = DefId {
                module: m,
                index: i as u32,
            };
            w.occ.push((d.span, Target::Def(id)));
            if let DefKind::External { ty, .. } = &d.kind {
                w.ty(ty);
            }
        }
        for &t in &module.types {
            let def = p.ty(t);
            w.occ.push((def.span, Target::Type(TypeRef::Custom(t))));
            match &def.body {
                TypeBody::Alias(ty) => w.ty(ty),
                TypeBody::Custom(cs) => {
                    for (ci, c) in cs.iter().enumerate() {
                        let cref = CtorRef {
                            ty: t,
                            index: ci as u32,
                        };
                        w.occ.push((c.span, Target::Ctor(cref)));
                        for (fi, f) in c.fields.iter().enumerate() {
                            if let Some(l) = &f.label {
                                w.occ.push((l.span, Target::Field(vec![(cref, fi as u32)])));
                            }
                            w.ty(&f.ty);
                        }
                    }
                }
            }
        }
        self.walk_imports_exports(m, &mut w.occ);
        w
    }

    /// The names in `import` and `export`, which the HIR does not keep.
    fn walk_imports_exports(&self, m: ModuleId, occ: &mut Vec<(Span, Target)>) {
        let ast = self.ast(m);
        for item in &ast.imports {
            let alias = item.alias.as_ref().or(item.path.last());
            let Some(target) = alias.and_then(|a| self.import_module(m, &a.name)) else {
                continue;
            };
            if let Some(a) = &item.alias {
                occ.push((a.span, Target::Module(target)));
            }
            if let (Some(first), Some(last)) = (item.path.first(), item.path.last()) {
                occ.push((first.span.to(last.span), Target::Module(target)));
            }
            for n in &item.names {
                occ.extend(self.member_target(target, &n.name).map(|t| (n.span, t)));
            }
        }
        for e in &ast.exports {
            occ.extend(
                self.member_target(m, &e.name.name)
                    .map(|t| (e.name.span, t)),
            );
        }
    }

    /// The top-level name, or else the type, with this name in a module.
    fn member_target(&self, m: ModuleId, name: &str) -> Option<Target> {
        let module = self.program.module(m);
        if let Some(i) = module.defs.iter().position(|d| d.name == name) {
            return Some(Target::Def(DefId {
                module: m,
                index: i as u32,
            }));
        }
        if let Some(&t) = module
            .types
            .iter()
            .find(|t| self.program.ty(**t).name == name)
        {
            return Some(Target::Type(TypeRef::Custom(t)));
        }
        for &t in &module.types {
            if let TypeBody::Custom(cs) = &self.program.ty(t).body
                && let Some(i) = cs.iter().position(|c| c.name == name)
            {
                return Some(Target::Ctor(CtorRef {
                    ty: t,
                    index: i as u32,
                }));
            }
        }
        None
    }

    // ---- completions_at ----

    /// The names that may be written at `offset`. After `m.`, where `m` is
    /// a module, the exported members of `m`. After `v.`, the fields of the
    /// known type of `v` (§4.8, §6.7). Else the local names in scope, the
    /// top-level names and types of the module, the imported names, the
    /// built-in names.
    pub fn completions_at(&self, m: ModuleId, offset: u32) -> Vec<Completion> {
        let text = self.text(m);
        let bytes = text.as_bytes();
        let offset = (offset as usize).min(text.len());
        let mut start = offset;
        while start > 0 && is_word_byte(bytes[start - 1]) {
            start -= 1;
        }
        if start > 0 && bytes[start - 1] == b'.' {
            let end = start - 1;
            let mut base = end;
            while base > 0 && is_word_byte(bytes[base - 1]) {
                base -= 1;
            }
            let found = if base == end {
                Vec::new()
            } else {
                self.member_completions(m, Span::new(base as u32, end as u32))
            };
            if !found.is_empty() {
                return found;
            }
            // Any other expression before the dot, such as `f(x).` or
            // `{ t }.`: what the checker knew of its value there.
            return self.field_completions(m, start as u32);
        }
        self.scope_completions(m, offset as u32)
    }

    /// The fields that may be written after the dot of the field access
    /// whose name starts at `name_start`, from the known type of the value
    /// before the dot (§6.7).
    fn field_completions(&self, m: ModuleId, name_start: u32) -> Vec<Completion> {
        let w = self.walk(m);
        let Some(&(_, span)) = w.fields.iter().find(|(n, _)| n.start == name_start) else {
            return Vec::new();
        };
        let ctors = self
            .types
            .and_then(|t| t.locals.as_ref())
            .and_then(|l| l.field_ctors(m, span).map(<[CtorRef]>::to_vec))
            .unwrap_or_default();
        self.fields_of_ctors(&ctors)
    }

    /// The members that may be written after `base.`, where `base` is the
    /// span of a name in the text of the module (the text need not have
    /// the dot: it may be the text before the dot was written).
    pub fn member_completions_of(&self, m: ModuleId, base: Span) -> Vec<Completion> {
        self.member_completions(m, base)
    }

    /// The members after `base.`.
    fn member_completions(&self, m: ModuleId, base: Span) -> Vec<Completion> {
        let text = self.text(m);
        let name = &text[base.start as usize..base.end as usize];
        let w = self.walk(m);
        // The name or field that ends at the end of `base`, the smallest.
        let found = w
            .occ
            .iter()
            .filter(|(s, _)| s.end == base.end && s.start <= base.start)
            .min_by_key(|(s, _)| s.end - s.start)
            .map(|(s, t)| (*s, t.clone()));
        let local = matches!(&found, Some((s, Target::Local(_))) if *s == base);
        if !local && let Some(t) = self.import_module(m, name) {
            return self.module_members(t);
        }
        // §7.5: a name known to be made by one constructor.
        if let Some(c) = self.types.and_then(|t| t.known_ctors.get(&(m, base))) {
            return self.fields_of_ctors(&[*c]);
        }
        let mut types: Vec<TypeRef> = Vec::new();
        match found {
            Some((_, Target::Local(v))) => {
                let var = self
                    .def_of_var(m, v)
                    .and_then(|d| match self.program.def(d).kind {
                        DefKind::Binding { var, .. } => Some(var),
                        DefKind::External { .. } => None,
                    })
                    .unwrap_or(v);
                types = self.var_types(var);
            }
            Some((_, Target::Def(d))) => {
                if let DefKind::Binding { var, .. } = self.program.def(d).kind {
                    types = self.var_types(var);
                }
            }
            Some((_, Target::Field(fields))) => {
                for (c, i) in fields {
                    if let Some(f) = self.ctor_def(c).and_then(|c| c.fields.get(i as usize)) {
                        named_types(&f.ty, &mut types);
                    }
                }
            }
            _ => {}
        }
        let mut ctors = Vec::new();
        for t in types {
            if let TypeRef::Custom(t) = t
                && let TypeBody::Custom(cs) = &self.program.ty(t).body
            {
                ctors.extend((0..cs.len() as u32).map(|index| CtorRef { ty: t, index }));
            }
        }
        self.fields_of_ctors(&ctors)
    }

    fn var_types(&self, v: VarId) -> Vec<TypeRef> {
        self.types
            .and_then(|t| t.locals.as_ref())
            .map(|l| l.named_types(v))
            .unwrap_or_default()
    }

    /// The labels that every one of the constructors has (§6.7).
    fn fields_of_ctors(&self, ctors: &[CtorRef]) -> Vec<Completion> {
        let mut out: Vec<Completion> = Vec::new();
        let Some(first) = ctors.first().and_then(|c| self.ctor_def(*c)) else {
            return out;
        };
        for f in &first.fields {
            let Some(label) = &f.label else { continue };
            let everywhere = ctors.iter().all(|c| {
                self.ctor_def(*c).is_some_and(|d| {
                    d.fields
                        .iter()
                        .any(|g| g.label.as_ref().is_some_and(|l| l.name == label.name))
                })
            });
            if everywhere && !out.iter().any(|c| c.label == label.name) {
                out.push(Completion {
                    label: label.name.clone(),
                    kind: CompletionKind::Field,
                    detail: Some(self.show_type(&f.ty)),
                });
            }
        }
        out
    }

    /// The exported names, types, and constructors of a module.
    fn module_members(&self, t: ModuleId) -> Vec<Completion> {
        let p = self.program;
        let mut out = Vec::new();
        for (i, d) in p.module(t).defs.iter().enumerate() {
            if d.exported {
                out.push(self.def_completion(DefId {
                    module: t,
                    index: i as u32,
                }));
            }
        }
        for &ty in &p.module(t).types {
            let def = p.ty(ty);
            if def.exported {
                out.push(self.type_completion(TypeRef::Custom(ty)));
            }
            if def.constructors_exported {
                out.extend(self.ctor_completions(ty));
            }
        }
        out
    }

    fn def_completion(&self, d: DefId) -> Completion {
        let def = self.program.def(d);
        let detail = self.def_type(d);
        let is_fn = match &def.kind {
            DefKind::External { .. } => true,
            DefKind::Binding { binding, .. } => matches!(
                self.program.module(d.module).bindings[*binding as usize]
                    .binding
                    .value
                    .kind,
                ExprKind::Fn(_)
            ),
        };
        Completion {
            label: def.name.clone(),
            kind: if is_fn {
                CompletionKind::Function
            } else {
                CompletionKind::Value
            },
            detail,
        }
    }

    fn type_completion(&self, t: TypeRef) -> Completion {
        Completion {
            label: self.type_ref_name(t),
            kind: CompletionKind::Type,
            detail: match t {
                TypeRef::Custom(t) => Some(self.type_definition(t)),
                TypeRef::Builtin(b) => Some(builtin_head(b)),
            },
        }
    }

    fn ctor_completions(&self, t: TypeId) -> Vec<Completion> {
        let TypeBody::Custom(cs) = &self.program.ty(t).body else {
            return Vec::new();
        };
        (0..cs.len() as u32)
            .map(|index| {
                let c = CtorRef { ty: t, index };
                Completion {
                    label: cs[index as usize].name.clone(),
                    kind: CompletionKind::Constructor,
                    detail: Some(self.ctor_type(c)),
                }
            })
            .collect()
    }

    fn scope_completions(&self, m: ModuleId, offset: u32) -> Vec<Completion> {
        let p = self.program;
        let module = p.module(m);
        let mut out: Vec<Completion> = Vec::new();
        // Local names, the innermost first.
        let mut scope = Vec::new();
        Scope {
            offset,
            out: &mut scope,
            direct: Vec::new(),
        }
        .module(module);
        for b in scope.iter().rev() {
            out.push(Completion {
                label: b.name.clone(),
                kind: CompletionKind::Local,
                detail: self.local_type(m, b.var, offset),
            });
        }
        // The top-level names of the module.
        for i in 0..module.defs.len() as u32 {
            out.push(self.def_completion(DefId {
                module: m,
                index: i,
            }));
        }
        for &t in &module.types {
            out.push(self.type_completion(TypeRef::Custom(t)));
            out.extend(self.ctor_completions(t));
        }
        // The imports: modules, and names put in by `(A, b) = path`.
        for (name, t) in &module.import_names {
            out.push(Completion {
                label: name.clone(),
                kind: CompletionKind::Module,
                detail: Some(format!("module {}", p.module(*t).path)),
            });
        }
        let ast = self.ast(m);
        for item in &ast.imports {
            let alias = item.alias.as_ref().or(item.path.last());
            let Some(target) = alias.and_then(|a| self.import_module(m, &a.name)) else {
                continue;
            };
            for n in &item.names {
                match self.member_target(target, &n.name) {
                    Some(Target::Def(d)) => out.push(self.def_completion(d)),
                    Some(
                        Target::Type(TypeRef::Custom(t)) | Target::Ctor(CtorRef { ty: t, .. }),
                    ) => {
                        out.push(self.type_completion(TypeRef::Custom(t)));
                        out.extend(
                            self.ctor_completions(t)
                                .into_iter()
                                .filter(|c| c.label == n.name),
                        );
                    }
                    _ => {}
                }
            }
        }
        // The built-in names (§12).
        if let Some(prelude) = p.prelude {
            let pm = p.module(prelude);
            for (i, d) in pm.defs.iter().enumerate() {
                if d.exported {
                    out.push(self.def_completion(DefId {
                        module: prelude,
                        index: i as u32,
                    }));
                }
            }
            for &t in &pm.types {
                out.push(self.type_completion(TypeRef::Custom(t)));
                out.extend(self.ctor_completions(t));
            }
        }
        for b in BuiltinType::ALL {
            out.push(self.type_completion(TypeRef::Builtin(b)));
        }
        // A name covers the names after it with the same name and kind.
        let mut seen = HashSet::new();
        out.retain(|c| {
            let space = matches!(c.kind, CompletionKind::Type);
            seen.insert((c.label.clone(), space))
        });
        out
    }

    // ---- symbols ----

    /// The top-level definitions of a module, in source order: bindings,
    /// declarations, and types with their constructors.
    pub fn symbols(&self, m: ModuleId) -> Vec<Symbol> {
        let p = self.program;
        let module = p.module(m);
        let ast = self.ast(m);
        let mut out = Vec::new();
        for item in &ast.items {
            let within = |s: Span| item.span.start <= s.start && s.end <= item.span.end;
            match &item.kind {
                ast::ItemKind::MacroItem { .. } => {}
                ast::ItemKind::TypeDef(def) if def.alias_pattern => {
                    for pattern in &p.alias_patterns {
                        if pattern.module != m || pattern.span != item.span {
                            continue;
                        }
                        for id in &pattern.binders {
                            let t = p.ty(*id);
                            out.push(Symbol {
                                name: t.name.clone(),
                                kind: SymbolKind::Type,
                                span: item.span,
                                name_span: t.span,
                                detail: Some(self.type_definition(*id)),
                                children: Vec::new(),
                            });
                        }
                    }
                }
                ast::ItemKind::TypeDef(def) => {
                    let Some(&t) = module
                        .types
                        .iter()
                        .find(|t| p.ty(**t).span == def.name.span)
                    else {
                        continue;
                    };
                    let mut children = Vec::new();
                    if let ast::TypeBody::Custom(cs) = &def.body {
                        for (i, c) in cs.iter().enumerate() {
                            children.push(Symbol {
                                name: c.name.name.clone(),
                                kind: SymbolKind::Constructor,
                                span: c.span,
                                name_span: c.name.span,
                                detail: Some(self.ctor_type(CtorRef {
                                    ty: t,
                                    index: i as u32,
                                })),
                                children: Vec::new(),
                            });
                        }
                    }
                    out.push(Symbol {
                        name: def.name.name.clone(),
                        kind: SymbolKind::Type,
                        span: item.span,
                        name_span: def.name.span,
                        detail: Some(self.type_head(t)),
                        children,
                    });
                }
                ast::ItemKind::Binding(_) | ast::ItemKind::Declaration { .. } => {
                    for (i, d) in module.defs.iter().enumerate() {
                        if !within(d.span) {
                            continue;
                        }
                        let c = self.def_completion(DefId {
                            module: m,
                            index: i as u32,
                        });
                        out.push(Symbol {
                            name: d.name.clone(),
                            kind: if c.kind == CompletionKind::Function {
                                SymbolKind::Function
                            } else {
                                SymbolKind::Value
                            },
                            span: item.span,
                            name_span: d.span,
                            detail: c.detail,
                            children: Vec::new(),
                        });
                    }
                }
            }
        }
        out
    }
}

fn with_type(name: &str, ty: Option<&str>) -> String {
    match ty {
        Some(t) => format!("{name}: {t}"),
        None => name.to_string(),
    }
}

fn builtin_head(b: BuiltinType) -> String {
    const PARAMS: [&str; 4] = ["A", "R", "E", "N"];
    match b {
        BuiltinType::List => "List(A)".into(),
        BuiltinType::Boundary => "Boundary(R, E, N)".into(),
        BuiltinType::Continuation => format!("Continuation({})", PARAMS.join(", ")),
        b => b.name().into(),
    }
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The span of the word (a name, an `UpName`, or a number) at `offset`.
fn word_at(text: &str, offset: u32) -> Option<Span> {
    let bytes = text.as_bytes();
    let offset = (offset as usize).min(bytes.len());
    let mut start = offset;
    while start > 0 && is_word_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = offset;
    while end < bytes.len() && is_word_byte(bytes[end]) {
        end += 1;
    }
    (start < end).then(|| Span::new(start as u32, end as u32))
}

/// The span of the last word in `span`: `x` of `m.x`.
fn last_word(text: &str, span: Span) -> Span {
    let bytes = text.as_bytes();
    let mut end = (span.end as usize).min(bytes.len());
    while end > span.start as usize && !is_word_byte(bytes[end - 1]) {
        end -= 1;
    }
    let mut start = end;
    while start > span.start as usize && is_word_byte(bytes[start - 1]) {
        start -= 1;
    }
    if start == end {
        span
    } else {
        Span::new(start as u32, end as u32)
    }
}

/// The span of the name at the start of `span`, after `m.` if it is
/// there: `C` of `m.C(x, y)`.
fn head_word(text: &str, span: Span) -> Span {
    let bytes = text.as_bytes();
    let limit = (span.end as usize).min(bytes.len());
    let mut start = span.start as usize;
    loop {
        let mut end = start;
        while end < limit && is_word_byte(bytes[end]) {
            end += 1;
        }
        if end == start {
            return span;
        }
        if end < limit && bytes[end] == b'.' && end + 1 < limit && is_word_byte(bytes[end + 1]) {
            start = end + 1;
            continue;
        }
        return Span::new(start as u32, end as u32);
    }
}

fn named_types(t: &Type, out: &mut Vec<TypeRef>) {
    match &t.kind {
        TypeKind::Named { ty, .. } => out.push(*ty),
        TypeKind::Union(ms) | TypeKind::Inter(ms) => ms.iter().for_each(|m| named_types(m, out)),
        TypeKind::Diff(ms) => {
            if let Some(m) = ms.first() {
                named_types(m, out);
            }
        }
        _ => {}
    }
}

/// The names of a module, gathered from the HIR.
struct Walk<'q> {
    q: &'q Query<'q>,
    m: ModuleId,
    text: &'q str,
    occ: Vec<(Span, Target)>,
    binders: HashMap<VarId, Span>,
    /// Each field access `e.f`: the span of `f`, and of `e.f`.
    fields: Vec<(Span, Span)>,
}

impl Walk<'_> {
    fn binder(&mut self, b: &Binder) {
        if b.name.starts_with('%') {
            return;
        }
        self.binders.entry(b.var).or_insert(b.span);
        self.occ.push((b.span, Target::Local(b.var)));
    }

    fn pattern(&mut self, p: &Pattern) {
        match &p.kind {
            PatternKind::Quote { captures, .. } => {
                captures.iter().for_each(|(_, p)| self.pattern(p))
            }
            PatternKind::Var(b) => self.binder(b),
            PatternKind::Discard
            | PatternKind::String(_)
            | PatternKind::Int { .. }
            | PatternKind::Float { .. } => {}
            PatternKind::Typed { binder, ty } => {
                if let Some(b) = binder {
                    self.binder(b);
                }
                self.ty(ty);
            }
            PatternKind::Named { binder, pattern } => {
                self.binder(binder);
                self.pattern(pattern);
            }
            PatternKind::Prefix { rest, .. } => {
                if let Some(b) = rest {
                    self.binder(b);
                }
            }
            PatternKind::Constructor { ctor, args, .. } => {
                self.occ
                    .push((head_word(self.text, p.span), Target::Ctor(*ctor)));
                args.iter().for_each(|a| self.pattern(a));
            }
            PatternKind::Tuple(ps) | PatternKind::Error(ps) => {
                ps.iter().for_each(|a| self.pattern(a))
            }
            PatternKind::List { items, rest } => {
                items.iter().for_each(|a| self.pattern(a));
                if let Some(Some(b)) = rest {
                    self.binder(b);
                }
            }
            PatternKind::BitArray(segs) => {
                for s in segs {
                    self.pattern(&s.value);
                    self.format(s.format.as_ref());
                }
            }
        }
    }

    fn format(&mut self, f: Option<&Format>) {
        if let Some(a) = f.and_then(|f| f.arg.as_ref()) {
            self.expr(a);
        }
    }

    fn ty(&mut self, t: &Type) {
        match &t.kind {
            TypeKind::Named { ty, args } => {
                self.occ
                    .push((head_word(self.text, t.span), Target::Type(*ty)));
                args.iter().for_each(|a| self.ty(a));
            }
            TypeKind::Var(v) => self.occ.push((t.span, Target::TyVar(*v))),
            TypeKind::Of { value, .. } => {
                let target = match value {
                    ValueRef::Local(v) => Target::Local(*v),
                    ValueRef::Def(d) => Target::Def(*d),
                    ValueRef::Ctor(c) => Target::Ctor(*c),
                };
                self.occ.push((last_word(self.text, t.span), target));
            }
            TypeKind::Lit(_) | TypeKind::Hole | TypeKind::Error => {}
            TypeKind::Tuple(ts)
            | TypeKind::Union(ts)
            | TypeKind::Inter(ts)
            | TypeKind::Diff(ts) => ts.iter().for_each(|a| self.ty(a)),
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => {
                params.iter().for_each(|p| self.ty(&p.ty));
                self.ty(ret);
                err.iter().chain(env.iter()).for_each(|x| self.ty(x));
            }
        }
    }

    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            match s {
                Stmt::Bind(b) => {
                    self.pattern(&b.pattern);
                    self.expr(&b.value);
                }
                Stmt::Env { ty, value, .. } => {
                    self.ty(ty);
                    self.expr(value);
                }
                Stmt::Expr(e) => self.expr(e),
            }
        }
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Int(_) => self.occ.push((e.span, Target::Literal(BuiltinType::Int))),
            ExprKind::Float(_) => self.occ.push((e.span, Target::Literal(BuiltinType::Float))),
            ExprKind::String(_) => self
                .occ
                .push((e.span, Target::Literal(BuiltinType::String))),
            ExprKind::Name(v) => {
                if e.span.start == e.span.end {
                    return;
                }
                let span = last_word(self.text, e.span);
                let t = match v {
                    ValueRef::Local(v) => Target::Local(*v),
                    ValueRef::Def(d) => Target::Def(*d),
                    ValueRef::Ctor(c) => Target::Ctor(*c),
                };
                self.occ.push((span, t));
            }
            ExprKind::Tuple(xs) => xs.iter().for_each(|x| self.expr(x)),
            ExprKind::List { items, rest } => {
                items.iter().for_each(|x| self.expr(x));
                if let Some(r) = rest {
                    self.expr(r);
                }
            }
            ExprKind::BitArray(segs) => {
                for s in segs {
                    self.expr(&s.value);
                    self.format(s.format.as_ref());
                }
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Fn(f) => {
                for p in &f.params {
                    match p {
                        Param::Pattern { pattern, .. } => self.pattern(pattern),
                        Param::Env { var, name, ty } => {
                            self.binders.entry(*var).or_insert(name.span);
                            self.occ.push((name.span, Target::Local(*var)));
                            self.ty(ty);
                        }
                    }
                }
                if let Some(r) = &f.ret {
                    self.ty(&r.value);
                    r.err.iter().chain(r.env.iter()).for_each(|t| self.ty(t));
                }
                self.block(&f.body);
            }
            ExprKind::Case { subject, clauses } => {
                self.expr(subject);
                for c in clauses {
                    c.patterns.iter().for_each(|p| self.pattern(p));
                    if let Some(g) = &c.guard {
                        self.expr(g);
                    }
                    self.expr(&c.body);
                }
            }
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::Field { target, name } => {
                self.expr(target);
                self.fields.push((name.span, e.span));
                let access = self.q.types.and_then(|t| t.access.get(&(self.m, e.span)));
                if let Some(Access::Field(fields)) = access {
                    self.occ.push((name.span, Target::Field(fields.clone())));
                }
            }
            ExprKind::TupleIndex { target, .. } => self.expr(target),
            ExprKind::RecordUpdate {
                ctor,
                ctor_span,
                base,
                fields,
            } => {
                self.occ
                    .push((last_word(self.text, *ctor_span), Target::Ctor(*ctor)));
                self.expr(base);
                let def = self.q.ctor_def(*ctor);
                for (label, value) in fields {
                    let index = def.and_then(|d| {
                        d.fields
                            .iter()
                            .position(|f| f.label.as_ref().is_some_and(|l| l.name == label.name))
                    });
                    if let Some(i) = index {
                        self.occ
                            .push((label.span, Target::Field(vec![(*ctor, i as u32)])));
                    }
                    self.expr(value);
                }
            }
            ExprKind::Unary { expr, .. } => self.expr(expr),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            ExprKind::Error => {}
        }
    }
}

/// The local names in scope at an offset (§5), outermost first.
struct Scope<'o> {
    offset: u32,
    out: &'o mut Vec<Binder>,
    /// The names of the direct values that the offset is in: they are in
    /// scope only inside the function bodies of the value (§6.10).
    direct: Vec<Binder>,
}

fn contains(span: Span, offset: u32) -> bool {
    span.start <= offset && offset <= span.end
}

fn pattern_binders(p: &Pattern, out: &mut Vec<Binder>) {
    match &p.kind {
        PatternKind::Quote { captures, .. } => {
            captures.iter().for_each(|(_, p)| pattern_binders(p, out))
        }
        PatternKind::Var(b) => out.push(b.clone()),
        PatternKind::Typed { binder, .. } => out.extend(binder.clone()),
        PatternKind::Named { binder, pattern } => {
            out.push(binder.clone());
            pattern_binders(pattern, out);
        }
        PatternKind::Prefix { rest, .. } => out.extend(rest.clone()),
        PatternKind::Constructor { args: ps, .. }
        | PatternKind::Tuple(ps)
        | PatternKind::Error(ps) => ps.iter().for_each(|q| pattern_binders(q, out)),
        PatternKind::List { items, rest } => {
            items.iter().for_each(|q| pattern_binders(q, out));
            if let Some(Some(b)) = rest {
                out.push(b.clone());
            }
        }
        PatternKind::BitArray(segs) => segs.iter().for_each(|s| pattern_binders(&s.value, out)),
        _ => {}
    }
    out.retain(|b| !b.name.starts_with('%'));
}

impl Scope<'_> {
    fn module(&mut self, m: &Module) {
        if let Some(b) = m
            .bindings
            .iter()
            .find(|b| contains(b.binding.span, self.offset))
        {
            self.expr(&b.binding.value);
        }
    }

    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            let span = match s {
                Stmt::Bind(b) => Span::new(
                    b.span.start.min(b.value.span.start),
                    b.span.end.max(b.value.span.end),
                ),
                Stmt::Env { span, .. } => *span,
                Stmt::Expr(e) => e.span,
            };
            if contains(span, self.offset) {
                match s {
                    Stmt::Bind(b) => {
                        // §6.10: the names of a direct value may be used in
                        // the function bodies of the value.
                        if b.direct {
                            pattern_binders(&b.pattern, &mut self.direct);
                        }
                        self.expr(&b.value);
                    }
                    Stmt::Env { value, .. } => self.expr(value),
                    Stmt::Expr(e) => self.expr(e),
                }
                return;
            }
            if span.end > self.offset {
                return;
            }
            if let Stmt::Bind(b) = s {
                pattern_binders(&b.pattern, self.out);
            }
        }
    }

    fn expr(&mut self, e: &Expr) {
        if !contains(e.span, self.offset) {
            return;
        }
        match &e.kind {
            ExprKind::Fn(f) => {
                self.out.append(&mut self.direct);
                for p in &f.params {
                    match p {
                        Param::Pattern { pattern, .. } => pattern_binders(pattern, self.out),
                        Param::Env { var, name, .. } => self.out.push(Binder {
                            var: *var,
                            name: name.name.clone(),
                            span: name.span,
                        }),
                    }
                }
                self.block(&f.body);
            }
            ExprKind::Block(b) => self.block(b),
            ExprKind::Case { subject, clauses } => {
                self.expr(subject);
                if let Some(c) = clauses.iter().find(|c| contains(c.span, self.offset)) {
                    if let Some(p) = c.patterns.first() {
                        pattern_binders(p, self.out);
                    }
                    if let Some(g) = &c.guard {
                        self.expr(g);
                    }
                    self.expr(&c.body);
                }
            }
            ExprKind::Tuple(xs) => xs.iter().for_each(|x| self.expr(x)),
            ExprKind::List { items, rest } => {
                items.iter().for_each(|x| self.expr(x));
                if let Some(r) = rest {
                    self.expr(r);
                }
            }
            ExprKind::BitArray(segs) => segs.iter().for_each(|s| self.expr(&s.value)),
            ExprKind::Call { callee, args } => {
                self.expr(callee);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
                self.expr(target)
            }
            ExprKind::RecordUpdate { base, fields, .. } => {
                self.expr(base);
                fields.iter().for_each(|(_, x)| self.expr(x));
            }
            ExprKind::Unary { expr, .. } => self.expr(expr),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            _ => {}
        }
    }
}
