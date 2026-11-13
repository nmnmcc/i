//! Navigation in the resolved tree: the type of a name, what makes a type,
//! the macro that a call expands, the files that imports name, and the
//! shape of a module (folding, selection, symbols).

use super::*;

/// A node of the HIR, for the queries that look at the shape of a module.
#[derive(Clone, Copy)]
pub(crate) enum Node<'h> {
    Expr(&'h Expr),
    Pattern(&'h Pattern),
    Type(&'h Type),
    Block(&'h Block),
    Clause(&'h Clause),
    Binding(&'h Binding),
    Stmt(Span),
}

impl Node<'_> {
    pub fn span(&self) -> Span {
        match self {
            Node::Expr(e) => e.span,
            Node::Pattern(p) => p.span,
            Node::Type(t) => t.span,
            Node::Block(b) => b.span,
            Node::Clause(c) => c.span,
            Node::Binding(b) => b.span,
            Node::Stmt(s) => *s,
        }
    }
}

/// Calls `f` with every node of the bindings and types of a module, each
/// before the nodes in it.
pub(crate) fn visit_module<'h>(p: &'h Program, m: ModuleId, f: &mut dyn FnMut(Node<'h>)) {
    let module = p.module(m);
    for b in &module.bindings {
        visit_binding(&b.binding, f);
    }
    for d in &module.defs {
        if let DefKind::External { ty, .. } = &d.kind {
            visit_type(ty, f);
        }
    }
    for &t in &module.types {
        match &p.ty(t).body {
            TypeBody::Alias(ty) => visit_type(ty, f),
            TypeBody::Custom(cs) => {
                for c in cs {
                    for field in &c.fields {
                        visit_type(&field.ty, f);
                    }
                }
            }
        }
    }
}

fn visit_binding<'h>(b: &'h Binding, f: &mut dyn FnMut(Node<'h>)) {
    f(Node::Binding(b));
    visit_pattern(&b.pattern, f);
    visit_expr(&b.value, f);
}

pub(crate) fn visit_pattern<'h>(p: &'h Pattern, f: &mut dyn FnMut(Node<'h>)) {
    f(Node::Pattern(p));
    match &p.kind {
        PatternKind::Quote { captures, .. } => {
            captures.iter().for_each(|(_, c)| visit_pattern(c, f))
        }
        PatternKind::Typed { ty, .. } => visit_type(ty, f),
        PatternKind::Named { pattern, .. } => visit_pattern(pattern, f),
        PatternKind::Constructor { args, .. }
        | PatternKind::Tuple(args)
        | PatternKind::Error(args) => args.iter().for_each(|a| visit_pattern(a, f)),
        PatternKind::List { items, .. } => items.iter().for_each(|a| visit_pattern(a, f)),
        PatternKind::BitArray(segs) => segs.iter().for_each(|s| visit_pattern(&s.value, f)),
        PatternKind::Var(_)
        | PatternKind::Discard
        | PatternKind::Int { .. }
        | PatternKind::Float { .. }
        | PatternKind::String(_)
        | PatternKind::Prefix { .. } => {}
    }
}

pub(crate) fn visit_type<'h>(t: &'h Type, f: &mut dyn FnMut(Node<'h>)) {
    f(Node::Type(t));
    match &t.kind {
        TypeKind::Named { args, .. } => args.iter().for_each(|a| visit_type(a, f)),
        TypeKind::Tuple(ts) | TypeKind::Union(ts) | TypeKind::Inter(ts) | TypeKind::Diff(ts) => {
            ts.iter().for_each(|a| visit_type(a, f))
        }
        TypeKind::Record { base, fields, .. } => {
            if let Some(b) = base {
                visit_type(b, f);
            }
            fields.iter().for_each(|(_, t)| visit_type(t, f));
        }
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => {
            params.iter().for_each(|p| visit_type(&p.ty, f));
            visit_type(ret, f);
            err.iter().chain(env.iter()).for_each(|t| visit_type(t, f));
        }
        TypeKind::Lit(_)
        | TypeKind::Of { .. }
        | TypeKind::Var(_)
        | TypeKind::Hole
        | TypeKind::Error => {}
    }
}

fn visit_block<'h>(b: &'h Block, f: &mut dyn FnMut(Node<'h>)) {
    f(Node::Block(b));
    for s in &b.stmts {
        match s {
            Stmt::Bind(b) => visit_binding(b, f),
            Stmt::Env { ty, value, span } => {
                f(Node::Stmt(*span));
                visit_type(ty, f);
                visit_expr(value, f);
            }
            Stmt::Expr(e) => visit_expr(e, f),
        }
    }
}

pub(crate) fn visit_expr<'h>(e: &'h Expr, f: &mut dyn FnMut(Node<'h>)) {
    f(Node::Expr(e));
    match &e.kind {
        ExprKind::Tuple(xs) => xs.iter().for_each(|x| visit_expr(x, f)),
        ExprKind::Record { base, fields } => {
            if let Some(b) = base {
                visit_expr(b, f);
            }
            fields.iter().for_each(|(_, x)| visit_expr(x, f));
        }
        ExprKind::List { items, rest } => {
            items.iter().for_each(|x| visit_expr(x, f));
            if let Some(r) = rest {
                visit_expr(r, f);
            }
        }
        ExprKind::BitArray(segs) => segs.iter().for_each(|s| visit_expr(&s.value, f)),
        ExprKind::Block(b) => visit_block(b, f),
        ExprKind::Fn(func) => {
            for p in &func.params {
                match p {
                    Param::Pattern { pattern, .. } => visit_pattern(pattern, f),
                    Param::Env { ty, .. } => visit_type(ty, f),
                }
            }
            if let Some(r) = &func.ret {
                visit_type(&r.value, f);
                r.err
                    .iter()
                    .chain(r.env.iter())
                    .for_each(|t| visit_type(t, f));
            }
            visit_block(&func.body, f);
        }
        ExprKind::Case { subject, clauses } => {
            visit_expr(subject, f);
            for c in clauses {
                f(Node::Clause(c));
                c.patterns.iter().for_each(|p| visit_pattern(p, f));
                if let Some(g) = &c.guard {
                    visit_expr(g, f);
                }
                visit_expr(&c.body, f);
            }
        }
        ExprKind::Call { callee, args } => {
            visit_expr(callee, f);
            args.iter().for_each(|a| visit_expr(&a.value, f));
        }
        ExprKind::Field { target, .. } | ExprKind::TupleIndex { target, .. } => {
            visit_expr(target, f)
        }
        ExprKind::RecordUpdate { base, fields, .. } => {
            visit_expr(base, f);
            fields.iter().for_each(|(_, x)| visit_expr(x, f));
        }
        ExprKind::Unary { expr, .. } => visit_expr(expr, f),
        ExprKind::Binary { lhs, rhs, .. } => {
            visit_expr(lhs, f);
            visit_expr(rhs, f);
        }
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::String(_)
        | ExprKind::Name(_)
        | ExprKind::Error => {}
    }
}

/// A range of lines that an editor may fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Folding {
    pub span: Span,
    /// The imports or the exports.
    pub imports: bool,
}

/// A call of a macro in the written text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroCall {
    /// The name of the macro in the call.
    pub name: Span,
    /// The whole call.
    pub call: Span,
    pub macro_name: String,
    /// The name of the macro in its definition.
    pub definition: Location,
    pub doc: Option<String>,
    /// The path of the module of the macro.
    pub module: String,
}

/// A file that a place names: the module of an import path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentLink {
    pub span: Span,
    pub target: ModuleId,
    pub source: SourceId,
}

/// A definition of a package, for a search of the whole workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    /// The path of its module.
    pub container: String,
}

