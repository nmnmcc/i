//! Completion by the place of the position, and signature help.
//!
//! As rust-analyzer does, the place is found in the resolved tree around
//! the position (a type, a pattern of a match, the arguments of a call),
//! and only for what is not in the tree yet, from the token just before the
//! word being written.

use super::nav::{Node, visit_module};
use super::*;

/// What kind of place a completion is at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionContext {
    /// In the imports: of the names of a module in `(…) = path`, or of the
    /// module paths, written from `start`.
    Import {
        names_of: Option<ModuleId>,
        start: u32,
    },
    /// After `!`: a service type (§9.2).
    Environment,
    /// A type.
    Type,
    /// A pattern of a match whose subject has these types.
    Pattern { types: Vec<TypeId> },
    /// An expression, maybe an argument of the call with this span.
    Expression { call: Option<Span> },
}

/// The signature of the function of a call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    /// `name(a: A, b: B) -> C`.
    pub label: String,
    pub params: Vec<SignatureParam>,
    /// The parameter that the argument at the position goes to.
    pub active: Option<u32>,
    pub documentation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureParam {
    /// The offsets of the parameter in the label.
    pub start: u32,
    pub end: u32,
    pub label: Option<String>,
}

impl Query<'_> {
    /// The place of a completion at `offset`.
    pub fn completion_context(&self, m: ModuleId, offset: u32) -> CompletionContext {
        let text = self.written_text(m);
        let offset = offset.min(text.len() as u32);
        let start = word_start(text, offset);
        let ast = self.ast(m);
        // The imports.
        let in_imports = ast.imports_span.is_some_and(|s| contains(s, offset))
            || ast.imports.iter().any(|i| contains(i.span, offset));
        if in_imports {
            for item in &ast.imports {
                let (Some(first), Some(_)) = (item.path.first(), item.path.last()) else {
                    continue;
                };
                if contains(item.span, offset)
                    && offset < first.span.start
                    && !item.names.is_empty()
                    || item.names.iter().any(|n| contains(n.span, offset))
                {
                    let alias = item.alias.as_ref().or(item.path.last());
                    let t = alias.and_then(|a| self.import_module(m, &a.name));
                    return CompletionContext::Import { names_of: t, start };
                }
            }
            return CompletionContext::Import {
                names_of: None,
                start: path_start(text, offset),
            };
        }
        let before = prev_byte(text, start);
        if before == Some(b'!') {
            return CompletionContext::Environment;
        }
        // In the resolved tree.
        let p = self.program;
        let mut in_type = false;
        let mut case: Option<&Expr> = None;
        let mut pipe_subject: Option<&Expr> = None;
        let mut call: Option<&Expr> = None;
        let mut record = false;
        visit_module(p, m, &mut |n| match n {
            // Not a hole: the lowering of a provide `e <- (v)` gives each
            // value `v` the type `_` with the span of the value.
            Node::Type(t)
                if t.span.start <= start
                    && offset <= t.span.end
                    && t.span.start < t.span.end
                    && !matches!(t.kind, TypeKind::Hole) =>
            {
                in_type = true
            }
            Node::Expr(e) if e.span.start < offset && offset <= e.span.end => match &e.kind {
                ExprKind::Case { .. } if case.is_none_or(|c| c.span.start <= e.span.start) => {
                    case = Some(e)
                }
                ExprKind::Binary { op: ilang_syntax::ast::BinOp::Pipe, lhs, .. }
                    if pipe_subject.is_none_or(|old| old.span.start <= lhs.span.start) =>
                {
                    pipe_subject = Some(lhs)
                }
                ExprKind::Call { callee, .. }
                    if callee.span.end <= start
                        && offset < e.span.end
                        && call.is_none_or(|c| c.span.start <= e.span.start) =>
                {
                    call = Some(e)
                }
                ExprKind::Record { .. } => record = true,
                _ => {}
            },
            _ => {}
        });
        // A service pattern `<- (name: S, …)` of a function head, or the
        // need `<- (S, …)` of a function type (syntax three §2.10.2): a
        // type there, or the place after `name:`, is a service. After
        // `e <- (` of a provide the items are values, which are not types
        // of the tree.
        if (in_type || before == Some(b':')) && after_need_arrow(text, start) {
            return CompletionContext::Environment;
        }
        if in_type {
            return CompletionContext::Type;
        }
        if let Some(Expr {
            kind: ExprKind::Case { subject, clauses },
            ..
        }) = case
            && offset > subject.span.end
            && call.is_none_or(|c| c.span.start < subject.span.end)
        {
            let in_body = clauses.iter().any(|c| {
                contains(c.body.span, start) && c.body.span.start < c.body.span.end
                    || c.guard.as_ref().is_some_and(|g| contains(g.span, start))
            });
            let in_pattern = clauses
                .iter()
                .any(|c| c.patterns.iter().any(|p| contains(p.span, offset)));
            // A new clause starts a line, or follows the `{`, or in the
            // Lisp surface, where clauses are lists, a `(` or a `)`.
            let at_clause_start = line_before(text, start).trim().is_empty()
                || prev_byte(text, start) == Some(b'{')
                || self.is_lisp(m) && matches!(prev_byte(text, start), Some(b'(' | b')'));
            if in_pattern || (!in_body && at_clause_start) {
                return CompletionContext::Pattern {
                    types: self.subject_types(m, subject, clauses),
                };
            }
        }
        // An unfinished branch such as `| () { ... }` can lower as a
        // different-shape function instead of a Case. The source still puts
        // this cursor in the pattern head of the match, not its body.
        if let Some(subject) = pipe_subject
            && !self.is_lisp(m)
            && in_source_branch_head(text, offset)
        {
            return CompletionContext::Pattern {
                types: self.subject_types(m, subject, &[]),
            };
        }
        if before == Some(b':') && call.is_none() && !record {
            return CompletionContext::Type;
        }
        CompletionContext::Expression {
            call: call.map(|c| c.span),
        }
    }

    /// The custom types of the subject of a match: of its constructor
    /// patterns, or else of its known type.
    pub(crate) fn subject_types(
        &self,
        m: ModuleId,
        subject: &Expr,
        clauses: &[Clause],
    ) -> Vec<TypeId> {
        // The members of the subject's type (a sum is a union of
        // one-constructor types, syntax three §2.8), then the types of the
        // constructors that the clauses name.
        let mut out = Vec::new();
        let types = match &subject.kind {
            ExprKind::Name(ValueRef::Local(v)) => {
                let v = match self.canonical(m, Target::Local(*v)) {
                    Target::Def(d) => match self.program.def(d).kind {
                        DefKind::Binding { var, .. } => var,
                        DefKind::External { .. } => *v,
                    },
                    _ => *v,
                };
                self.var_types(v)
            }
            ExprKind::Name(ValueRef::Def(d)) => match self.program.def(*d).kind {
                DefKind::Binding { var, .. } => self.var_types(var),
                DefKind::External { .. } => Vec::new(),
            },
            _ => Vec::new(),
        };
        for t in types {
            if let TypeRef::Custom(t) = t
                && !out.contains(&t)
            {
                out.push(t);
            }
        }
        for c in clauses {
            for p in &c.patterns {
                if let PatternKind::Constructor { ctor, .. } = &p.kind
                    && !out.contains(&ctor.ty)
                {
                    out.push(ctor.ty);
                }
            }
        }
        out
    }

    pub(crate) fn context_completions(&self, m: ModuleId, offset: u32) -> Vec<Completion> {
        match self.completion_context(m, offset) {
            CompletionContext::Import {
                names_of: Some(t), ..
            } => self.module_members(t),
            CompletionContext::Import {
                names_of: None,
                start,
            } => self.import_completions(m, Span::new(start, offset)),
            CompletionContext::Environment => self.service_completions(m),
            CompletionContext::Type => self.type_completions(m, offset),
            CompletionContext::Pattern { types } => self.pattern_completions(m, offset, &types),
            CompletionContext::Expression { call } => {
                let mut out = Vec::new();
                if let Some(call) = call {
                    out.extend(self.label_completions(m, call));
                }
                out.extend(self.scope_completions(m, offset));
                let text = self.written_text(m);
                let start = word_start(text, offset);
                let prefix = &text[start as usize..offset as usize];
                if !prefix.is_empty() {
                    out.extend(self.auto_import_completions(m, prefix));
                }
                out
            }
        }
    }

    /// The modules that a module may import, by the path it writes them
    /// with (§2.1), with their exports as documentation.
    fn import_completions(&self, m: ModuleId, replace: Span) -> Vec<Completion> {
        let mut out = Vec::new();
        for (i, _) in self.program.modules.iter().enumerate() {
            let t = ModuleId(i as u32);
            if t == m {
                continue;
            }
            let Some(path) = self.import_path(m, t) else {
                continue;
            };
            let names: Vec<String> = self
                .module_members(t)
                .into_iter()
                .map(|c| c.label)
                .collect();
            let mut doc = self.module_doc(t).unwrap_or_default();
            if !names.is_empty() {
                if !doc.is_empty() {
                    doc.push_str("\n\n");
                }
                doc.push_str(&format!("Exports: {}", names.join(", ")));
            }
            out.push(Completion {
                label: path,
                kind: CompletionKind::Module,
                detail: Some("module".into()),
                documentation: (!doc.is_empty()).then_some(doc),
                replace: Some(replace),
                ..Default::default()
            });
        }
        out
    }

    /// The path that module `m` writes to import `t`, when it may import it
    /// (§2.1): a module of its package, of the library, or of a dependency,
    /// and a test module only from a test module.
    pub fn import_path(&self, m: ModuleId, t: ModuleId) -> Option<String> {
        let p = self.program;
        let (from, to) = (p.module(m), p.module(t));
        if Some(t) == p.prelude || to.path.is_empty() {
            return None;
        }
        if to.kind == ilang_package::ModuleKind::Test
            && (from.kind != ilang_package::ModuleKind::Test || from.package != to.package)
        {
            return None;
        }
        if to.kind == ilang_package::ModuleKind::Prelude {
            return None;
        }
        if from.package == to.package || to.package == self.graph.library {
            return Some(to.path.clone());
        }
        let package = &self.graph.packages[from.package];
        let rest = to.path.split_once('/').map(|x| x.1)?;
        let deps =
            package
                .dependencies
                .iter()
                .chain(if from.kind == ilang_package::ModuleKind::Test {
                    package.dev_dependencies.iter()
                } else {
                    [].iter()
                });
        for (name, id) in deps {
            if *id == Some(to.package) {
                return Some(format!("{name}/{rest}"));
            }
        }
        None
    }

    /// The `////` comment of a module.
    pub(crate) fn module_doc(&self, m: ModuleId) -> Option<String> {
        let ast = self.ast(m);
        if ast.doc.is_empty() {
            return None;
        }
        Some(
            ast.doc
                .iter()
                .map(|l| l.strip_prefix(' ').unwrap_or(l))
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }

    /// The types that may be written in a module: its own and imported
    /// types, the built-in ones, the modules (for `m.T`), and the type
    /// variables of the item around the position.
    fn type_completions(&self, m: ModuleId, offset: u32) -> Vec<Completion> {
        let p = self.program;
        let module = p.module(m);
        let mut out: Vec<Completion> = Vec::new();
        let w = self.walk(m);
        if let Some(item) = self.item_around(m, Span::new(offset, offset)) {
            for (s, t) in &w.occ {
                if let Target::TyVar(v) = t
                    && item.start <= s.start
                    && s.end <= item.end
                {
                    let name = self.ty_var_name(*v);
                    if !out.iter().any(|c| c.label == name) {
                        out.push(Completion {
                            label: name,
                            kind: CompletionKind::Type,
                            detail: Some("type variable".into()),
                            preferred: true,
                            ..Default::default()
                        });
                    }
                }
            }
        }
        for &t in module.types.iter().chain(&module.imported_types) {
            out.push(self.with_doc(self.type_completion(TypeRef::Custom(t))));
        }
        if let Some(prelude) = p.prelude {
            for &t in &p.module(prelude).types {
                out.push(self.type_completion(TypeRef::Custom(t)));
            }
        }
        for b in BuiltinType::ALL {
            out.push(self.type_completion(TypeRef::Builtin(b)));
        }
        for (name, t) in &module.import_names {
            out.push(Completion {
                label: name.clone(),
                kind: CompletionKind::Module,
                detail: Some(format!("module {}", p.module(*t).path)),
                ..Default::default()
            });
        }
        let mut seen = HashSet::new();
        out.retain(|c| seen.insert(c.label.clone()));
        out
    }

    /// The types that may be services (§9.2): named custom types that are
    /// not records. Those used as services in the program come first.
    fn service_completions(&self, m: ModuleId) -> Vec<Completion> {
        let p = self.program;
        let used: HashSet<TypeRef> = self
            .types
            .map(|t| t.services.values().copied().collect())
            .unwrap_or_default();
        let module = p.module(m);
        let mut ids: Vec<TypeId> = module
            .types
            .iter()
            .chain(&module.imported_types)
            .copied()
            .collect();
        if let Some(prelude) = p.prelude {
            ids.extend(&p.module(prelude).types);
        }
        let mut out = Vec::new();
        for t in ids {
            if !matches!(p.ty(t).body, TypeBody::Custom(_)) {
                continue;
            }
            let mut c = self.with_doc(self.type_completion(TypeRef::Custom(t)));
            c.preferred = used.contains(&TypeRef::Custom(t));
            out.push(c);
        }
        for (name, t) in &module.import_names {
            out.push(Completion {
                label: name.clone(),
                kind: CompletionKind::Module,
                detail: Some(format!("module {}", p.module(*t).path)),
                ..Default::default()
            });
        }
        out.sort_by_key(|c| !c.preferred);
        let mut seen = HashSet::new();
        out.retain(|c| seen.insert(c.label.clone()));
        out
    }

    /// The constructor patterns of the types of a match subject, as
    /// snippets, and `_`.
    fn pattern_completions(&self, m: ModuleId, offset: u32, types: &[TypeId]) -> Vec<Completion> {
        let p = self.program;
        let lisp = self.is_lisp(m);
        let mut out = Vec::new();
        for &t in types {
            let TypeBody::Custom(cs) = &p.ty(t).body else {
                continue;
            };
            let Some(prefix) = self.ctor_prefix(m, t) else {
                continue;
            };
            for (i, c) in cs.iter().enumerate() {
                let cref = CtorRef {
                    ty: t,
                    index: i as u32,
                };
                let name = format!("{prefix}{}", c.name);
                let mut parts: Vec<String> = Vec::new();
                for f in &c.fields {
                    if f.label.is_none() {
                        parts.push("_".into());
                    }
                }
                if c.fields.iter().any(|f| f.label.is_some()) {
                    parts.push("..".into());
                }
                let pattern = if c.fields.is_empty() {
                    name.clone()
                } else {
                    format!("{name}({})", parts.join(", "))
                };
                let pattern = if lisp {
                    super::actions::lisp_pattern(&pattern).unwrap_or(pattern)
                } else {
                    pattern
                };
                // Each `_` is a place of the snippet.
                let mut insert = String::new();
                let mut place = 0;
                let bytes = pattern.as_bytes();
                for (i, ch) in pattern.char_indices() {
                    let alone = ch == '_'
                        && (i == 0 || !is_word_byte(bytes[i - 1]))
                        && bytes.get(i + 1).is_none_or(|b| !is_word_byte(*b));
                    if alone {
                        place += 1;
                        insert.push_str(&format!("${{{place}:_}}"));
                    } else {
                        insert.push(ch);
                    }
                }
                out.push(Completion {
                    label: name,
                    kind: CompletionKind::Constructor,
                    detail: Some(self.ctor_type(cref)),
                    documentation: self.doc(p.ty(t).module, p.ty(t).span),
                    insert: Some(insert),
                    snippet: true,
                    preferred: true,
                    defined_in: Some(p.module(p.ty(t).module).path.clone()),
                    ..Default::default()
                });
            }
        }
        if types.is_empty() {
            out.extend(
                self.scope_completions(m, offset)
                    .into_iter()
                    .filter(|c| c.kind == CompletionKind::Constructor),
            );
        }
        out.push(Completion {
            label: "_".into(),
            kind: CompletionKind::Keyword,
            detail: Some("any value".into()),
            ..Default::default()
        });
        out
    }

    /// What a constructor of type `t` is written with in module `m`: no
    /// prefix where its name is in scope, else the name of its module.
    pub(crate) fn ctor_prefix(&self, m: ModuleId, t: TypeId) -> Option<String> {
        let p = self.program;
        let owner = p.ty(t).module;
        if owner == m || Some(owner) == p.prelude || p.module(m).imported_types.contains(&t) {
            return Some(String::new());
        }
        let TypeBody::Custom(cs) = &p.ty(t).body else {
            return None;
        };
        if self
            .ast(m)
            .imports
            .iter()
            .any(|i| i.names.iter().any(|n| cs.iter().any(|c| c.name == n.name)))
        {
            return Some(String::new());
        }
        p.module(m)
            .import_names
            .iter()
            .find(|(_, x)| *x == owner)
            .map(|(n, _)| format!("{n}."))
    }

    /// The labels of the parameters of the function of a call that its
    /// arguments do not give yet.
    fn label_completions(&self, m: ModuleId, call: Span) -> Vec<Completion> {
        let Some((callee, args)) = self.call_at(m, call) else {
            return Vec::new();
        };
        let given: Vec<&str> = args
            .iter()
            .filter_map(|a| a.label.as_ref().map(|l| l.name.as_str()))
            .collect();
        let lisp = self.is_lisp(m);
        self.callee_params(m, callee)
            .into_iter()
            .filter_map(|(label, ty)| {
                let label = label?;
                if given.contains(&label.as_str()) {
                    return None;
                }
                Some(Completion {
                    label: format!("{label} ="),
                    kind: CompletionKind::Field,
                    detail: ty,
                    insert: Some(if lisp {
                        format!("(= {label} ")
                    } else {
                        format!("{label} = ")
                    }),
                    preferred: true,
                    ..Default::default()
                })
            })
            .collect()
    }

    /// The call with this span in module `m`.
    fn call_at(&self, m: ModuleId, span: Span) -> Option<(&Expr, &[Arg])> {
        let mut found = None;
        visit_module(self.program, m, &mut |n| {
            if let Node::Expr(e) = n
                && e.span == span
                && let ExprKind::Call { callee, args } = &e.kind
            {
                found = Some((&**callee, args.as_slice()));
            }
        });
        found
    }

    /// The parameters of the function that `callee` names: their labels and
    /// types.
    fn callee_params(&self, m: ModuleId, callee: &Expr) -> Vec<(Option<String>, Option<String>)> {
        let p = self.program;
        match &callee.kind {
            ExprKind::Name(ValueRef::Ctor(c)) => self
                .ctor_def(*c)
                .map(|d| {
                    d.fields
                        .iter()
                        .map(|f| {
                            (
                                f.label.as_ref().map(|l| l.name.clone()),
                                Some(self.show_type(&f.ty)),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            ExprKind::Name(v) => {
                let d = match v {
                    ValueRef::Def(d) => Some(*d),
                    ValueRef::Local(v) => self.def_of_var(m, *v),
                    ValueRef::Ctor(_) => None,
                };
                let ty = match d {
                    Some(d) => self.def_type(d),
                    None => match v {
                        ValueRef::Local(v) => self.local_type(m, *v, callee.span.start),
                        _ => None,
                    },
                };
                let shown = ty.as_deref().and_then(split_fn_type);
                let labels: Vec<Option<String>> = match d.map(|d| &p.def(d).kind) {
                    Some(DefKind::Binding { binding, .. }) => {
                        let b = &p.module(d.unwrap().module).bindings[*binding as usize].binding;
                        match &b.value.kind {
                            ExprKind::Fn(f) => f
                                .params
                                .iter()
                                .filter_map(|x| match x {
                                    Param::Pattern { label, .. } => Some(label.clone()),
                                    Param::Env { .. } => None,
                                })
                                .collect(),
                            _ => Vec::new(),
                        }
                    }
                    Some(DefKind::External { ty, .. }) => match &ty.kind {
                        TypeKind::Fn { params, .. } => params
                            .iter()
                            .map(|x| x.label.as_ref().map(|l| l.name.clone()))
                            .collect(),
                        _ => Vec::new(),
                    },
                    None => Vec::new(),
                };
                match shown {
                    Some((params, _)) => params
                        .into_iter()
                        .enumerate()
                        .map(|(i, text)| {
                            let (label, ty) = match text.split_once(" = ") {
                                Some((l, t)) if ilang_package::is_name(l) => {
                                    (Some(l.to_string()), t.to_string())
                                }
                                _ => (None, text.clone()),
                            };
                            (labels.get(i).cloned().flatten().or(label), Some(ty))
                        })
                        .collect(),
                    None => labels.into_iter().map(|l| (l, None)).collect(),
                }
            }
            _ => Vec::new(),
        }
    }

    /// The exported names of the modules that `m` does not import, whose
    /// name starts with `prefix`, with the edit that imports them.
    fn auto_import_completions(&self, m: ModuleId, prefix: &str) -> Vec<Completion> {
        let p = self.program;
        let imported: Vec<ModuleId> = p.module(m).import_names.iter().map(|(_, t)| *t).collect();
        let visible = self.visible_top_level(m);
        let mut out = Vec::new();
        for (i, _) in p.modules.iter().enumerate() {
            let t = ModuleId(i as u32);
            if t == m || imported.contains(&t) {
                continue;
            }
            let Some(path) = self.import_path(m, t) else {
                continue;
            };
            for mut c in self.module_members(t) {
                if !c.label.starts_with(prefix) || visible.contains(&c.label) {
                    continue;
                }
                let names = if c.kind == CompletionKind::Type {
                    None
                } else {
                    Some(c.label.clone())
                };
                if c.kind == CompletionKind::Type {
                    continue;
                }
                c.edits = vec![self.import_edit(m, &path, names.as_deref())];
                c.detail = Some(format!(
                    "{} (import from {path})",
                    c.detail.unwrap_or_default()
                ));
                out.push(c);
            }
        }
        out
    }

    /// The documentation comment of the top-level name, type, or
    /// constructor `name` of the module with this path.
    pub fn documentation_of(&self, path: &str, name: &str) -> Option<String> {
        let p = self.program;
        let i = p.modules.iter().position(|m| m.path == path)?;
        let t = ModuleId(i as u32);
        match self.member_target(t, name)? {
            Target::Def(d) => self.doc(t, p.def(d).span),
            Target::Type(TypeRef::Custom(ty)) | Target::Ctor(CtorRef { ty, .. }) => {
                let def = p.ty(ty);
                self.doc(def.module, def.span)
            }
            _ => None,
        }
    }

    fn with_doc(&self, mut c: Completion) -> Completion {
        if let Some(path) = &c.defined_in {
            c.documentation = self.documentation_of(path, &c.label);
        }
        c
    }

    /// The signature of the function of the innermost call around `offset`,
    /// with the parameter that the argument there goes to.
    pub fn signature_help(&self, m: ModuleId, offset: u32) -> Option<Signature> {
        let mut call: Option<&Expr> = None;
        visit_module(self.program, m, &mut |n| {
            if let Node::Expr(e) = n
                && let ExprKind::Call { callee, .. } = &e.kind
                && callee.span.end <= offset
                && offset < e.span.end.max(callee.span.end + 1)
                && e.span.start <= offset
                && call.is_none_or(|c| c.span.start <= e.span.start)
            {
                call = Some(e);
            }
        });
        let call = call?;
        let ExprKind::Call { callee, args } = &call.kind else {
            unreachable!("a call")
        };
        let params = self.callee_params(m, callee);
        if params.is_empty() && !matches!(callee.kind, ExprKind::Name(_)) {
            return None;
        }
        let w = self.walk(m);
        let name = w
            .occ
            .iter()
            .filter(|(s, _)| callee.span.start <= s.start && s.end <= callee.span.end)
            .max_by_key(|(s, _)| s.start)
            .map(|(s, _)| self.text(m)[s.start as usize..s.end as usize].to_string())
            .unwrap_or_default();
        let mut label = format!("{name}(");
        let mut out = Vec::new();
        for (i, (l, ty)) in params.iter().enumerate() {
            if i > 0 {
                label.push_str(", ");
            }
            let start = label.len() as u32;
            match (l, ty) {
                (Some(l), Some(t)) => label.push_str(&format!("{l} = {t}")),
                (Some(l), None) => label.push_str(l),
                (None, Some(t)) => label.push_str(t),
                (None, None) => label.push('_'),
            }
            out.push(SignatureParam {
                start,
                end: label.len() as u32,
                label: l.clone(),
            });
        }
        label.push(')');
        let ret = match &callee.kind {
            ExprKind::Name(ValueRef::Ctor(c)) => Some(format!(" -> {}", self.type_head(c.ty))),
            ExprKind::Name(ValueRef::Def(d)) => self
                .def_type(*d)
                .as_deref()
                .and_then(split_fn_type_need)
                .map(need_and_result),
            ExprKind::Name(ValueRef::Local(v)) => {
                let t = match self.def_of_var(m, *v) {
                    Some(d) => self.def_type(d),
                    None => self.local_type(m, *v, callee.span.start),
                };
                t.as_deref()
                    .and_then(split_fn_type_need)
                    .map(need_and_result)
            }
            _ => None,
        };
        if let Some(r) = ret {
            label.push_str(&r);
        }
        // The argument at the position: by its label, else by place.
        let mut active = None;
        let mut position = 0u32;
        for a in args {
            let at = a.value.span.start <= offset && offset <= a.value.span.end
                || a.label.as_ref().is_some_and(|l| contains(l.span, offset));
            if let Some(l) = &a.label {
                if at {
                    active = params
                        .iter()
                        .position(|(x, _)| x.as_deref() == Some(&l.name))
                        .map(|i| i as u32);
                }
            } else if at {
                active = Some(position);
            }
            if a.label.is_none() && a.value.span.end < offset {
                position += 1;
            }
        }
        let active = active.or_else(|| {
            let text = self.text(m);
            let after_last = args.last().map_or(callee.span.end, |a| a.value.span.end);
            let between = text
                .get(after_last as usize..offset as usize)
                .unwrap_or_default();
            let positional = args.iter().filter(|a| a.label.is_none()).count() as u32;
            Some(if args.is_empty() {
                0
            } else if between.contains(',') || (self.is_lisp(m) && between.contains(' ')) {
                positional
            } else {
                positional.saturating_sub(1)
            })
        });
        let documentation = match &callee.kind {
            ExprKind::Name(ValueRef::Def(d)) => self.doc(d.module, self.program.def(*d).span),
            ExprKind::Name(ValueRef::Ctor(c)) => {
                let t = self.program.ty(c.ty);
                self.doc(t.module, t.span)
            }
            ExprKind::Name(ValueRef::Local(v)) => self
                .def_of_var(m, *v)
                .and_then(|d| self.doc(d.module, self.program.def(d).span)),
            _ => None,
        };
        Some(Signature {
            label,
            active: active.filter(|a| (*a as usize) < out.len()),
            params: out,
            documentation,
        })
    }
}

/// The parameters and the rest of a shown function type `(a, b) -> c`.
pub(crate) fn split_fn_type(t: &str) -> Option<(Vec<String>, String)> {
    split_fn_type_need(t).map(|(p, _, r)| (p, r))
}

/// The parameter items, the need (the text after `<-`, if any) and the
/// result of a shown function type `(P) <- S -> R` (syntax three §3.1).
pub(crate) fn split_fn_type_need(t: &str) -> Option<(Vec<String>, Option<String>, String)> {
    let t = t.trim();
    let t = t
        .strip_prefix("{ ")
        .and_then(|x| x.strip_suffix(" }"))
        .unwrap_or(t);
    if !t.starts_with('(') {
        return None;
    }
    let mut depth = 0i32;
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut close = None;
    for (i, c) in t.char_indices() {
        match c {
            '(' | '[' | '{' => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            ')' | ']' | '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            ',' if depth == 1 => {
                parts.push(cur.trim().to_string());
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    let close = close?;
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }
    let mut rest = t[close + 1..].trim_start();
    let mut need = None;
    if let Some(n) = rest.strip_prefix("<-") {
        let n = n.trim_start();
        let mut depth = 0i32;
        let mut end = None;
        for (i, c) in n.char_indices() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ if depth == 0 && n[i..].starts_with(" ->") => {
                    end = Some(i);
                    break;
                }
                _ => {}
            }
        }
        let end = end?;
        need = Some(n[..end].trim().to_string());
        rest = n[end..].trim_start();
    }
    let rest = rest.strip_prefix("->")?.trim().to_string();
    Some((parts, need, rest))
}

/// ` <- S -> R`, or ` -> R` when nothing is needed.
fn need_and_result((_, need, ret): (Vec<String>, Option<String>, String)) -> String {
    match need {
        Some(n) => format!(" <- {n} -> {ret}"),
        None => format!(" -> {ret}"),
    }
}

fn word_start(text: &str, offset: u32) -> u32 {
    let b = text.as_bytes();
    let mut s = (offset as usize).min(b.len());
    while s > 0 && is_word_byte(b[s - 1]) {
        s -= 1;
    }
    s as u32
}

/// The start of the module path being written before `offset`.
fn path_start(text: &str, offset: u32) -> u32 {
    let b = text.as_bytes();
    let mut s = (offset as usize).min(b.len());
    while s > 0 && (is_word_byte(b[s - 1]) || b[s - 1] == b'/') {
        s -= 1;
    }
    s as u32
}

/// The last byte before `offset` that is not a space.
fn prev_byte(text: &str, offset: u32) -> Option<u8> {
    text.as_bytes()[..(offset as usize).min(text.len())]
        .iter()
        .rev()
        .find(|b| !b.is_ascii_whitespace())
        .copied()
}

/// The text of the line before `offset`.
/// The position is an item of the group `( … )` that follows a `<-`:
/// a service pattern or a need.
fn after_need_arrow(text: &str, offset: u32) -> bool {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = (offset as usize).min(bytes.len());
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b')' | b']' | b'}' => depth += 1,
            b'[' | b'{' if depth == 0 => return false,
            b'(' | b'[' | b'{' if depth > 0 => depth -= 1,
            b'(' => return text[..i].trim_end().ends_with("<-"),
            _ => {}
        }
    }
    false
}

fn line_before(text: &str, offset: u32) -> &str {
    let o = (offset as usize).min(text.len());
    let start = text[..o].rfind(['\n', '\r']).map_or(0, |i| i + 1);
    &text[start..o]
}

/// Whether the cursor is inside the parentheses directly following a
/// syntax-three branch join or match pipe. Tokens keep comments and strings
/// from inventing a head, and the delimiter stack excludes branch bodies.
fn in_source_branch_head(text: &str, offset: u32) -> bool {
    use ilang_syntax::token::TokenKind as T;
    let lexed = ilang_syntax::lexer::lex3(text);
    let mut stack = Vec::new();
    let mut previous = None;
    for token in &lexed.tokens {
        if token.span.start >= offset { break; }
        match token.kind {
            T::LParen => stack.push((T::LParen, matches!(previous, Some(T::Pipe | T::PipeGt)))),
            T::LBrace => stack.push((T::LBrace, false)),
            T::LBracket => stack.push((T::LBracket, false)),
            T::RParen | T::RBrace | T::RBracket => { stack.pop(); },
            _ => {}
        }
        previous = Some(token.kind.clone());
    }
    stack.iter().rev().take_while(|(kind, _)| !matches!(kind, T::LBrace)).any(|(_, head)| *head)
}
