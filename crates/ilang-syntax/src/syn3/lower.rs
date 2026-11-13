//! The transition front of syntax three (docs/design/syntax-3.md §7.2,
//! stage 8): a syntax-three tree is lowered to the current tree, which the
//! resolver reads. This is the reverse of `convert`, form by form.
//!
//! The checker of this stage is today's, so the forms that only the later
//! stages can type are compile errors here, each naming its stage: a chain
//! with several shape groups (stage 9), `..name: N` in a service pattern
//! (stage E6), a key on a parameter (stage 9), keys in data patterns
//! (stage 6).
//!
//! Three things have no spelling in today's syntax and are passed on as
//! follows:
//! - `e <- (s1, s2)` becomes the block `{ ! _ = s1  ! _ = s2  e }`: an
//!   environment statement whose type is `_` takes its key from the known
//!   type of its value (the checker reads it so).
//! - A written `-> R` of a function literal leaves the needs to inference:
//!   it becomes `-> R ^ _ ! _`, which the checker reads as "inferred".
//! - `<- N` or `<- (..N, …)` with no `Fail` entry in the signature of a
//!   binding with the type parameter `N` carries the errors in `N`: it
//!   becomes `^ N%E ! N`, with `N%E` one more type parameter of the binding.
//!
//! One-constructor types: the checker of this stage has no unions of data
//! types that stand for a sum, so `C1 = (…)`, `C2 = (…)`, `T = C1 | C2`
//! written one after the other are read as today's `T { C1(…) C2(…) }`; an
//! export of `C1` alone is then part of the export of `T`.

use super::ast as n;
use crate::ast as o;
use crate::span::Span;
use crate::{Code, Diagnostic};
use std::collections::{HashMap, HashSet};

/// Lowers a module of syntax three to the current tree.
pub fn lower(m: &n::Module) -> (o::Module, Vec<Diagnostic>) {
    lower_surface(m, false)
}

/// Lowers a module read from the Lisp surface. There a match is today's
/// pipe into a clause block, as the Lisp rows of today read it.
pub fn lower_lisp(m: &n::Module) -> (o::Module, Vec<Diagnostic>) {
    lower_surface(m, true)
}

fn lower_surface(m: &n::Module, lisp: bool) -> (o::Module, Vec<Diagnostic>) {
    lower_expanded(m, lisp, None)
}

/// Step 2d boundary: written Lisp keeps its transition spelling, while
/// expansion results use the same expression reader as standalone results.
/// The source map gives the start of the virtual expansion suffix.
pub fn lower_expanded(
    m: &n::Module,
    lisp: bool,
    generated_from: Option<u32>,
) -> (o::Module, Vec<Diagnostic>) {
    let mut cx = Cx {
        transition: Default::default(),
        fragment: None,
        lisp,
        generated_from,
        errors: Vec::new(),
        rows: Vec::new(),
        fresh: 0,
        group: false,
        hidden: Vec::new(),
        // `core` is not the name of another module than `i/core`.
        core_free: !m.imports.iter().any(|i| {
            let named = match &i.alias {
                Some(a) => a.name == "core",
                None => i.names.is_empty() && i.path.last().is_some_and(|p| p.name == "core"),
            };
            let library = matches!(i.path.as_slice(), [a, b] if a.name == "i" && b.name == "core");
            named && !library
        }),
    };
    let module = cx.module(m);
    (module, cx.errors)
}