impl Query<'_> {
    /// The types of the name at `offset`: of a value, the custom types of
    /// its known type; of a constructor or a field, the type it makes or
    /// holds; of a type, itself.
    pub fn type_definition_at(&self, m: ModuleId, offset: u32) -> Vec<Location> {
        let Some((owner, _, target)) = self.target_at_offset(m, offset) else {
            return Vec::new();
        };
        let mut types: Vec<TypeRef> = Vec::new();
        match &target {
            Target::Local(v) => types = self.var_types(*v),
            Target::Def(d) => match &self.program.def(*d).kind {
                DefKind::Binding { var, .. } => types = self.var_types(*var),
                DefKind::External { ty, .. } => named_types_deep(ty, &mut types),
            },
            Target::Ctor(c) => types.push(TypeRef::Custom(c.ty)),
            Target::Field(fields) => {
                for (c, i) in fields {
                    if let Some(f) = self.ctor_def(*c).and_then(|c| c.fields.get(*i as usize)) {
                        named_types(&f.ty, &mut types);
                    }
                }
            }
            Target::Type(t) => types.push(*t),
            Target::Literal(_) | Target::TyVar(_) | Target::Module(_) => {}
        }
        let mut out = Vec::new();
        for t in types {
            if let Some(l) = self.definition(owner, &Target::Type(t))
                && !out.contains(&l)
            {
                out.push(l);
            }
        }
        out
    }

    /// What makes the values of the type at `offset`: its constructors,
    /// and for a service type the environment statements that give it
    /// (§9.2). Of a constructor or a value, its definition.
    pub fn implementations_at(&self, m: ModuleId, offset: u32) -> Vec<Location> {
        let Some((owner, _, target)) = self.target_at_offset(m, offset) else {
            return Vec::new();
        };
        let Target::Type(TypeRef::Custom(t)) = target else {
            return self.definition(owner, &target).into_iter().collect();
        };
        let p = self.program;
        let mut out = Vec::new();
        // A name for a union (syntax three §2.8): the constructors of its
        // members, in the order written.
        let mut types = Vec::new();
        union_members(p, t, &mut types);
        for ty in types {
            if let TypeBody::Custom(cs) = &p.ty(ty).body {
                for (i, _) in cs.iter().enumerate() {
                    if let Some(l) = self.definition(
                        owner,
                        &Target::Ctor(CtorRef {
                            ty,
                            index: i as u32,
                        }),
                    ) {
                        out.push(l);
                    }
                }
            }
        }
        for m in self.own_modules() {
            let mut found = Vec::new();
            visit_module(p, m, &mut |n| {
                if let Node::Block(b) = n {
                    for s in &b.stmts {
                        if let Stmt::Env { ty, span, .. } = s
                            && matches!(&ty.kind, TypeKind::Named { ty: TypeRef::Custom(x), .. } if *x == t)
                        {
                            found.push(*span);
                        }
                    }
                }
            });
            for span in found {
                if let Some(l) = self.written_place(m, span) {
                    out.push(l);
                }
            }
        }
        out
    }

    /// The macro call whose name is at `offset` in the written text of a
    /// module, and the macro it calls. Macro calls are not in the HIR (they
    /// are expanded), so the call is found in the written tree and its name
    /// by the imports of the module.
    pub fn macro_call_at(&self, m: ModuleId, offset: u32) -> Option<MacroCall> {
        let ast = self.ast(m);
        let mut found = None;
        let mut visit = |e: &ast::Expr| {
            if let ast::ExprKind::Call { callee, .. } = &e.kind
                && callee.span.start <= offset
                && offset <= callee.span.end
                && let Some(c) = self.resolve_macro(m, callee, e.span)
            {
                found = Some(c);
            }
        };
        for item in &ast.items {
            match &item.kind {
                ast::ItemKind::Binding(b) => ast_exprs(&b.value, &mut visit),
                ast::ItemKind::MacroItem { args, .. } => {
                    args.iter().for_each(|a| ast_exprs(&a.value, &mut visit))
                }
                _ => {}
            }
        }
        found
    }

    /// The macro that a written callee `f`, `m.f` or `(. m f)` names.
    fn resolve_macro(&self, m: ModuleId, callee: &ast::Expr, call: Span) -> Option<MacroCall> {
        let (module, name) = match &callee.kind {
            ast::ExprKind::Field { target, name } => match &target.kind {
                ast::ExprKind::Name {
                    module: None,
                    name: base,
                } => (Some(self.import_module(m, &base.name)?), name),
                _ => return None,
            },
            ast::ExprKind::Name { module: None, name } => (None, name),
            _ => return None,
        };
        let t = match module {
            Some(t) => t,
            None => self
                .ast(m)
                .imports
                .iter()
                .find_map(|i| {
                    if !i.names.iter().any(|n| n.name == name.name) {
                        return None;
                    }
                    let alias = i.alias.as_ref().or(i.path.last())?;
                    self.import_module(m, &alias.name)
                })
                .unwrap_or(m),
        };
        let (span, doc) = self.macro_definition(t, &name.name)?;
        Some(MacroCall {
            name: name.span,
            call,
            macro_name: name.name.clone(),
            definition: Location {
                module: t,
                source: self.source_of(t),
                span,
            },
            doc,
            module: self.program.module(t).path.clone(),
        })
    }

    /// The name of the binding of a macro in a module: an item with the
    /// attribute `@macro`, and its documentation.
    fn macro_definition(&self, t: ModuleId, name: &str) -> Option<(Span, Option<String>)> {
        let ast = self.ast(t);
        for item in &ast.items {
            if !item
                .attributes
                .iter()
                .any(|a| a.module.is_none() && a.name.name == "macro")
            {
                continue;
            }
            let ast::ItemKind::Binding(b) = &item.kind else {
                continue;
            };
            let binder = match &b.pattern.kind {
                ast::PatternKind::Var(n) if n == name => b.pattern.span,
                ast::PatternKind::Typed { binder, .. } if binder.name == name => binder.span,
                _ => continue,
            };
            let text = self.written_text(t);
            let span = find_word(text, binder, name, false).unwrap_or(binder);
            let doc = (!item.doc.is_empty()).then(|| {
                item.doc
                    .iter()
                    .map(|l| l.strip_prefix(' ').unwrap_or(l))
                    .collect::<Vec<_>>()
                    .join("\n")
            });
            return Some((span, doc));
        }
        None
    }

    /// The code that the macro call with the span `call` was expanded to,
    /// as the expansion wrote it.
    pub fn expansion_of(&self, m: ModuleId, call: Span) -> Option<String> {
        let p = self.program;
        let module = p.module(m);
        let scoped = module.scoped.as_ref()?;
        let text = self.text(m);
        let mut best: Option<Span> = None;
        visit_module(p, m, &mut |n| {
            let span = n.span();
            if !scoped.is_generated(span) || span.end as usize > text.len() {
                return;
            }
            let Some(origin) = scoped.origin(span) else {
                return;
            };
            let from_call = origin
                .call_chain
                .first()
                .is_some_and(|c| c.syntax.span() == call)
                || origin.call_site.span() == call;
            if from_call && best.is_none_or(|b| span.end - span.start > b.end - b.start) {
                best = Some(span);
            }
        });
        best.map(|s| text[s.start as usize..s.end as usize].to_string())
    }

    /// The import paths of a module, with the module that each names.
    pub fn document_links(&self, m: ModuleId) -> Vec<DocumentLink> {
        let ast = self.ast(m);
        let mut out = Vec::new();
        for item in &ast.imports {
            let alias = item.alias.as_ref().or(item.path.last());
            let (Some(first), Some(last)) = (item.path.first(), item.path.last()) else {
                continue;
            };
            if let Some(t) = alias.and_then(|a| self.import_module(m, &a.name)) {
                out.push(DocumentLink {
                    span: first.span.to(last.span),
                    target: t,
                    source: self.source_of(t),
                });
            }
        }
        out
    }

    /// The parts of a module that may be folded: the imports and exports,
    /// the items, and the blocks, functions and clauses of more than one
    /// line.
    pub fn folding_ranges(&self, m: ModuleId) -> Vec<Folding> {
        let ast = self.ast(m);
        let text = self.written_text(m);
        let lines = |s: Span| {
            let end = (s.end as usize).min(text.len());
            let start = (s.start as usize).min(end);
            text[start..end].contains('\n')
        };
        let mut out = Vec::new();
        for s in [ast.imports_span, ast.exports_span].into_iter().flatten() {
            if lines(s) {
                out.push(Folding {
                    span: s,
                    imports: true,
                });
            }
        }
        for item in &ast.items {
            if lines(item.span) {
                out.push(Folding {
                    span: item.span,
                    imports: false,
                });
            }
        }
        visit_module(self.program, m, &mut |n| {
            let span = match n {
                Node::Block(b) => b.span,
                Node::Clause(c) => c.span,
                Node::Expr(e)
                    if matches!(
                        e.kind,
                        ExprKind::Case { .. } | ExprKind::Call { .. } | ExprKind::List { .. }
                    ) =>
                {
                    e.span
                }
                _ => return,
            };
            if span.end as usize <= text.len() && lines(span) {
                out.push(Folding {
                    span,
                    imports: false,
                });
            }
        });
        out.sort_by_key(|f| (f.span.start, std::cmp::Reverse(f.span.end)));
        out.dedup_by_key(|f| f.span);
        out
    }

    /// The nested spans around `offset`, the smallest first: names,
    /// expressions, statements, blocks, the item, the file.
    pub fn selection_ranges(&self, m: ModuleId, offset: u32) -> Vec<Span> {
        let text = self.written_text(m);
        let mut spans = Vec::new();
        visit_module(self.program, m, &mut |n| {
            let s = n.span();
            if s.start <= offset
                && offset <= s.end
                && s.start < s.end
                && s.end as usize <= text.len()
            {
                spans.push(s);
            }
        });
        if let Some(w) = word_at(text, offset) {
            spans.push(w);
        }
        let ast = self.ast(m);
        for s in ast
            .items
            .iter()
            .map(|i| i.span)
            .chain(ast.imports.iter().map(|i| i.span))
            .chain(ast.imports_span)
            .chain(ast.exports_span)
        {
            if s.start <= offset && offset <= s.end {
                spans.push(s);
            }
        }
        spans.push(Span::new(0, text.len() as u32));
        spans.sort_by_key(|s| (s.end - s.start, std::cmp::Reverse(s.start)));
        spans.dedup();
        // Each span must hold the one before it.
        let mut out: Vec<Span> = Vec::new();
        for s in spans {
            if out
                .last()
                .is_none_or(|l| s.start <= l.start && l.end <= s.end && s != *l)
            {
                out.push(s);
            }
        }
        out
    }

    /// The definitions of the modules of the package whose name holds
    /// `query`, ignoring case (all of them for an empty query).
    pub fn workspace_symbols(&self, query: &str) -> Vec<WorkspaceSymbol> {
        let query = query.to_lowercase();
        let mut out = Vec::new();
        for m in self.own_modules() {
            let source = self.source_of(m);
            let path = self.program.module(m).path.clone();
            let mut add = |s: &Symbol| {
                if fuzzy(&s.name.to_lowercase(), &query) {
                    out.push(WorkspaceSymbol {
                        name: s.name.clone(),
                        kind: s.kind,
                        location: Location {
                            module: m,
                            source,
                            span: s.name_span,
                        },
                        container: path.clone(),
                    });
                }
            };
            for s in self.symbols(m) {
                add(&s);
                s.children.iter().for_each(&mut add);
            }
        }
        out
    }
}