pub use super::facts::Transition;
impl Transition {
    fn binding(&mut self, binding: &o::Binding) {
        self.parameters.insert(
            (binding.span, binding.pattern.span),
            binding
                .type_params
                .iter()
                .map(|p| (p.name.clone(), p.span))
                .collect(),
        );
    }
    fn block(&mut self, block: &o::Block) {
        for statement in &block.stmts {
            if let o::Stmt::Bind(binding) = statement {
                self.binding(binding);
            }
        }
    }
    fn record<T: crate::walk::Walk>(&mut self, expression: &mut T) {
        struct Bindings<'a>(&'a mut Transition);
        impl crate::walk::Visitor for Bindings<'_> {
            fn expr(&mut self, expression: &mut o::Expr, enter: bool) {
                if !enter {
                    return;
                }
                match &expression.kind {
                    o::ExprKind::Quote(datum) => self.0.quoted(datum),
                    o::ExprKind::Block(block) => self.0.block(block),
                    o::ExprKind::Fn(function) => self.0.block(&function.body),
                    _ => {}
                }
            }
        }
        crate::walk::Walk::walk(expression, &mut Bindings(self));
    }
    fn quoted(&mut self, datum: &crate::lisp::Datum) {
        use crate::lisp::DatumKind;
        let mut work = vec![datum];
        while let Some(datum) = work.pop() {
            match &datum.kind {
                DatumKind::Unquote(inner) | DatumKind::Splice(inner) => {
                    self.expressions.insert(inner.span);
                    if let Ok(native) = super::lisp::expr_from_datum(inner)
                        && let Ok(mut lowered) = lower_expr(&native)
                    {
                        self.record_expression(&mut lowered);
                    }
                }
                DatumKind::Quote(inner) => work.push(inner),
                DatumKind::Round(children)
                | DatumKind::Square(children)
                | DatumKind::Curly(children)
                | DatumKind::Bits(children) => work.extend(children),
                _ => {}
            }
        }
    }
    pub fn record_expression(&mut self, expression: &mut o::Expr) {
        self.record(expression);
    }
    pub fn record_pattern(&mut self, pattern: &mut o::Pattern) {
        self.record(pattern);
    }
    pub fn record_items(&mut self, items: &mut [o::Item]) {
        self.complete = true;
        for item in items {
            match &mut item.kind {
                o::ItemKind::Binding(binding) => {
                    self.binding(binding);
                    self.record_pattern(&mut binding.pattern);
                    self.record_expression(&mut binding.value);
                }
                o::ItemKind::TypeDef(def) => {
                    if let o::TypeBody::Custom(constructors) = &def.body {
                        let members: Vec<_> = constructors
                            .iter()
                            .map(|c| c.span)
                            .filter(|span| *span != item.span)
                            .collect();
                        if !members.is_empty() {
                            self.sums.insert(item.span, members);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Lower after expansion with the original binding and fragment boundaries.
pub fn lower_with_transition(
    m: &n::Module,
    lisp: bool,
    generated_from: Option<u32>,
    written: &Transition,
    generated: Option<&Transition>,
) -> (o::Module, Vec<Diagnostic>) {
    let mut transition = written.clone();
    if let Some(generated) = generated {
        transition.extend(generated);
    }
    let mut cx = Cx::plain();
    cx.lisp = lisp;
    cx.generated_from = generated_from;
    cx.core_free = !m.imports.iter().any(|i| {
        let named = match &i.alias {
            Some(a) => a.name == "core",
            None => i.names.is_empty() && i.path.last().is_some_and(|p| p.name == "core"),
        };
        let library = matches!(i.path.as_slice(), [a, b] if a.name == "i" && b.name == "core");
        named && !library
    });
    cx.transition = std::sync::Arc::new(transition);
    let module = cx.module(m);
    (module, cx.errors)
}

/// Lowers one expression (macro results, tests).
pub fn lower_expr(e: &n::Expr) -> Result<o::Expr, Diagnostic> {
    let mut cx = Cx::plain();
    let out = cx.expr(e);
    cx.finish(out)
}

/// Standalone typed output, including fragments expanded inside it.
pub fn lower_expr_with_transition(
    e: &n::Expr,
    transition: Option<&Transition>,
) -> Result<o::Expr, Diagnostic> {
    let mut cx = Cx::plain();
    if let Some(transition) = transition {
        cx.transition = std::sync::Arc::new(transition.clone());
    }
    let out = cx.expr(e);
    cx.finish(out)
}

pub fn lower_type(t: &n::Type) -> Result<o::TypeExpr, Diagnostic> {
    let mut cx = Cx::plain();
    let out = cx.ty(t);
    cx.finish(out)
}

pub fn lower_pattern(p: &n::Pattern) -> Result<o::Pattern, Diagnostic> {
    let mut cx = Cx::plain();
    let out = cx.pattern(p);
    cx.finish(out)
}

/// Lowers items that stand alone (macro results).
pub fn lower_items(items: &[n::Item]) -> Result<Vec<o::Item>, Diagnostic> {
    let mut cx = Cx::plain();
    let (out, _) = cx.items(items);
    cx.finish(out)
}

struct Row {
    name: String,
    /// The signature of the binding writes no `Fail` beside this row
    /// variable, so the row variable carries the errors.
    carries: bool,
    used: bool,
}

/// The row variables that a type writes beside a `Fail` entry.
fn fail_rows(t: &n::Type, out: &mut HashSet<String>) {
    let each =
        |ts: &[n::Type], out: &mut HashSet<String>| ts.iter().for_each(|t| fail_rows(t, out));
    match &t.kind {
        n::TypeKind::Named { args, .. } | n::TypeKind::Call { args, .. } => each(args, out),
        n::TypeKind::Union(ts) | n::TypeKind::Inter(ts) | n::TypeKind::Diff(ts) => each(ts, out),
        n::TypeKind::Data(items) => {
            for i in items {
                match i {
                    n::TItem::Pos(t) | n::TItem::Label(_, t) | n::TItem::Spread(t) => {
                        fail_rows(t, out)
                    }
                }
            }
        }
        n::TypeKind::Fn(f) => {
            fail_rows(&f.param, out);
            fail_rows(&f.ret, out);
            if let Some(s) = &f.services {
                fail_rows(s, out);
                if let n::TypeKind::Data(items) = &s.kind {
                    let fail = items.iter().any(|i| {
                        matches!(i, n::TItem::Pos(t)
                            if matches!(&t.kind, n::TypeKind::Named { module: None, name, .. } if name.name == "Fail"))
                    });
                    if fail {
                        for i in items {
                            if let n::TItem::Spread(t) = i
                                && let n::TypeKind::Named { name, .. } = &t.kind
                            {
                                out.insert(name.name.clone());
                            }
                        }
                    }
                }
            }
        }
        n::TypeKind::Lit(_) | n::TypeKind::ValueOf { .. } | n::TypeKind::Hole => {}
    }
}

fn pattern_fail_rows(p: &n::Pattern, out: &mut HashSet<String>) {
    match &p.kind {
        n::PatternKind::Typed { ty, .. } => fail_rows(ty, out),
        n::PatternKind::Ctor {
            args: Some(items), ..
        }
        | n::PatternKind::Data(items) => {
            for i in items {
                match i {
                    n::DPItem::Pos(p) | n::DPItem::Key(_, p) => pattern_fail_rows(p, out),
                    n::DPItem::Rest(_) => {}
                }
            }
        }
        n::PatternKind::List { items, .. } | n::PatternKind::And(items) => {
            items.iter().for_each(|p| pattern_fail_rows(p, out))
        }
        _ => {}
    }
}

struct Cx {
    transition: std::sync::Arc<Transition>,
    fragment: Option<(u8, Span)>,
    lisp: bool,
    generated_from: Option<u32>,
    errors: Vec<Diagnostic>,
    /// The type parameters of the bindings around, for the error parameter
    /// that a row variable carries.
    rows: Vec<Row>,
    fresh: usize,
    /// The module imports nothing under the name `core`, so `core.never`
    /// is the empty match (§2.13).
    core_free: bool,
    /// The chain being lowered is one shape group of a multi chain: its
    /// branches have one shape and may have keys.
    group: bool,
    /// The hidden group bindings of the multi chains bound at the top
    /// level, by the name of the chain.
    hidden: Vec<(String, String)>,
}

/// The shape of a branch: its number of positions and its sorted labels.
fn shape_of(f: &n::Function) -> (usize, Vec<&str>) {
    let mut labels: Vec<&str> = f
        .params
        .iter()
        .filter_map(|p| p.key.as_ref())
        .map(|k| k.name.as_str())
        .collect();
    labels.sort();
    (f.params.len() - labels.len(), labels)
}

/// The branches of a chain by shape, in the order of the first branch of
/// each group (§2.5).
fn shape_groups(fs: &[n::Function]) -> Vec<Vec<n::Function>> {
    let mut groups: Vec<Vec<n::Function>> = Vec::new();
    for f in fs {
        match groups.iter_mut().find(|g| shape_of(&g[0]) == shape_of(f)) {
            Some(g) => g.push(f.clone()),
            None => groups.push(vec![f.clone()]),
        }
    }
    groups
}

/// A chain with several shape groups, or with a label in its one shape.
fn is_multi(fs: &[n::Function]) -> bool {
    fs.iter()
        .any(|f| f.params.iter().any(|p| p.key.is_some()) || shape_of(f) != shape_of(&fs[0]))
}

/// The parameter items of a branch in canonical order: the positions, then
/// the labels in UTF-8 byte order.
fn canonical(f: &n::Function) -> Vec<&n::PItem> {
    let mut items: Vec<&n::PItem> = f.params.iter().filter(|p| p.key.is_none()).collect();
    let mut keyed: Vec<&n::PItem> = f.params.iter().filter(|p| p.key.is_some()).collect();
    keyed.sort_by(|a, b| {
        a.key
            .as_ref()
            .unwrap()
            .name
            .cmp(&b.key.as_ref().unwrap().name)
    });
    items.extend(keyed);
    items
}

/// The label under which a shape is written in the transition tree: the
/// number of positions and then each label after a comma.
pub fn shape_text(positions: usize, labels: &[&str]) -> String {
    let mut text = positions.to_string();
    for l in labels {
        text.push(',');
        text.push_str(l);
    }
    text
}

/// The label of the one parameter of a multi-shape function type in the
/// transition tree.
pub const DATA: &str = "%data";

/// The callee of the transition form of a multi chain (see `multi_expr`).
pub const MULTI: &str = "%multi";

/// The hidden binding of group `index` of the chain bound to `name`.
pub fn group_name(name: &str, index: usize) -> String {
    format!("{name}%g{index}")
}

fn id(i: &n::Ident) -> o::Ident {
    o::Ident {
        name: i.name.clone(),
        span: i.span.0,
    }
}

fn opt_id(i: &Option<n::Ident>) -> Option<o::Ident> {
    i.as_ref().map(id)
}

fn ident(name: impl Into<String>, span: Span) -> o::Ident {
    o::Ident {
        name: name.into(),
        span,
    }
}

fn binder(b: &n::Binder) -> o::Binder {
    o::Binder {
        name: b.name.clone(),
        discard: b.discard,
        span: b.span.0,
    }
}

fn expr(kind: o::ExprKind, span: Span) -> o::Expr {
    o::Expr { kind, span }
}

fn pat(kind: o::PatternKind, span: Span) -> o::Pattern {
    o::Pattern { kind, span }
}

fn ty(kind: o::TypeKind, span: Span) -> o::TypeExpr {
    o::TypeExpr { kind, span }
}

fn name_expr(name: &str, span: Span) -> o::Expr {
    expr(
        o::ExprKind::Name {
            module: None,
            name: ident(name, span),
        },
        span,
    )
}

/// The transition form of a multi-shape value (deleted in stage 12, when
/// the resolver reads the syntax-three tree): a call of the reserved name
/// [`MULTI`] with one argument per shape group, whose label is the group's
/// shape ([`shape_text`]) and whose value is the group's function.
fn multi_expr(groups: Vec<(String, o::Expr)>, span: Span) -> o::Expr {
    let at = Span::new(span.end.saturating_sub(1), span.end);
    let args = groups
        .into_iter()
        .map(|(shape, value)| o::Arg {
            label: Some(ident(shape, value.span)),
            value,
        })
        .collect();
    expr(
        o::ExprKind::Call {
            callee: Box::new(name_expr(MULTI, at)),
            args,
        },
        at,
    )
}

/// The bare names of a head `(A, B)`, when every part is one.
fn head_params(head: &[n::Type]) -> Option<Vec<o::Ident>> {
    head.iter()
        .map(|t| match &t.kind {
            n::TypeKind::Named {
                module: None,
                name,
                args,
            } if args.is_empty() => Some(id(name)),
            _ => None,
        })
        .collect()
}

/// The name of a member `C` or `C(A, …)` of a sum, when its arguments are
/// bare names.
fn member_name(t: &n::Type) -> Option<&str> {
    match &t.kind {
        n::TypeKind::Named {
            module: None,
            name,
            args,
        } if head_params(args).is_some() => Some(&name.name),
        _ => None,
    }
}

impl Cx {
    fn boundary(&self, kind: u8, span: Span) -> bool {
        self.fragment != Some((kind, span))
            && match kind {
                0 => self.transition.expressions.contains(&span),
                1 => self.transition.types.contains(&span),
                _ => self.transition.patterns.contains(&span),
            }
    }
    fn standalone(&self, kind: u8, span: Span) -> Cx {
        let mut cx = Cx::plain();
        cx.transition = self.transition.clone();
        cx.fragment = Some((kind, span));
        cx
    }
    fn plain() -> Cx {
        Cx {
            transition: Default::default(),
            fragment: None,
            lisp: false,
            generated_from: None,
            errors: Vec::new(),
            rows: Vec::new(),
            fresh: 0,
            core_free: false,
            group: false,
            hidden: Vec::new(),
        }
    }

    fn finish<T>(mut self, out: T) -> Result<T, Diagnostic> {
        if self.errors.is_empty() {
            Ok(out)
        } else {
            Err(self.errors.remove(0))
        }
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.errors
            .push(Diagnostic::new(Code::Unexpected, span, message));
    }

    fn fresh(&mut self, what: &str) -> String {
        self.fresh += 1;
        format!("%{what}{}", self.fresh)
    }

    // ---- Module and items ----

    fn module(&mut self, m: &n::Module) -> o::Module {
        let (items, absorbed) = self.items(&m.items);
        let cover = |spans: Vec<Span>| spans.into_iter().reduce(Span::to);
        o::Module {
            doc: m.doc.clone(),
            imports: m
                .imports
                .iter()
                .map(|i| o::ImportItem {
                    alias: opt_id(&i.alias),
                    names: i.names.iter().map(id).collect(),
                    path: i.path.iter().map(id).collect(),
                    span: i.span.0,
                })
                .collect(),
            exports: m
                .exports
                .iter()
                .filter(|e| e.constructors || !absorbed.contains(&e.name.name))
                .flat_map(|e| {
                    // The group bindings of an exported chain go with it.
                    let hidden = self
                        .hidden
                        .iter()
                        .filter(|(name, _)| !e.constructors && *name == e.name.name);
                    let mut items: Vec<o::ExportItem> = hidden
                        .map(|(_, h)| o::ExportItem {
                            name: ident(h.clone(), e.span.0),
                            constructors: false,
                            span: e.span.0,
                        })
                        .collect();
                    items.push(o::ExportItem {
                        name: id(&e.name),
                        constructors: e.constructors,
                        span: e.span.0,
                    });
                    items
                })
                .collect(),
            items,
            imports_span: cover(m.imports.iter().map(|i| i.span.0).collect()),
            exports_span: cover(m.exports.iter().map(|e| e.span.0).collect()),
        }
    }

    /// The items, and the names of the one-constructor types that became
    /// constructors of a sum.
    ///
    /// A definition `T = C` or `T(A) = C(A)` with one member is a sum of
    /// one constructor only while `C` is not written as a type elsewhere
    /// in the module; otherwise it is the alias it reads as (§2.8, kind 4)
    /// and `C` keeps its type. (A union `C1 | … | Ck` is still read as one
    /// type with those constructors, so a member written as a type is not
    /// defined: the final tree of lane/fa removes this.)
    fn items(&mut self, items: &[n::Item]) -> (Vec<o::Item>, HashSet<String>) {
        struct Used(HashSet<String>);
        impl crate::walk::Visitor for Used {
            fn ty(&mut self, t: &mut o::TypeExpr) {
                if let o::TypeKind::Named {
                    module: None, name, ..
                } = &t.kind
                {
                    self.0.insert(name.name.clone());
                }
            }
        }
        let (errors, fresh) = (self.errors.len(), self.fresh);
        let mut plain = HashSet::new();
        loop {
            let (mut out, absorbed, single) = self.items_once(items, &plain);
            let mut used = Used(HashSet::new());
            crate::walk::Walk::walk(&mut out, &mut used);
            let aliases: Vec<usize> = single
                .into_iter()
                .filter(|(_, member)| used.0.contains(member))
                .map(|(i, _)| i)
                .collect();
            if aliases.is_empty() {
                return (out, absorbed);
            }
            plain.extend(aliases);
            self.errors.truncate(errors);
            self.fresh = fresh;
        }
    }

    /// The items, with the definitions at `plain` read as aliases; the
    /// names of the absorbed types; and the one-member sums with the name
    /// of their member.
    #[allow(clippy::type_complexity)]
    fn items_once(
        &mut self,
        items: &[n::Item],
        plain: &HashSet<usize>,
    ) -> (Vec<o::Item>, HashSet<String>, Vec<(usize, String)>) {
        // The sums: a definition `T = C1 | … | Ck` whose `k` members are
        // the data definitions just before it, in order.
        let mut member_of: HashMap<usize, usize> = HashMap::new();
        let mut absorbed = HashSet::new();
        let mut single = Vec::new();
        if self.transition.complete {
            for (i, item) in items.iter().enumerate() {
                if let Some(members) = self.transition.sums.get(&item.span.0) {
                    for member in members {
                        if let Some(at) = items.iter().position(|item| item.span.0 == *member) {
                            member_of.insert(at, i);
                            if let n::ItemKind::TypeDef(def) = &items[at].kind
                                && let n::ItemKind::TypeDef(parent) = &item.kind
                                && def.name.name != parent.name.name
                            {
                                absorbed.insert(def.name.name.clone());
                            }
                        }
                    }
                }
            }
        } else {
            for (i, item) in items.iter().enumerate() {
                let n::ItemKind::TypeDef(def) = &item.kind else {
                    continue;
                };
                if plain.contains(&i) {
                    continue;
                }
                let n::TypeRhs::Type(rhs) = &def.rhs else {
                    continue;
                };
                let members: Vec<&n::Type> = match &rhs.kind {
                    n::TypeKind::Union(ms) => ms.iter().collect(),
                    _ => vec![rhs],
                };
                let k = members.len();
                if k > i || head_params(&def.head).is_none() {
                    continue;
                }
                let fits = members.iter().enumerate().all(|(j, m)| {
                    let at = i - k + j;
                    !member_of.contains_key(&at)
                        && match &items[at].kind {
                            n::ItemKind::TypeDef(n::TypeDef {
                                name,
                                head,
                                rhs: n::TypeRhs::Data(_),
                                ..
                            }) => {
                                member_name(m) == Some(name.name.as_str())
                                    && matches!(&m.kind, n::TypeKind::Named { args, .. }
                                        if args.len() == head.len()
                                            && args.iter().zip(head).all(|(a, b)| member_name(a) == member_name(b)))
                            },
                            _ => false,
                        }
                });
                if fits {
                    for j in 0..k {
                        member_of.insert(i - k + j, i);
                        let n::ItemKind::TypeDef(member) = &items[i - k + j].kind else {
                            unreachable!()
                        };
                        if member.name.name != def.name.name {
                            absorbed.insert(member.name.name.clone());
                            // Only a one-member right side: a union keeps today's
                            // reading (its members are constructors), as the
                            // exports of i/syntax and std rely on it.
                            if k == 1 && !matches!(rhs.kind, n::TypeKind::Union(_)) {
                                single.push((i, member.name.name.clone()));
                            }
                        }
                    }
                }
            }
        }
        let mut sums: HashMap<usize, Vec<o::Constructor>> = HashMap::new();
        let mut out = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let attributes = item.attributes.iter().map(|a| self.attribute(a)).collect();
            let kind = match &item.kind {
                n::ItemKind::TypeDef(def) => {
                    if let Some(sum) = member_of.get(&i) {
                        let n::TypeRhs::Data(fields) = &def.rhs else {
                            unreachable!()
                        };
                        let c = self.constructor(&def.name, fields, item.span.0);
                        sums.entry(*sum).or_default().push(c);
                        continue;
                    }
                    o::ItemKind::TypeDef(self.type_def(def, sums.remove(&i), item.span.0))
                }
                n::ItemKind::Bind(b) => {
                    let mut bindings = self.bindings(b, true);
                    let last = bindings.pop().unwrap();
                    for hidden in bindings {
                        out.push(o::Item {
                            doc: Vec::new(),
                            attributes: Vec::new(),
                            span: hidden.span,
                            kind: o::ItemKind::Binding(hidden),
                        });
                    }
                    o::ItemKind::Binding(last)
                }
                n::ItemKind::Decl { name, ty: t } => {
                    let mut type_params = Vec::new();
                    let ty = match &t.kind {
                        n::TypeKind::Fn(f) if !f.generics.is_empty() => {
                            type_params = f.generics.iter().map(id).collect();
                            self.fn_type(f, t.span.0, true)
                        }
                        _ => self.ty(t),
                    };
                    o::ItemKind::Declaration {
                        name: id(name),
                        type_params,
                        ty,
                    }
                }
                n::ItemKind::Macro { module, name, args } => o::ItemKind::MacroItem {
                    module: opt_id(module),
                    name: id(name),
                    args: self.args(args),
                },
            };
            out.push(o::Item {
                doc: item.doc.clone(),
                attributes,
                kind,
                span: item.span.0,
            });
        }
        (out, absorbed, single)
    }

    fn attribute(&mut self, a: &n::Attribute) -> o::Attribute {
        let mut args = Vec::new();
        let mut call = None;
        if let Some(items) = &a.args {
            let strings: Option<Vec<(String, Span)>> = items
                .iter()
                .map(|i| match i {
                    n::DItem::Pos(e) => match &e.kind {
                        n::ExprKind::String(s) => Some((s.clone(), e.span.0)),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            match strings {
                Some(strings) if !strings.is_empty() => args = strings,
                _ => call = Some((self.args(items), a.span.0)),
            }
        }
        o::Attribute {
            module: opt_id(&a.module),
            name: id(&a.name),
            args,
            call,
            span: a.span.0,
        }
    }

    fn constructor(&mut self, name: &n::Ident, fields: &[n::TItem], span: Span) -> o::Constructor {
        let fields = fields
            .iter()
            .map(|f| match f {
                n::TItem::Pos(t) => o::Field {
                    label: None,
                    ty: self.ty(t),
                    span: t.span.0,
                },
                n::TItem::Label(l, t) => o::Field {
                    label: Some(id(l)),
                    ty: self.ty(t),
                    span: l.span.0.to(t.span.0),
                },
                n::TItem::Spread(t) => {
                    self.error(t.span.0, "`..` is only written in needs, after `<-`");
                    o::Field {
                        label: None,
                        ty: self.ty(t),
                        span: t.span.0,
                    }
                }
            })
            .collect();
        o::Constructor {
            name: id(name),
            fields,
            span,
        }
    }

    fn type_def(
        &mut self,
        def: &n::TypeDef,
        sum: Option<Vec<o::Constructor>>,
        span: Span,
    ) -> o::TypeDef {
        let params = head_params(&def.head);
        let custom = |cx: &mut Cx, ctors: Vec<o::Constructor>| {
            if params.is_none() {
                cx.error(
                    def.name.span.0,
                    "the head of a data type or an opaque type has only type parameters",
                );
            }
            o::TypeDef {
                alias_pattern: false,
                name: id(&def.name),
                params: params.clone().unwrap_or_default(),
                head: Vec::new(),
                body: o::TypeBody::Custom(ctors),
            }
        };
        if let Some(ctors) = sum {
            return custom(self, ctors);
        }
        match &def.rhs {
            n::TypeRhs::Opaque => custom(self, Vec::new()),
            n::TypeRhs::Data(fields) => {
                let c = self.constructor(&def.name, fields, span);
                custom(self, vec![c])
            }
            n::TypeRhs::Type(t) => {
                let source = self.ty(t);
                let alias_pattern = def.alias_pattern;
                let (params, head) = match params {
                    Some(params) if !alias_pattern => (params, Vec::new()),
                    _ => (Vec::new(), def.head.iter().map(|t| self.ty(t)).collect()),
                };
                o::TypeDef {
                    alias_pattern,
                    name: id(&def.name),
                    params,
                    head,
                    body: o::TypeBody::Alias(source),
                }
            }
        }
    }

    // ---- Statements ----

    fn binding(&mut self, b: &n::Binding) -> o::Binding {
        let mut generics: Vec<&n::Ident> = Vec::new();
        if let n::ExprKind::Chain(fs) = &b.value.kind {
            // Branches may share a name; a name written twice in one list
            // is kept twice, so that the resolver reports it (E0226).
            for f in fs {
                let before = generics.len();
                for g in &f.generics {
                    if !generics[..before].iter().any(|x| x.name == g.name) {
                        generics.push(g);
                    }
                }
            }
        }
        let typed_fn = match &b.pattern.kind {
            n::PatternKind::Typed { ty, .. } if !self.boundary(1, ty.span.0) => match &ty.kind {
                n::TypeKind::Fn(f) => Some(f),
                _ => None,
            },
            _ => None,
        };
        if let Some(f) = typed_fn {
            for g in &f.generics {
                if !generics.iter().any(|x| x.name == g.name) {
                    generics.push(g);
                }
            }
        }
        let mut with_fail = HashSet::new();
        pattern_fail_rows(&b.pattern, &mut with_fail);
        if let n::ExprKind::Chain(fs) = &b.value.kind {
            for f in fs {
                for p in &f.params {
                    p.choices
                        .iter()
                        .for_each(|c| pattern_fail_rows(c, &mut with_fail));
                }
                for s in f.services.iter().flatten() {
                    fail_rows(&s.ty, &mut with_fail);
                }
                if let Some(r) = &f.ret {
                    fail_rows(r, &mut with_fail);
                }
            }
        }
        let original = self
            .transition
            .parameters
            .get(&(b.span.0, b.pattern.span.0))
            .cloned();
        let mark = self.rows.len();
        self.rows.extend(generics.iter().map(|g| Row {
            name: g.name.clone(),
            carries: original.as_ref().map_or_else(
                || !with_fail.contains(&g.name),
                |parameters| {
                    parameters
                        .iter()
                        .any(|(name, _)| *name == format!("{}%E", g.name))
                },
            ),
            used: false,
        }));
        let pattern = match (&b.pattern.kind, typed_fn) {
            (n::PatternKind::Typed { binder: bd, ty: t }, Some(f)) => pat(
                o::PatternKind::Typed {
                    binder: binder(bd),
                    ty: self.fn_type(f, t.span.0, true),
                },
                b.pattern.span.0,
            ),
            _ => self.pattern(&b.pattern),
        };
        let value = match &b.value.kind {
            n::ExprKind::Chain(fs) if !self.boundary(0, b.value.span.0) => {
                self.chain(fs, b.value.span.0, true)
            }
            _ => self.expr(&b.value),
        };
        let mut type_params: Vec<o::Ident> = generics.iter().map(|g| id(g)).collect();
        for row in self.rows.drain(mark..) {
            if row.used {
                type_params.push(ident(format!("{}%E", row.name), b.span.0));
            }
        }
        if let Some(original) = original {
            type_params = original
                .into_iter()
                .map(|(name, span)| ident(name, span))
                .collect();
        }
        o::Binding {
            pattern,
            value,
            span: b.span.0,
            type_params,
        }
    }

    fn block(&mut self, b: &n::Block) -> o::Block {
        // A provide that ends a block is today's environment statements
        // before the rest of that block.
        let mut stmts = Vec::new();
        for (i, s) in b.stmts.iter().enumerate() {
            match s {
                n::Stmt::Expr(n::Expr {
                    kind: n::ExprKind::Provide { target, services },
                    ..
                }) if i + 1 == b.stmts.len() => stmts.extend(self.provide(target, services)),
                n::Stmt::Bind(b) => {
                    stmts.extend(self.bindings(b, false).into_iter().map(o::Stmt::Bind))
                }
                _ => stmts.push(self.stmt(s)),
            }
        }
        let mut block = o::Block {
            stmts,
            span: b.span.0,
        };
        // A migrated typed provider is a single-use local immediately before
        // its provide. Keep its annotation on the environment statement and
        // its value there too: a separate let would generalize it too early.
        let candidate = block.stmts.windows(2).any(|ss| {
            matches!(
                ss,
                [
                    o::Stmt::Bind(o::Binding {
                        pattern: o::Pattern {
                            kind: o::PatternKind::Typed { .. },
                            ..
                        },
                        ..
                    }),
                    o::Stmt::Env { .. }
                ]
            )
        });
        if candidate {
            use crate::walk::{Visitor, Walk};
            #[derive(Default)]
            struct Uses(HashMap<String, usize>);
            impl Visitor for Uses {
                fn expr(&mut self, e: &mut o::Expr, enter: bool) {
                    if enter && let o::ExprKind::Name { module: None, name } = &e.kind {
                        *self.0.entry(name.name.clone()).or_default() += 1;
                    }
                }
            }
            let mut uses = Uses::default();
            block.walk(&mut uses);
            let mut i = 0;
            while i + 1 < block.stmts.len() {
                let contract = match (&block.stmts[i], &block.stmts[i + 1]) {
                    (o::Stmt::Bind(b), o::Stmt::Env { value, .. }) => {
                        match (&b.pattern.kind, &value.kind) {
                            (
                                o::PatternKind::Typed { binder, ty },
                                o::ExprKind::Name { module: None, name },
                            ) if !binder.discard
                                && binder.name == name.name
                                && uses.0.get(&binder.name) == Some(&1) =>
                            {
                                Some(ty.clone())
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if let Some(mut contract) = contract {
                    // Closed function-valued providers keep their original
                    // generalization point and its deferred obligations.
                    // Open contracts still have to reach the provide before
                    // generalization; scalar providers carry no function scheme.
                    struct Boundary<'a> {
                        rows: &'a [Row],
                        contract: bool,
                        open: bool,
                        function: bool,
                    }
                    impl Visitor for Boundary<'_> {
                        fn ty(&mut self, t: &mut o::TypeExpr) {
                            if self.contract {
                                self.open |= matches!(&t.kind, o::TypeKind::Hole)
                                    || matches!(&t.kind, o::TypeKind::Named { module: None, name, args }
                                        if args.is_empty() && self.rows.iter().any(|r| r.name == name.name));
                            }
                        }
                        fn expr(&mut self, e: &mut o::Expr, enter: bool) {
                            if enter {
                                self.function |=
                                    matches!(e.kind, o::ExprKind::Fn(_) | o::ExprKind::Clauses(_));
                            }
                        }
                    }
                    let mut boundary = Boundary {
                        rows: &self.rows,
                        contract: true,
                        open: false,
                        function: false,
                    };
                    contract.walk(&mut boundary);
                    if !boundary.open {
                        boundary.contract = false;
                        let o::Stmt::Bind(binding) = &mut block.stmts[i] else {
                            unreachable!()
                        };
                        binding.value.walk(&mut boundary);
                        if boundary.function {
                            i += 1;
                            continue;
                        }
                    }
                    let o::Stmt::Bind(binding) = block.stmts.remove(i) else {
                        unreachable!()
                    };
                    let o::Stmt::Env { ty, value, .. } = &mut block.stmts[i] else {
                        unreachable!()
                    };
                    *ty = contract;
                    *value = binding.value;
                } else {
                    i += 1;
                }
            }
        }
        block
    }

    fn stmt(&mut self, s: &n::Stmt) -> o::Stmt {
        match s {
            n::Stmt::Bind(b) => o::Stmt::Bind(self.binding(b)),
            n::Stmt::Expr(e) => o::Stmt::Expr(self.expr(e)),
        }
    }

    /// A binding as one or more bindings of the current tree. A multi chain
    /// bound to one name with no written type (§2.6.1) becomes one hidden
    /// binding per shape group and then the name itself, bound to the
    /// multi-shape value of those groups.
    fn bindings(&mut self, b: &n::Binding, top: bool) -> Vec<o::Binding> {
        let (n::PatternKind::Var(name), n::ExprKind::Chain(fs)) = (&b.pattern.kind, &b.value.kind)
        else {
            return vec![self.binding(b)];
        };
        if !is_multi(fs) || self.boundary(0, b.value.span.0) {
            return vec![self.binding(b)];
        }
        let span = b.value.span.0;
        let mut out = Vec::new();
        let mut groups = Vec::new();
        for (i, group) in shape_groups(fs).into_iter().enumerate() {
            let first = group[0].span.0;
            let at = Span::new(first.start, first.start + 1);
            let (positions, labels) = shape_of(&group[0]);
            let shape = shape_text(positions, &labels);
            let hidden = group_name(name, i);
            let cover = first.to(group.last().unwrap().span.0);
            let binding = n::Binding {
                pattern: n::Pattern {
                    kind: n::PatternKind::Var(hidden.clone()),
                    span: n::Sp(at),
                },
                value: n::Expr {
                    kind: n::ExprKind::Chain(group),
                    span: n::Sp(cover),
                },
                span: b.span,
            };
            self.group = true;
            out.push(self.binding(&binding));
            groups.push((shape, name_expr(&hidden, at)));
            if top {
                self.hidden.push((name.clone(), hidden));
            }
        }
        out.push(o::Binding {
            pattern: self.pattern(&b.pattern),
            value: multi_expr(groups, span),
            span: b.span.0,
            type_params: Vec::new(),
        });
        out
    }

    /// A multi chain that is not bound to one name: the multi-shape value
    /// of its group functions, written in place.
    fn multi_value(&mut self, fs: &[n::Function], span: Span) -> o::Expr {
        let mut groups = Vec::new();
        for group in shape_groups(fs) {
            let (positions, labels) = shape_of(&group[0]);
            let shape = shape_text(positions, &labels);
            let cover = group[0].span.0.to(group.last().unwrap().span.0);
            self.group = true;
            groups.push((shape, self.chain(&group, cover, false)));
        }
        multi_expr(groups, span)
    }

    /// `e <- (s1, s2)`: the environment statements and then `e`. Every
    /// service is worked out in the outer environment (§2.10.5): when there
    /// are several and one is not a name, each is bound to a fresh local
    /// first, so that no service sees another.
    fn provide(&mut self, target: &n::Expr, services: &n::Expr) -> Vec<o::Stmt> {
        let mut stmts = Vec::new();
        let mut envs = Vec::new();
        match &services.kind {
            n::ExprKind::Data(items) => {
                let apart = items.len() > 1
                    && items.iter().any(|i| {
                        !matches!(i, n::DItem::Pos(n::Expr { kind: n::ExprKind::Name { .. }, .. }))
                    });
                for item in items {
                    match item {
                        n::DItem::Pos(e) => {
                            let mut value = self.expr(e);
                            if apart {
                                let name = self.fresh("service");
                                stmts.push(o::Stmt::Bind(o::Binding {
                                    pattern: pat(o::PatternKind::Var(name.clone()), e.span.0),
                                    value,
                                    span: e.span.0,
                                    type_params: Vec::new(),
                                }));
                                value = expr(
                                    o::ExprKind::Name {
                                        module: None,
                                        name: ident(name, e.span.0),
                                    },
                                    e.span.0,
                                );
                            }
                            envs.push(o::Stmt::Env {
                                ty: ty(o::TypeKind::Hole, e.span.0),
                                value,
                                span: e.span.0,
                            });
                        }
                        n::DItem::Label(l, _) => self.error(
                            l.span.0,
                            "a provided service has no label: it is keyed by its type (§2.10.2)",
                        ),
                        n::DItem::Spread(e) => self.error(
                            e.span.0,
                            "providing a whole environment `..env` needs stage E6 of syntax three, which this compiler does not have yet",
                        ),
                    }
                }
            }
            _ => self.error(
                services.span.0,
                "providing a value that is not written as data `(s1, s2)` needs stage E6 of syntax three, which this compiler does not have yet",
            ),
        }
        stmts.extend(envs);
        match &target.kind {
            n::ExprKind::Block(b) => stmts.extend(self.block(b).stmts),
            n::ExprKind::Provide {
                target: inner,
                services: more,
            } => stmts.extend(self.provide(inner, more)),
            _ => stmts.push(o::Stmt::Expr(self.expr(target))),
        }
        stmts
    }

    // ---- Functions ----

    fn params_of(&mut self, f: &n::Function, params: &mut Vec<o::Param>) {
        let mut fails = 0;
        for s in f.services.iter().flatten() {
            // `name: Fail(E)` is today's error part (`ret`).
            if fail_of(s).is_some() {
                fails += 1;
                if fails == 2 {
                    self.error(
                        s.ty.span.0,
                        "a need has one `Fail` entry: write `Fail(A | B)`",
                    );
                }
                continue;
            }
            if s.rest {
                self.error(
                    s.name.span.0,
                    "`..name: N` in a service pattern needs stage E6 of syntax three, which this compiler does not have yet",
                );
                continue;
            }
            params.push(o::Param::Env {
                name: id(&s.name),
                ty: self.ty(&s.ty),
            });
        }
    }

    /// The written `-> R`: the needs are left to inference. A `Fail(E)`
    /// item of the service pattern gives the error part `E`.
    fn ret(&mut self, f: &n::Function) -> Option<o::RetType> {
        let fail = f.services.iter().flatten().find_map(fail_of);
        let at = match (&f.ret, fail) {
            (Some(t), _) => t.span.0,
            (None, Some(e)) => e.span.0,
            (None, None) => return None,
        };
        let hole = || ty(o::TypeKind::Hole, at);
        Some(o::RetType {
            value: f.ret.as_ref().map_or_else(hole, |t| self.ty(t)),
            err: Some(fail.map_or_else(hole, |e| self.ty(e))),
            env: Some(hole()),
        })
    }

    fn body(&mut self, b: &n::Block) -> o::Expr {
        match b.stmts.as_slice() {
            [n::Stmt::Expr(e)] => self.expr(e),
            _ => expr(o::ExprKind::Block(self.block(b)), b.span.0),
        }
    }

    /// A chain as a value. `bound` is set when the chain is the value of a
    /// binding, which takes its type parameters.
    fn chain(&mut self, fs: &[n::Function], span: Span, bound: bool) -> o::Expr {
        let group = std::mem::replace(&mut self.group, false);
        if !group && is_multi(fs) {
            return self.multi_value(fs, span);
        }
        // The branches of one shape group take their parts in canonical
        // order; the keys are in the shape.
        let fs: Vec<n::Function> = fs
            .iter()
            .map(|f| {
                let mut g = f.clone();
                g.params = canonical(f)
                    .into_iter()
                    .map(|p| n::PItem {
                        key: None,
                        choices: p.choices.clone(),
                    })
                    .collect();
                g
            })
            .collect();
        let fs = fs.as_slice();
        for f in fs {
            if !bound && !f.generics.is_empty() {
                self.error(
                    f.generics[0].span.0,
                    "type parameters on a function literal that is not the value of a binding need stage 7 of syntax three, which this compiler does not have yet",
                );
            }
        }
        let count = fs[0].params.len();
        let plain =
            |f: &n::Function| f.guard.is_none() && f.params.iter().all(|p| p.choices.len() == 1);
        if let [f] = fs
            && plain(f)
        {
            let mut params: Vec<o::Param> = f
                .params
                .iter()
                .map(|p| o::Param::Pattern(self.pattern(&p.choices[0])))
                .collect();
            self.params_of(f, &mut params);
            let function = o::Function {
                params,
                ret: self.ret(f),
                body: self.block(&f.body),
            };
            return expr(o::ExprKind::Fn(Box::new(function)), span);
        }
        let bare = fs.iter().all(|f| f.services.is_none() && f.ret.is_none());
        if count == 1 && bare {
            let clauses = fs.iter().map(|f| self.clause(f, 1)).collect();
            return expr(o::ExprKind::Clauses(clauses), span);
        }
        if count == 0 {
            self.error(
                span,
                "a chain of branches with no parameter has nothing to choose by",
            );
        }
        // Several positions, or a written need or result: a function of
        // fresh parameters over a match.
        // Each fresh parameter has the span of the pattern of its position
        // in the first branch, and the match a span of its own: the checker
        // keeps facts by span.
        let names: Vec<(String, Span)> = (0..count)
            .map(|i| {
                let at = fs[0].params.get(i).map_or(span, |p| p.choices[0].span.0);
                (self.fresh("arg"), at)
            })
            .collect();
        let synthetic = Span::new(span.start, span.start);
        let inside = Span::new(span.start + 1, span.end.max(span.start + 1));
        let mut params: Vec<o::Param> = names
            .iter()
            .map(|(a, at)| o::Param::Pattern(pat(o::PatternKind::Var(a.clone()), *at)))
            .collect();
        let mut seen = HashSet::new();
        for f in fs {
            let mut own = Vec::new();
            self.params_of(f, &mut own);
            for p in own {
                if let o::Param::Env { name, .. } = &p
                    && seen.insert(name.name.clone())
                {
                    params.push(p);
                }
            }
        }
        // The written results: one type when the branches write the same
        // one; their union when every branch writes one (each branch body
        // keeps its tail position); none when a branch leaves it out.
        let mut written: Vec<&n::Type> = Vec::new();
        for t in fs.iter().filter_map(|f| f.ret.as_ref()) {
            if !written.contains(&t) {
                written.push(t);
            }
        }
        let ret = match written.as_slice() {
            [] => fs
                .iter()
                .find(|f| f.services.iter().flatten().any(|s| fail_of(s).is_some()))
                .and_then(|f| self.ret(f)),
            [_] => fs
                .iter()
                .find(|f| {
                    f.ret.is_some() || f.services.iter().flatten().any(|s| fail_of(s).is_some())
                })
                .and_then(|f| self.ret(f)),
            _ if fs.iter().any(|f| f.ret.is_none()) => None,
            many => {
                let at = many[0].span.0;
                let members = many.iter().map(|t| self.ty(t)).collect();
                Some(o::RetType {
                    value: ty(o::TypeKind::Union(members), at),
                    err: Some(ty(o::TypeKind::Hole, at)),
                    env: Some(ty(o::TypeKind::Hole, at)),
                })
            }
        };
        let subject = if count == 1 {
            name_expr(&names[0].0, names[0].1)
        } else {
            expr(
                o::ExprKind::Tuple(names.iter().map(|(a, at)| name_expr(a, *at)).collect()),
                synthetic,
            )
        };
        let clauses = fs.iter().map(|f| self.clause(f, count)).collect();
        let case = expr(
            o::ExprKind::Case {
                subject: Box::new(subject),
                clauses,
            },
            inside,
        );
        let function = o::Function {
            params,
            ret,
            body: o::Block {
                stmts: vec![o::Stmt::Expr(case)],
                span,
            },
        };
        expr(o::ExprKind::Fn(Box::new(function)), span)
    }

    /// A branch as a clause over one subject: the pattern of its one
    /// position, or the tuple of the patterns of its `count` positions.
    fn clause(&mut self, f: &n::Function, count: usize) -> o::Clause {
        let span = f.span.0;
        let patterns = if count == 1 && f.params.len() == 1 {
            f.params[0]
                .choices
                .iter()
                .map(|p| self.pattern(p))
                .collect()
        } else {
            // Every way to take one alternative of each position.
            let mut rows: Vec<Vec<o::Pattern>> = vec![Vec::new()];
            for p in &f.params {
                let choices: Vec<o::Pattern> = p.choices.iter().map(|c| self.pattern(c)).collect();
                rows = rows
                    .into_iter()
                    .flat_map(|row| {
                        choices.iter().map(move |c| {
                            let mut row = row.clone();
                            row.push(c.clone());
                            row
                        })
                    })
                    .collect();
            }
            rows.into_iter()
                .map(|row| pat(o::PatternKind::Tuple(row), span))
                .collect()
        };
        o::Clause {
            patterns,
            guard: f.guard.as_ref().map(|g| self.expr(g)),
            body: self.body(&f.body),
            span,
        }
    }

    // ---- Expressions ----

    fn args(&mut self, items: &[n::DItem]) -> Vec<o::Arg> {
        items
            .iter()
            .map(|i| match i {
                n::DItem::Pos(e) => o::Arg {
                    label: None,
                    value: self.expr(e),
                },
                n::DItem::Label(l, e) => o::Arg {
                    label: Some(id(l)),
                    value: self.expr(e),
                },
                n::DItem::Spread(e) => {
                    self.error(e.span.0, "`..` is the first item of a constructor call");
                    o::Arg {
                        label: None,
                        value: self.expr(e),
                    }
                }
            })
            .collect()
    }

    fn fields(&mut self, items: &[n::DItem]) -> Vec<(o::Ident, o::Expr)> {
        let mut out = Vec::new();
        for i in items {
            match i {
                n::DItem::Label(l, e) => out.push((id(l), self.expr(e))),
                n::DItem::Pos(e) | n::DItem::Spread(e) => self.error(
                    e.span.0,
                    "data with labels has only labelled items after an optional first `..base` (mixed data needs stage 6 of syntax three)",
                ),
            }
        }
        out
    }

    fn is_core_never(&self, e: &n::Expr) -> bool {
        self.core_free
            && matches!(&e.kind, n::ExprKind::Field { target, name }
                if name.name == "never"
                    && matches!(&target.kind, n::ExprKind::Name { module: None, name } if name.name == "core"))
    }

    fn expr(&mut self, e: &n::Expr) -> o::Expr {
        if self.boundary(0, e.span.0) {
            let mut cx = self.standalone(0, e.span.0);
            let result = cx.expr(e);
            self.errors.extend(cx.errors);
            return result;
        }
        let span = e.span.0;
        let kind = match &e.kind {
            n::ExprKind::Quote(q) => {
                if crate::lisp::syntax::ROWS3 {
                    if let Err(error) = super::lisp::quote_captures(&q.0, false) {
                        self.errors.push(error);
                    }
                    o::ExprKind::Quote(q.0.clone())
                } else {
                    o::ExprKind::Quote(Box::new(super::lisp::unconvert_quote(&q.0)))
                }
            }
            n::ExprKind::Int(v) => o::ExprKind::Int(v.clone()),
            n::ExprKind::Float(v) => o::ExprKind::Float(*v),
            n::ExprKind::String(s) => o::ExprKind::String(s.clone()),
            n::ExprKind::Interp(parts) => {
                let mut operands: Vec<o::Expr> = parts
                    .iter()
                    .map(|p| match p {
                        n::InterpPart::Text(s) => expr(o::ExprKind::String(s.clone()), span),
                        n::InterpPart::Expr(e) => self.expr(e),
                    })
                    .collect();
                if operands.len() < 2 {
                    operands.insert(0, expr(o::ExprKind::String(String::new()), span));
                }
                let mut operands = operands.into_iter();
                let mut acc = operands.next().unwrap();
                for next in operands {
                    acc = expr(
                        o::ExprKind::Binary {
                            op: o::BinOp::Concat,
                            lhs: Box::new(acc),
                            rhs: Box::new(next),
                        },
                        span,
                    );
                }
                return acc;
            }
            n::ExprKind::Name { module, name } => o::ExprKind::Name {
                module: opt_id(module),
                name: id(name),
            },
            n::ExprKind::Data(items) => {
                // `()` is the empty data, a value and a pattern of the type
                // `()` (§7.1 row 7): an empty tuple, as its pattern is.
                if items.iter().all(|i| matches!(i, n::DItem::Pos(_))) {
                    o::ExprKind::Tuple(
                        items
                            .iter()
                            .map(|i| match i {
                                n::DItem::Pos(e) => self.expr(e),
                                _ => unreachable!(),
                            })
                            .collect(),
                    )
                } else {
                    let (base, rest) = match items.split_first() {
                        Some((n::DItem::Spread(b), rest)) => (Some(Box::new(self.expr(b))), rest),
                        _ => (None, items.as_slice()),
                    };
                    o::ExprKind::Record {
                        base,
                        fields: self.fields(rest),
                    }
                }
            }
            n::ExprKind::List { items, rest } => o::ExprKind::List {
                items: items.iter().map(|i| self.expr(i)).collect(),
                rest: rest.as_ref().map(|r| Box::new(self.expr(r))),
            },
            n::ExprKind::BitArray(segments) => o::ExprKind::BitArray(
                segments
                    .iter()
                    .map(|s| o::Segment {
                        value: self.expr(&s.value),
                        format: self.format(&s.format),
                        span: s.value.span.0,
                    })
                    .collect(),
            ),
            n::ExprKind::Block(b) => o::ExprKind::Block(self.block(b)),
            n::ExprKind::Chain(fs) => return self.chain(fs, span, false),
            n::ExprKind::Call { callee, args } if self.is_core_never(callee) && args.len() == 1 => {
                let subject = match &args[0] {
                    n::DItem::Pos(e) | n::DItem::Label(_, e) | n::DItem::Spread(e) => self.expr(e),
                };
                o::ExprKind::Case {
                    subject: Box::new(subject),
                    clauses: Vec::new(),
                }
            }
            n::ExprKind::Call { callee, args } => match args.split_first() {
                Some((n::DItem::Spread(base), rest)) => o::ExprKind::RecordUpdate {
                    constructor: Box::new(self.expr(callee)),
                    base: Box::new(self.expr(base)),
                    fields: self.fields(rest),
                },
                _ => o::ExprKind::Call {
                    callee: Box::new(self.expr(callee)),
                    args: self.args(args),
                },
            },
            n::ExprKind::Provide { target, services } => o::ExprKind::Block(o::Block {
                stmts: self.provide(target, services),
                span,
            }),
            n::ExprKind::Field { .. } if self.is_core_never(e) => o::ExprKind::Clauses(Vec::new()),
            n::ExprKind::Field { target, name } => o::ExprKind::Field {
                target: Box::new(self.expr(target)),
                name: id(name),
            },
            n::ExprKind::Index { target, index } => o::ExprKind::TupleIndex {
                target: Box::new(self.expr(target)),
                index: index.clone(),
                index_span: Span::new(span.end.saturating_sub(1), span.end),
            },
            n::ExprKind::Unary { op, expr: inner } => o::ExprKind::Unary {
                op: *op,
                expr: Box::new(self.expr(inner)),
            },
            n::ExprKind::Binary { op, lhs, rhs } => o::ExprKind::Binary {
                op: *op,
                lhs: Box::new(self.expr(lhs)),
                rhs: Box::new(self.expr(rhs)),
            },
            n::ExprKind::Pipe { lhs, rhs } => match &rhs.kind {
                // A match (§2.11): one position per branch.
                n::ExprKind::Chain(fs)
                    if (!self.lisp
                        || self.generated_from.is_some_and(|start| span.start >= start))
                        && fs.iter().all(|f| {
                            f.params.len() == 1
                                && f.services.is_none()
                                && f.ret.is_none()
                                && f.generics.is_empty()
                                && f.params[0].key.is_none()
                        }) =>
                {
                    o::ExprKind::Case {
                        subject: Box::new(self.expr(lhs)),
                        clauses: fs.iter().map(|f| self.clause(f, 1)).collect(),
                    }
                }
                _ => o::ExprKind::Binary {
                    op: o::BinOp::Pipe,
                    lhs: Box::new(self.expr(lhs)),
                    rhs: Box::new(self.expr(rhs)),
                },
            },
        };
        expr(kind, span)
    }

    fn format(&mut self, f: &Option<n::Format>) -> Option<o::Format> {
        f.as_ref().map(|f| o::Format {
            name: id(&f.name),
            arg: f.arg.as_ref().map(|a| Box::new(self.expr(a))),
        })
    }

    // ---- Patterns ----

    fn pattern(&mut self, p: &n::Pattern) -> o::Pattern {
        if self.boundary(2, p.span.0) {
            let mut cx = self.standalone(2, p.span.0);
            let result = cx.pattern(p);
            self.errors.extend(cx.errors);
            return result;
        }
        let span = p.span.0;
        let kind = match &p.kind {
            n::PatternKind::Quote(q) if crate::lisp::syntax::ROWS3 => {
                match super::lisp::quote_captures(&q.datum.0, true) {
                    Ok(captures) => o::PatternKind::Quote {
                        datum: q.datum.0.clone(),
                        template: q.template,
                        // Native quote preparation and type expansion update
                        // the capture patterns. Keep their template slots and
                        // replacements instead of reconstructing stale nodes.
                        captures: (if q.captures.is_empty() {
                            &captures
                        } else {
                            &q.captures
                        })
                        .iter()
                        .map(|(splice, pattern)| (*splice, self.pattern(pattern)))
                        .collect(),
                    },
                    Err(error) => {
                        self.errors.push(error);
                        o::PatternKind::Discard("_".into())
                    }
                }
            }
            n::PatternKind::Quote(q) => {
                match crate::lisp::parse_pattern(&super::lisp::unconvert_quote(&q.datum.0)) {
                    Ok(p) => p.kind.clone(),
                    Err(d) => {
                        self.errors.push(d);
                        o::PatternKind::Discard("_".into())
                    }
                }
            }
            n::PatternKind::Var(s) => o::PatternKind::Var(s.clone()),
            n::PatternKind::Discard(s) => o::PatternKind::Discard(s.clone()),
            n::PatternKind::Typed { binder: b, ty: t } => o::PatternKind::Typed {
                binder: binder(b),
                ty: self.ty(t),
            },
            n::PatternKind::Int { negative, value } => o::PatternKind::Int {
                negative: *negative,
                value: value.clone(),
            },
            n::PatternKind::Float { negative, value } => o::PatternKind::Float {
                negative: *negative,
                value: *value,
            },
            n::PatternKind::String(s) => o::PatternKind::String(s.clone()),
            n::PatternKind::Prefix {
                prefix,
                suffix,
                rest,
            } => o::PatternKind::Prefix {
                prefix: prefix.clone(),
                suffix: suffix.clone(),
                rest: binder(rest),
            },
            n::PatternKind::Ctor { module, name, args } => {
                let mut out = Vec::new();
                let mut spread = false;
                for item in args.iter().flatten() {
                    match item {
                        n::DPItem::Pos(p) => out.push(self.pattern(p)),
                        n::DPItem::Key(l, p) => out.push(self.keyed(l, p)),
                        n::DPItem::Rest(None) => spread = true,
                        n::DPItem::Rest(Some(b)) => self.error(
                            b.span.0,
                            "a named rest `..name` in a pattern needs stage 6 of syntax three, which this compiler does not have yet",
                        ),
                    }
                }
                o::PatternKind::Constructor {
                    module: opt_id(module),
                    name: id(name),
                    args: out,
                    spread,
                }
            }
            n::PatternKind::Data(items) => {
                let mut out = Vec::new();
                for item in items {
                    match item {
                        n::DPItem::Pos(p) => out.push(self.pattern(p)),
                        n::DPItem::Key(l, _) => self.error(
                            l.span.0,
                            "a key in a data pattern needs stage 6 of syntax three, which this compiler does not have yet",
                        ),
                        n::DPItem::Rest(_) => self.error(
                            span,
                            "a rest `..` in a data pattern needs stage 6 of syntax three, which this compiler does not have yet",
                        ),
                    }
                }
                o::PatternKind::Tuple(out)
            }
            n::PatternKind::List { items, rest } => o::PatternKind::List {
                items: items.iter().map(|i| self.pattern(i)).collect(),
                rest: rest.as_ref().map(|r| r.as_ref().map(binder)),
            },
            n::PatternKind::BitArray(segments) => o::PatternKind::BitArray(
                segments
                    .iter()
                    .map(|s| o::Segment {
                        value: self.pattern(&s.value),
                        format: self.format(&s.format),
                        span: s.value.span.0,
                    })
                    .collect(),
            ),
            n::PatternKind::And(parts) => return self.and(parts, None, span),
        };
        pat(kind, span)
    }

    /// `p1 = p2 = …`: today's chain of names around one pattern. With
    /// `label`, the chain ends in the label of a constructor field.
    fn and(&mut self, parts: &[n::Pattern], label: Option<&n::Ident>, span: Span) -> o::Pattern {
        let mut names: Vec<o::Ident> = Vec::new();
        let mut core: Option<o::Pattern> = None;
        let mut discard = None;
        for p in parts {
            match &p.kind {
                n::PatternKind::Var(s) => names.push(ident(s.clone(), p.span.0)),
                n::PatternKind::Discard(_) => discard = Some(self.pattern(p)),
                _ if core.is_some() => self.error(
                    p.span.0,
                    "two patterns joined by `=` that are not names need stage 9 of syntax three, which this compiler does not have yet",
                ),
                _ => core = Some(self.pattern(p)),
            }
        }
        let mut inner = match (label, core) {
            (Some(l), Some(core)) => match core.kind.clone() {
                // `key = name: T`: today's `name = key: T`.
                o::PatternKind::Typed { binder: b, ty: t } => {
                    if b.name != l.name && !b.discard {
                        names.insert(0, ident(b.name.clone(), b.span));
                    }
                    pat(
                        o::PatternKind::Typed {
                            binder: o::Binder {
                                name: l.name.clone(),
                                discard: false,
                                span: l.span.0,
                            },
                            ty: t,
                        },
                        core.span,
                    )
                }
                _ => {
                    self.error(
                        core.span,
                        "a pattern inside a labelled field needs stage 4 of syntax three, which this compiler does not have yet; bind the field to a name and match the name",
                    );
                    core
                }
            },
            (Some(l), None) => {
                names.retain(|n| n.name != l.name);
                pat(o::PatternKind::Var(l.name.clone()), l.span.0)
            }
            (None, Some(core)) => core,
            (None, None) => match names.pop() {
                Some(last) => pat(o::PatternKind::Var(last.name), last.span),
                None => discard.take().expect("a conjunction has a part"),
            },
        };
        for name in names.into_iter().rev() {
            inner = pat(
                o::PatternKind::Named {
                    name,
                    pattern: Box::new(inner),
                },
                span,
            );
        }
        inner
    }

    /// `key = pattern` in a constructor pattern.
    fn keyed(&mut self, l: &n::Ident, p: &n::Pattern) -> o::Pattern {
        match &p.kind {
            n::PatternKind::And(parts) => self.and(parts, Some(l), p.span.0),
            n::PatternKind::Discard(_) => {
                // Today a labelled field is always bound to a name.
                let name = self.fresh("field");
                pat(
                    o::PatternKind::Named {
                        name: ident(name, p.span.0),
                        pattern: Box::new(pat(o::PatternKind::Var(l.name.clone()), l.span.0)),
                    },
                    p.span.0,
                )
            }
            _ => self.and(std::slice::from_ref(p), Some(l), p.span.0),
        }
    }

    // ---- Types ----

    /// The needs written after `<-` as the two rows of today's function
    /// type: the error channel and the environment. This is the only place
    /// where a need becomes a row: a `Fail(E)` entry is the error `E`, every
    /// other entry is a member of the environment, and `..N` (or `<- N`) is
    /// the row variable `N`, which also carries the errors when the
    /// signature writes no `Fail` beside it (§7.3).
    fn needs_row(
        &mut self,
        services: Option<&n::Type>,
        span: Span,
    ) -> (Option<o::TypeExpr>, Option<o::TypeExpr>) {
        let mut err: Option<o::TypeExpr> = None;
        let mut env: Vec<o::TypeExpr> = Vec::new();
        let mut row: Option<String> = None;
        let mut need = |cx: &mut Cx, t: &n::Type, spread: bool| {
            if let n::TypeKind::Named {
                module: None,
                name,
                args,
            } = &t.kind
            {
                if name.name == "Fail" && args.len() == 1 && !spread {
                    if err.is_some() {
                        cx.error(t.span.0, "a need has one `Fail` entry: write `Fail(A | B)`");
                    }
                    err = Some(cx.ty(&args[0]));
                    return;
                }
                if spread && args.is_empty() {
                    row = Some(name.name.clone());
                }
            }
            env.push(cx.ty(t));
        };
        match services.map(|s| (&s.kind, s)) {
            None => {}
            Some((n::TypeKind::Data(items), _)) => {
                for item in items {
                    match item {
                        n::TItem::Pos(t) => need(self, t, false),
                        n::TItem::Spread(t) => need(self, t, true),
                        n::TItem::Label(l, _) => self.error(
                            l.span.0,
                            "a need has no label: it is keyed by its type (§2.10.2)",
                        ),
                    }
                }
            }
            // `<- N`: a row variable.
            Some((_, t)) => need(self, t, true),
        }
        // The errors that a row variable carries (§7.3).
        if err.is_none()
            && let Some(row) = row
            && let Some(found) = self.rows.iter_mut().rev().find(|r| r.name == row)
            && found.carries
        {
            found.used = true;
            err = Some(ty(
                o::TypeKind::Named {
                    module: None,
                    name: ident(format!("{row}%E"), span),
                    args: Vec::new(),
                },
                span,
            ));
        }
        let env = match env.len() {
            0 => None,
            1 => env.pop(),
            _ => Some(ty(o::TypeKind::Union(env), span)),
        };
        (err, env)
    }

    fn fn_type(&mut self, f: &n::FnType, span: Span, generics_taken: bool) -> o::TypeExpr {
        if !generics_taken && !f.generics.is_empty() {
            self.error(
                f.generics[0].span.0,
                "type parameters on a function type that is not the type of a binding need stage 7 of syntax three, which this compiler does not have yet",
            );
        }
        let mut params = Vec::new();
        match &f.param.kind {
            n::TypeKind::Data(items)
                if items.is_empty() || items.iter().any(|i| !matches!(i, n::TItem::Label(..))) =>
            {
                for item in items {
                    match item {
                        n::TItem::Pos(t) => params.push(o::FnParamType {
                            label: None,
                            ty: self.ty(t),
                        }),
                        n::TItem::Label(l, t) => params.push(o::FnParamType {
                            label: Some(id(l)),
                            ty: self.ty(t),
                        }),
                        n::TItem::Spread(t) => {
                            self.error(t.span.0, "`..` is only written in needs, after `<-`")
                        }
                    }
                }
            }
            _ => params.push(o::FnParamType {
                label: Some(ident(DATA, f.param.span.0)),
                ty: self.ty(&f.param),
            }),
        }
        let (err, env) = self.needs_row(f.services.as_ref(), span);
        ty(
            o::TypeKind::Fn {
                params,
                ret: Box::new(self.ty(&f.ret)),
                err: err.map(Box::new),
                env: env.map(Box::new),
            },
            span,
        )
    }

    fn ty(&mut self, t: &n::Type) -> o::TypeExpr {
        if self.boundary(1, t.span.0) {
            let mut cx = self.standalone(1, t.span.0);
            let result = cx.ty(t);
            self.errors.extend(cx.errors);
            return result;
        }
        let span = t.span.0;
        let kind = match &t.kind {
            n::TypeKind::Lit(v) => o::TypeKind::Lit(v.clone()),
            n::TypeKind::Named { module, name, args } => o::TypeKind::Named {
                module: opt_id(module),
                name: id(name),
                args: args.iter().map(|a| self.ty(a)).collect(),
            },
            n::TypeKind::ValueOf { module, name } => o::TypeKind::ValueOf {
                module: opt_id(module),
                name: id(name),
            },
            n::TypeKind::Call { module, name, args } => o::TypeKind::Call {
                module: opt_id(module),
                name: id(name),
                args: args.iter().map(|a| self.ty(a)).collect(),
            },
            n::TypeKind::Hole => o::TypeKind::Hole,
            n::TypeKind::Data(items) => {
                if items.iter().all(|i| matches!(i, n::TItem::Pos(_))) && !items.is_empty() {
                    o::TypeKind::Tuple(
                        items
                            .iter()
                            .map(|i| match i {
                                n::TItem::Pos(t) => self.ty(t),
                                _ => unreachable!(),
                            })
                            .collect(),
                    )
                } else {
                    let mut fields = Vec::new();
                    for item in items {
                        match item {
                            n::TItem::Label(l, t) => fields.push((
                                o::RecordLabel {
                                    ident: id(l),
                                    parameter: false,
                                },
                                self.ty(t),
                            )),
                            n::TItem::Pos(t) | n::TItem::Spread(t) => self.error(
                                t.span.0,
                                "a data type with labels has only labelled items (mixed data needs stage 6 of syntax three)",
                            ),
                        }
                    }
                    o::TypeKind::Record {
                        base: None,
                        fields,
                        removed: Vec::new(),
                    }
                }
            }
            n::TypeKind::Fn(f) => return self.fn_type(f, span, false),
            n::TypeKind::Union(parts) => {
                o::TypeKind::Union(parts.iter().map(|p| self.ty(p)).collect())
            }
            n::TypeKind::Inter(parts) => {
                o::TypeKind::Inter(parts.iter().map(|p| self.ty(p)).collect())
            }
            n::TypeKind::Diff(parts) => {
                o::TypeKind::Diff(parts.iter().map(|p| self.ty(p)).collect())
            }
        };
        ty(kind, span)
    }
}

/// The `E` of a service item `name: Fail(E)`.
fn fail_of(s: &n::SItem) -> Option<&n::Type> {
    match &s.ty.kind {
        n::TypeKind::Named {
            module: None,
            name,
            args,
        } if !s.rest && name.name == "Fail" && args.len() == 1 => Some(&args[0]),
        _ => None,
    }
}