/// The letters of `query` are in `name` in the same order.
fn fuzzy(name: &str, query: &str) -> bool {
    let mut chars = name.chars();
    query.chars().all(|q| chars.any(|c| c == q))
}

/// The named types of a type, also in the parameters and results of
/// function types.
fn named_types_deep(t: &Type, out: &mut Vec<TypeRef>) {
    walk_type(t, &mut |x| {
        if let TypeKind::Named { ty, .. } = &x.kind {
            out.push(*ty);
        }
    });
}

/// Calls `f` with every expression of a written expression.
pub(crate) fn ast_exprs(e: &ast::Expr, f: &mut dyn FnMut(&ast::Expr)) {
    use ast::ExprKind as K;
    f(e);
    match &e.kind {
        K::Tuple(xs) => xs.iter().for_each(|x| ast_exprs(x, f)),
        K::Record { base, fields } => {
            if let Some(b) = base {
                ast_exprs(b, f);
            }
            fields.iter().for_each(|(_, x)| ast_exprs(x, f));
        }
        K::List { items, rest } => {
            items.iter().for_each(|x| ast_exprs(x, f));
            if let Some(r) = rest {
                ast_exprs(r, f);
            }
        }
        K::BitArray(segs) => segs.iter().for_each(|s| ast_exprs(&s.value, f)),
        K::Block(b) => ast_block(b, f),
        K::Fn(func) => ast_block(&func.body, f),
        K::Case { subject, clauses } => {
            ast_exprs(subject, f);
            ast_clauses(clauses, f);
        }
        K::Clauses(clauses) => ast_clauses(clauses, f),
        K::Call { callee, args } => {
            ast_exprs(callee, f);
            args.iter().for_each(|a| ast_exprs(&a.value, f));
        }
        K::Field { target, .. } | K::TupleIndex { target, .. } => ast_exprs(target, f),
        K::RecordUpdate {
            constructor,
            base,
            fields,
        } => {
            ast_exprs(constructor, f);
            ast_exprs(base, f);
            fields.iter().for_each(|(_, x)| ast_exprs(x, f));
        }
        K::Unary { expr, .. } => ast_exprs(expr, f),
        K::Binary { lhs, rhs, .. } => {
            ast_exprs(lhs, f);
            ast_exprs(rhs, f);
        }
        K::Quote(_) | K::Int(_) | K::Float(_) | K::String(_) | K::Name { .. } => {}
    }
}

fn ast_clauses(clauses: &[ast::Clause], f: &mut dyn FnMut(&ast::Expr)) {
    for c in clauses {
        if let Some(g) = &c.guard {
            ast_exprs(g, f);
        }
        ast_exprs(&c.body, f);
    }
}

fn ast_block(b: &ast::Block, f: &mut dyn FnMut(&ast::Expr)) {
    for s in &b.stmts {
        match s {
            ast::Stmt::Bind(b) => ast_exprs(&b.value, f),
            ast::Stmt::Env { value, .. } => ast_exprs(value, f),
            ast::Stmt::Expr(e) => ast_exprs(e, f),
        }
    }
}

/// `t` itself, or the custom types that the alias `t` names as the members
/// of its union, through other aliases.
fn union_members(p: &Program, t: TypeId, out: &mut Vec<TypeId>) {
    fn walk(p: &Program, ty: &Type, out: &mut Vec<TypeId>, depth: usize) {
        match &ty.kind {
            TypeKind::Union(ts) => {
                for x in ts {
                    walk(p, x, out, depth);
                }
            }
            TypeKind::Named {
                ty: TypeRef::Custom(id),
                ..
            } => match &p.ty(*id).body {
                TypeBody::Alias(inner) if depth < 16 => walk(p, inner, out, depth + 1),
                TypeBody::Custom(_) if !out.contains(id) => out.push(*id),
                _ => {}
            },
            _ => {}
        }
    }
    match &p.ty(t).body {
        TypeBody::Alias(inner) => walk(p, inner, out, 0),
        TypeBody::Custom(_) => out.push(t),
    }
}
