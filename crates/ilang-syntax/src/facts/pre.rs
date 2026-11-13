//! What the lowering of syntax three (`crate::syn3::lower`) knew
//! about a module beside the tree it built, worked out from the
//! syntax-three tree alone:
//!
//! - the errors of the forms that this compiler cannot type yet ("needs
//!   stage 9", mixed data, …), with the same code, span, text and order;
//! - the fresh names (`%arg1`, `%service2`, `%field3`), numbered in the same
//!   order, so that the HIR has the same names;
//! - the type parameters of every binding (the generics of its functions and
//!   of its signature, and `N%E` for a row variable that carries the errors);
//! - the function types whose needs get the error parameter `N%E`;
//! - the sums: `C1 = (…)`, `C2 = (…)`, `T = C1 | C2`.
//!
//! The walk is the walk of `lower.rs`, form by form, without the output.
//! Nodes are named by their address: the tree is not moved while it is
//! resolved.

use super::Transition;
use crate::span::Span;
use crate::syn3::ast as n;
use crate::syn3::ast::resolved as o;
use crate::{Code, Diagnostic};
use std::collections::{HashMap, HashSet};

pub fn addr<T>(t: &T) -> usize {
    t as *const T as usize
}

/// The transition's written result and error channels, without old nodes.
pub struct Return<'a> {
    pub values: Vec<&'a n::Type>,
    pub error: Option<&'a n::Type>,
    pub span: Span,
}

pub fn fail_of(s: &n::SItem) -> Option<&n::Type> {
    match &s.ty.kind {
        n::TypeKind::Named {
            module: None,
            name,
            args,
        } if !s.rest && name.name == "Fail" && args.len() == 1 => Some(&args[0]),
        _ => None,
    }
}

pub fn function_return(f: &n::Function) -> Option<Return<'_>> {
    let error = f.services.iter().flatten().find_map(fail_of);
    let span = f.ret.as_ref().or(error)?.span.0;
    Some(Return {
        values: f.ret.iter().collect(),
        error,
        span,
    })
}

pub fn chain_return(fs: &[n::Function]) -> Option<Return<'_>> {
    let mut values = Vec::new();
    for t in fs.iter().filter_map(|f| f.ret.as_ref()) {
        if !values.contains(&t) {
            values.push(t);
        }
    }
    match values.as_slice() {
        [] => fs
            .iter()
            .find(|f| f.services.iter().flatten().any(|s| fail_of(s).is_some()))
            .and_then(function_return),
        [_] => fs
            .iter()
            .find(|f| f.ret.is_some() || f.services.iter().flatten().any(|s| fail_of(s).is_some()))
            .and_then(function_return),
        _ if fs.iter().any(|f| f.ret.is_none()) => None,
        _ => Some(Return {
            span: values[0].span.0,
            values,
            error: None,
        }),
    }
}

#[derive(Default)]
pub struct Pre {
    pub errors: Vec<Diagnostic>,
    pub functions: HashSet<Span>,
    /// By the address of the first function of a chain: its fresh
    /// parameters.
    pub args: HashMap<usize, Vec<String>>,
    /// By the address of a provided service: its fresh local.
    pub services: HashMap<usize, String>,
    /// By the address of the pattern `_` of `key = _`: its fresh name.
    pub fields: HashMap<usize, String>,
    /// By the address of a function type: the row variable `N` whose error
    /// parameter `N%E` is the error of the type.
    pub row_err: HashMap<usize, String>,
    /// By the address of a binding: its type parameters.
    pub type_params: HashMap<usize, Vec<o::Ident>>,
    /// The index of a member definition of a sum to the index of the sum.
    pub member_of: HashMap<usize, usize>,
    /// The names of the member definitions that became constructors.
    pub absorbed: HashSet<String>,
    pub core_free: bool,
    pub lisp: bool,
    pub matches: HashSet<usize>,
    pub never: HashSet<usize>,
}

struct Row {
    name: String,
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

pub fn id(i: &n::Ident) -> o::Ident {
    o::Ident {
        name: i.name.clone(),
        span: i.span.0,
    }
}

/// The bare names of a head `(A, B)`, when every part is one.
pub fn head_params(head: &[n::Type]) -> Option<Vec<o::Ident>> {
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

/// The sums of an item list: a definition `T = C1 | … | Ck` whose `k`
/// members are the data definitions just before it, in order. Gives the
/// index of each member to the index of its sum, and the names of the
/// members that are not the sum's own name.
pub fn sums(items: &[n::Item]) -> (HashMap<usize, usize>, HashSet<String>) {
    let mut plain = HashSet::new();
    loop {
        let mut member_of = HashMap::new();
        let mut absorbed = HashSet::new();
        let mut singles = Vec::new();
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
            let members: Vec<_> = match &rhs.kind {
                n::TypeKind::Union(ms) => ms.iter().collect(),
                _ => vec![rhs],
            };
            let k = members.len();
            if k > i || head_params(&def.head).is_none() {
                continue;
            }
            let fits=members.iter().enumerate().all(|(j,m)| {
                let at=i-k+j;
                !member_of.contains_key(&at) && match &items[at].kind {
                    n::ItemKind::TypeDef(n::TypeDef { name,head,rhs:n::TypeRhs::Data(_),.. }) =>
                        member_name(m)==Some(name.name.as_str()) && matches!(&m.kind,n::TypeKind::Named { args,.. }
                            if args.len()==head.len() && args.iter().zip(head).all(|(a,b)| member_name(a)==member_name(b))),
                    _=>false,
                }
            });
            if fits {
                for j in 0..k {
                    let at = i - k + j;
                    member_of.insert(at, i);
                    let n::ItemKind::TypeDef(member) = &items[at].kind else {
                        unreachable!()
                    };
                    if member.name.name != def.name.name {
                        absorbed.insert(member.name.name.clone());
                        if k == 1 && !matches!(rhs.kind, n::TypeKind::Union(_)) {
                            singles.push((i, member.name.name.clone()));
                        }
                    }
                }
            }
        }
        struct Used(HashSet<String>);
        impl crate::syn3::walk::Visitor for Used {
            fn ty(&mut self, t: &mut n::Type, enter: bool) {
                if enter
                    && let n::TypeKind::Named {
                        module: None, name, ..
                    } = &t.kind
                {
                    self.0.insert(name.name.clone());
                }
            }
        }
        let parents: HashSet<_> = member_of.values().copied().collect();
        let mut used = Used(HashSet::new());
        for (i, item) in items.iter().enumerate() {
            let mut item = item.clone();
            if let n::ItemKind::TypeDef(def) = &mut item.kind {
                if head_params(&def.head).is_some() && !def.alias_pattern {
                    def.head.clear();
                }
                if parents.contains(&i) {
                    def.rhs = n::TypeRhs::Opaque;
                }
            }
            crate::syn3::walk::item(&mut item, &mut used);
        }
        let aliases: Vec<_> = singles
            .into_iter()
            .filter(|(_, member)| used.0.contains(member))
            .map(|(i, _)| i)
            .collect();
        if aliases.is_empty() {
            return (member_of, absorbed);
        }
        plain.extend(aliases);
    }
}

/// What a chain is in the HIR.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChainKind {
    /// One function with no guard and one choice per parameter.
    Plain,
    /// Branches of one position with no written need or result: a function
    /// of `%arg` over a match.
    Clauses,
    /// Several positions, or a written need or result: a function of fresh
    /// parameters over a match.
    Wrapped,
}

pub fn chain_kind(fs: &[n::Function]) -> ChainKind {
    let plain =
        |f: &n::Function| f.guard.is_none() && f.params.iter().all(|p| p.choices.len() == 1);
    if let [f] = fs
        && plain(f)
    {
        return ChainKind::Plain;
    }
    let count = fs[0].params.len();
    let bare = fs.iter().all(|f| f.services.is_none() && f.ret.is_none());
    if count == 1 && bare {
        ChainKind::Clauses
    } else {
        ChainKind::Wrapped
    }
}

/// `a |> F1 | F2` is a match (§2.11): one position per branch.
pub fn is_match(fs: &[n::Function], lisp: bool) -> bool {
    !lisp
        && fs.iter().all(|f| {
            f.params.len() == 1
                && f.services.is_none()
                && f.ret.is_none()
                && f.generics.is_empty()
                && f.params[0].key.is_none()
        })
}

/// `core.never` when the module imports nothing else under the name `core`.
pub fn is_core_never(core_free: bool, e: &n::Expr) -> bool {
    core_free
        && matches!(&e.kind, n::ExprKind::Field { target, name }
            if name.name == "never"
                && matches!(&target.kind, n::ExprKind::Name { module: None, name } if name.name == "core"))
}

pub fn core_free(m: &n::Module) -> bool {
    !m.imports.iter().any(|i| {
        let named = match &i.alias {
            Some(a) => a.name == "core",
            None => i.names.is_empty() && i.path.last().is_some_and(|p| p.name == "core"),
        };
        let library = matches!(i.path.as_slice(), [a, b] if a.name == "i" && b.name == "core");
        named && !library
    })
}

/// The attribute's arguments when all are plain strings (and there is one);
/// `None` when it has no brackets or is written as a call.
pub fn attribute_strings(a: &n::Attribute) -> Option<Vec<(String, Span)>> {
    let items = a.args.as_ref()?;
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
    strings.filter(|s| !s.is_empty())
}

/// The attribute is written as a call: brackets with something other than
/// one or more plain strings.
pub fn attribute_call(a: &n::Attribute) -> bool {
    a.args.is_some() && attribute_strings(a).is_none()
}

/// The parts of `p1 = p2 = …` as today's chain of names around one
/// pattern. With a label, the chain ends in the label of a constructor
/// field.
pub struct AndPlan<'t> {
    /// The names around the inner pattern, the outermost first.
    pub names: Vec<(String, Span)>,
    pub inner: Inner<'t>,
    /// The span of every name layer.
    pub span: Span,
}

pub enum Inner<'t> {
    Pat(&'t n::Pattern),
    /// `label: T`, with the span of the written typed pattern.
    Typed {
        label: &'t n::Ident,
        ty: &'t n::Type,
        span: Span,
    },
    Var(String, Span),
}

pub fn and_plan<'t>(
    parts: &'t [n::Pattern],
    label: Option<&'t n::Ident>,
    span: Span,
) -> AndPlan<'t> {
    let mut names: Vec<(String, Span)> = Vec::new();
    let mut core: Option<&n::Pattern> = None;
    let mut discard = None;
    for p in parts {
        match &p.kind {
            n::PatternKind::Var(s) => names.push((s.clone(), p.span.0)),
            n::PatternKind::Discard(_) => discard = Some(p),
            _ if core.is_some() => {}
            _ => core = Some(p),
        }
    }
    let inner = match (label, core) {
        (Some(l), Some(core)) => match &core.kind {
            n::PatternKind::Typed { binder: b, ty: t } => {
                if b.name != l.name && !b.discard {
                    names.insert(0, (b.name.clone(), b.span.0));
                }
                Inner::Typed {
                    label: l,
                    ty: t,
                    span: core.span.0,
                }
            }
            _ => Inner::Pat(core),
        },
        (Some(l), None) => {
            names.retain(|n| n.0 != l.name);
            Inner::Var(l.name.clone(), l.span.0)
        }
        (None, Some(core)) => Inner::Pat(core),
        (None, None) => match names.pop() {
            Some(last) => Inner::Var(last.0, last.1),
            None => Inner::Pat(discard.take().expect("a conjunction has a part")),
        },
    };
    AndPlan { names, inner, span }
}

/// `key = pattern` in a constructor pattern. `fresh` is the name for
/// `key = _`.
pub fn keyed_plan<'t>(
    l: &'t n::Ident,
    p: &'t n::Pattern,
    fresh: impl FnOnce() -> String,
) -> AndPlan<'t> {
    match &p.kind {
        n::PatternKind::And(parts) => and_plan(parts, Some(l), p.span.0),
        n::PatternKind::Discard(_) => AndPlan {
            names: vec![(fresh(), p.span.0)],
            inner: Inner::Var(l.name.clone(), l.span.0),
            span: p.span.0,
        },
        _ => and_plan(std::slice::from_ref(p), Some(l), p.span.0),
    }
}

/// The type parameters that a binding takes from its functions and its
/// signature, in written order.
pub(super) fn binding_generics(b: &n::Binding, include_typed: bool) -> Vec<&n::Ident> {
    let mut generics: Vec<&n::Ident> = Vec::new();
    if let n::ExprKind::Chain(fs) = &b.value.kind {
        // Branches may share a name; a name written twice in one list is
        // kept twice, so that the resolver reports it (E0226).
        for f in fs {
            let before = generics.len();
            for g in &f.generics {
                if !generics[..before].iter().any(|x| x.name == g.name) {
                    generics.push(g);
                }
            }
        }
    }
    if let Some(f) = typed_fn(b).filter(|_| include_typed) {
        for g in &f.generics {
            if !generics.iter().any(|x| x.name == g.name) {
                generics.push(g);
            }
        }
    }
    generics
}

fn typed_fn(b: &n::Binding) -> Option<&n::FnType> {
    match &b.pattern.kind {
        n::PatternKind::Typed { ty, .. } => match &ty.kind {
            n::TypeKind::Fn(f) => Some(f),
            _ => None,
        },
        _ => None,
    }
}

struct Cx {
    transition: Transition,
    fragment: Option<(u8, Span)>,
    generated_from: Option<u32>,
    out: Pre,
    rows: Vec<Row>,
    fresh: usize,
}

pub fn pre(m: &n::Module, lisp: bool) -> Pre {
    pre_with(m, lisp, None, &Transition::default())
}
pub fn pre_with(
    m: &n::Module,
    lisp: bool,
    generated_from: Option<u32>,
    transition: &Transition,
) -> Pre {
    let mut cx = Cx {
        transition: transition.clone(),
        fragment: None,
        generated_from,
        out: Pre {
            lisp,
            core_free: core_free(m),
            ..Pre::default()
        },
        rows: Vec::new(),
        fresh: 0,
    };
    cx.items(&m.items);
    cx.out
}

/// The same for an expression that stands alone (a macro result): no
/// module around it, so `core.never` is not read as the empty match.
pub fn pre_expr(e: &n::Expr) -> Pre {
    pre_expr_with(e, &Transition::default())
}
pub fn pre_expr_with(e: &n::Expr, transition: &Transition) -> Pre {
    let mut cx = Cx {
        transition: transition.clone(),
        fragment: None,
        generated_from: None,
        out: Pre { ..Pre::default() },
        rows: Vec::new(),
        fresh: 0,
    };
    cx.expr(e);
    cx.out
}

impl Cx {
    fn boundary(&self, kind: u8, span: Span) -> bool {
        self.fragment != Some((kind, span))
            && match kind {
                0 => self.transition.is_expression(span),
                1 => self.transition.is_type(span),
                _ => self.transition.is_pattern(span),
            }
    }
    fn standalone(&mut self, kind: u8, span: Span, run: impl FnOnce(&mut Self)) {
        let mut cx = Self {
            transition: self.transition.clone(),
            fragment: Some((kind, span)),
            generated_from: None,
            out: Pre::default(),
            rows: Vec::new(),
            fresh: 0,
        };
        run(&mut cx);
        self.out.errors.extend(cx.out.errors);
        self.out.args.extend(cx.out.args);
        self.out.services.extend(cx.out.services);
        self.out.fields.extend(cx.out.fields);
        self.out.row_err.extend(cx.out.row_err);
        self.out.type_params.extend(cx.out.type_params);
        self.out.matches.extend(cx.out.matches);
        self.out.never.extend(cx.out.never);
        self.out.functions.extend(cx.out.functions);
    }
    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.out
            .errors
            .push(Diagnostic::new(Code::Unexpected, span, message));
    }

    fn fresh(&mut self, what: &str) -> String {
        self.fresh += 1;
        format!("%{what}{}", self.fresh)
    }

    fn items(&mut self, items: &[n::Item]) {
        let (member_of, absorbed) = if self.transition.is_complete() {
            let mut members = HashMap::new();
            let mut absorbed = HashSet::new();
            for (i, parent) in items.iter().enumerate() {
                if let Some(spans) = self.transition.sum_members(parent.span.0) {
                    for span in spans {
                        if let Some(at) = items.iter().position(|item| item.span.0 == *span) {
                            members.insert(at, i);
                            if let (n::ItemKind::TypeDef(member), n::ItemKind::TypeDef(parent)) =
                                (&items[at].kind, &parent.kind)
                                && member.name.name != parent.name.name
                            {
                                absorbed.insert(member.name.name.clone());
                            }
                        }
                    }
                }
            }
            (members, absorbed)
        } else {
            sums(items)
        };
        for (i, item) in items.iter().enumerate() {
            for a in &item.attributes {
                if attribute_call(a) {
                    self.args(a.args.as_ref().unwrap());
                }
            }
            match &item.kind {
                n::ItemKind::TypeDef(def) => {
                    if member_of.contains_key(&i) {
                        let n::TypeRhs::Data(fields) = &def.rhs else {
                            unreachable!()
                        };
                        self.constructor(fields);
                        continue;
                    }
                    let sum = member_of.values().any(|s| *s == i);
                    self.type_def(def, sum);
                }
                n::ItemKind::Bind(b) => self.binding(b),
                n::ItemKind::Decl { ty: t, .. } => match &t.kind {
                    n::TypeKind::Fn(f) if !f.generics.is_empty() => self.fn_type(f, t.span.0, true),
                    _ => self.ty(t),
                },
                n::ItemKind::Macro { args, .. } => self.args(args),
            }
        }
        self.out.member_of = member_of;
        self.out.absorbed = absorbed;
    }

    fn constructor(&mut self, fields: &[n::TItem]) {
        for f in fields {
            match f {
                n::TItem::Pos(t) | n::TItem::Label(_, t) => self.ty(t),
                n::TItem::Spread(t) => {
                    self.error(t.span.0, "`..` is only written in needs, after `<-`");
                    self.ty(t);
                }
            }
        }
    }

    fn type_def(&mut self, def: &n::TypeDef, sum: bool) {
        let params = head_params(&def.head);
        let custom = |cx: &mut Cx| {
            if params.is_none() {
                cx.error(
                    def.name.span.0,
                    "the head of a data type or an opaque type has only type parameters",
                );
            }
        };
        if sum {
            return custom(self);
        }
        match &def.rhs {
            n::TypeRhs::Opaque => custom(self),
            n::TypeRhs::Data(fields) => {
                self.constructor(fields);
                custom(self)
            }
            n::TypeRhs::Type(t) => {
                self.ty(t);
                let alias_pattern = def.alias_pattern;
                if params.is_none() || alias_pattern {
                    for t in &def.head {
                        self.ty(t);
                    }
                }
            }
        }
    }

    fn binding(&mut self, b: &n::Binding) {
        let typed_fn = match &b.pattern.kind {
            n::PatternKind::Typed { ty, .. } if self.boundary(1, ty.span.0) => None,
            _ => typed_fn(b),
        };
        let generics = binding_generics(b, typed_fn.is_some());
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
            .formals(b.span.0, b.pattern.span.0)
            .map(<[_]>::to_vec);
        let mark = self.rows.len();
        self.rows.extend(generics.iter().map(|g| Row {
            name: g.name.clone(),
            carries: original.as_ref().map_or_else(
                || !with_fail.contains(&g.name),
                |ps| ps.iter().any(|(name, _)| *name == format!("{}%E", g.name)),
            ),
            used: false,
        }));
        match (&b.pattern.kind, typed_fn) {
            (n::PatternKind::Typed { ty: t, .. }, Some(f)) => self.fn_type(f, t.span.0, true),
            _ => self.pattern(&b.pattern),
        }
        match &b.value.kind {
            n::ExprKind::Chain(fs) if !self.boundary(0, b.value.span.0) => {
                self.chain(fs, b.value.span.0, true)
            }
            _ => self.expr(&b.value),
        }
        let mut type_params: Vec<o::Ident> = generics.iter().map(|g| id(g)).collect();
        for row in self.rows.drain(mark..) {
            if row.used {
                type_params.push(o::Ident {
                    name: format!("{}%E", row.name),
                    span: b.span.0,
                });
            }
        }
        if let Some(original) = original {
            type_params = original
                .into_iter()
                .map(|(name, span)| o::Ident { name, span })
                .collect();
        }
        self.out.type_params.insert(addr(b), type_params);
    }

    fn block(&mut self, b: &n::Block) {
        for (i, s) in b.stmts.iter().enumerate() {
            match s {
                n::Stmt::Expr(n::Expr {
                    kind: n::ExprKind::Provide { target, services },
                    ..
                }) if i + 1 == b.stmts.len() => self.provide(target, services),
                n::Stmt::Bind(b) => self.binding(b),
                n::Stmt::Expr(e) => self.expr(e),
            }
        }
    }

    fn provide(&mut self, target: &n::Expr, services: &n::Expr) {
        match &services.kind {
            n::ExprKind::Data(items) => {
                let apart = provide_apart(items);
                for item in items {
                    match item {
                        n::DItem::Pos(e) => {
                            self.expr(e);
                            if apart {
                                let name = self.fresh("service");
                                self.out.services.insert(addr(e), name);
                            }
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
        match &target.kind {
            n::ExprKind::Block(b) => self.block(b),
            n::ExprKind::Provide {
                target: inner,
                services: more,
            } => self.provide(inner, more),
            _ => self.expr(target),
        }
    }

    fn params_of(&mut self, f: &n::Function) {
        let mut fails = 0;
        for s in f.services.iter().flatten() {
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
            self.ty(&s.ty);
        }
    }

    fn written_return(&mut self, ret: Option<Return<'_>>) {
        if let Some(ret) = ret {
            for value in ret.values {
                self.ty(value);
            }
            if let Some(error) = ret.error {
                self.ty(error);
            }
        }
    }

    fn body(&mut self, b: &n::Block) {
        match b.stmts.as_slice() {
            [n::Stmt::Expr(e)] => self.expr(e),
            _ => self.block(b),
        }
    }

    fn keys(&mut self, f: &n::Function) {
        for p in &f.params {
            if let Some(k) = &p.key {
                self.error(
                    k.span.0,
                    "a key on a parameter needs shape groups (stage 9 of syntax three), which this compiler does not have yet",
                );
            }
        }
    }

    fn chain(&mut self, fs: &[n::Function], span: Span, bound: bool) {
        self.out.functions.insert(span);
        for f in fs {
            self.keys(f);
            if !bound && !f.generics.is_empty() {
                self.error(
                    f.generics[0].span.0,
                    "type parameters on a function literal that is not the value of a binding need stage 7 of syntax three, which this compiler does not have yet",
                );
            }
        }
        let count = fs[0].params.len();
        if let Some(other) = fs.iter().find(|f| f.params.len() != count) {
            self.error(
                other.span.0,
                "a chain whose branches have different shapes needs shape groups (stage 9 of syntax three), which this compiler does not have yet",
            );
        }
        match chain_kind(fs) {
            ChainKind::Plain => {
                let f = &fs[0];
                for p in &f.params {
                    self.pattern(&p.choices[0]);
                }
                self.params_of(f);
                self.written_return(function_return(f));
                self.block(&f.body);
            }
            ChainKind::Clauses => {
                for f in fs {
                    self.clause(f, 1);
                }
            }
            ChainKind::Wrapped => {
                if count == 0 {
                    self.error(
                        span,
                        "a chain of branches with no parameter has nothing to choose by",
                    );
                }
                let names: Vec<String> = (0..count).map(|_| self.fresh("arg")).collect();
                self.out.args.insert(addr(&fs[0]), names);
                for f in fs {
                    self.params_of(f);
                }
                self.written_return(chain_return(fs));
                for f in fs {
                    self.clause(f, count);
                }
            }
        }
    }

    fn clause(&mut self, f: &n::Function, _count: usize) {
        for p in &f.params {
            for c in &p.choices {
                self.pattern(c);
            }
        }
        if let Some(g) = &f.guard {
            self.expr(g);
        }
        self.body(&f.body);
    }

    fn args(&mut self, items: &[n::DItem]) {
        for i in items {
            match i {
                n::DItem::Pos(e) | n::DItem::Label(_, e) => self.expr(e),
                n::DItem::Spread(e) => {
                    self.error(e.span.0, "`..` is the first item of a constructor call");
                    self.expr(e);
                }
            }
        }
    }

    fn fields(&mut self, items: &[n::DItem]) {
        for i in items {
            match i {
                n::DItem::Label(_, e) => self.expr(e),
                n::DItem::Pos(e) | n::DItem::Spread(e) => self.error(
                    e.span.0,
                    "data with labels has only labelled items after an optional first `..base` (mixed data needs stage 6 of syntax three)",
                ),
            }
        }
    }

    fn expr(&mut self, e: &n::Expr) {
        if self.boundary(0, e.span.0) {
            self.standalone(0, e.span.0, |cx| cx.expr(e));
            return;
        }
        let span = e.span.0;
        match &e.kind {
            n::ExprKind::Quote(q) => {
                if let Err(error) = crate::syn3::lisp::quote_captures(&q.0, false) {
                    self.out.errors.push(error);
                }
            }
            n::ExprKind::Int(_)
            | n::ExprKind::Float(_)
            | n::ExprKind::String(_)
            | n::ExprKind::Name { .. } => {}
            n::ExprKind::Interp(parts) => {
                for p in parts {
                    if let n::InterpPart::Expr(e) = p {
                        self.expr(e);
                    }
                }
            }
            n::ExprKind::Data(items) => {
                if items.iter().all(|i| matches!(i, n::DItem::Pos(_))) {
                    for i in items {
                        if let n::DItem::Pos(e) = i {
                            self.expr(e);
                        }
                    }
                } else {
                    let rest = match items.split_first() {
                        Some((n::DItem::Spread(b), rest)) => {
                            self.expr(b);
                            rest
                        }
                        _ => items.as_slice(),
                    };
                    self.fields(rest);
                }
            }
            n::ExprKind::List { items, rest } => {
                for i in items {
                    self.expr(i);
                }
                if let Some(r) = rest {
                    self.expr(r);
                }
            }
            n::ExprKind::BitArray(segments) => {
                for s in segments {
                    self.expr(&s.value);
                    self.format(&s.format);
                }
            }
            n::ExprKind::Block(b) => self.block(b),
            n::ExprKind::Chain(fs) => self.chain(fs, span, false),
            n::ExprKind::Call { callee, args }
                if is_core_never(self.out.core_free, callee) && args.len() == 1 =>
            {
                self.out.never.insert(addr(e));
                match &args[0] {
                    n::DItem::Pos(e) | n::DItem::Label(_, e) | n::DItem::Spread(e) => self.expr(e),
                }
            }
            n::ExprKind::Call { callee, args } => match args.split_first() {
                Some((n::DItem::Spread(base), rest)) => {
                    self.expr(callee);
                    self.expr(base);
                    self.fields(rest);
                }
                _ => {
                    self.expr(callee);
                    self.args(args);
                }
            },
            n::ExprKind::Provide { target, services } => self.provide(target, services),
            n::ExprKind::Field { .. } if is_core_never(self.out.core_free, e) => {
                self.out.never.insert(addr(e));
                self.out.functions.insert(span);
            }
            n::ExprKind::Field { target, .. } | n::ExprKind::Index { target, .. } => {
                self.expr(target)
            }
            n::ExprKind::Unary { expr: inner, .. } => self.expr(inner),
            n::ExprKind::Binary { lhs, rhs, .. } => {
                self.expr(lhs);
                self.expr(rhs);
            }
            n::ExprKind::Pipe { lhs, rhs } => match &rhs.kind {
                n::ExprKind::Chain(fs)
                    if is_match(
                        fs,
                        self.out.lisp
                            && !self.generated_from.is_some_and(|start| span.start >= start),
                    ) =>
                {
                    self.out.matches.insert(addr(e));
                    self.expr(lhs);
                    for f in fs {
                        self.clause(f, 1);
                    }
                }
                _ => {
                    self.expr(lhs);
                    self.expr(rhs);
                }
            },
        }
    }

    fn format(&mut self, f: &Option<n::Format>) {
        if let Some(f) = f
            && let Some(a) = &f.arg
        {
            self.expr(a);
        }
    }

    fn pattern(&mut self, p: &n::Pattern) {
        if self.boundary(2, p.span.0) {
            self.standalone(2, p.span.0, |cx| cx.pattern(p));
            return;
        }
        let span = p.span.0;
        match &p.kind {
            n::PatternKind::Quote(q) => match super::quote_captures(q) {
                Ok(captures) => {
                    for (_, capture) in captures.iter() {
                        self.pattern(capture);
                    }
                }
                Err(error) => self.out.errors.push(error),
            },
            n::PatternKind::Var(_)
            | n::PatternKind::Discard(_)
            | n::PatternKind::Int { .. }
            | n::PatternKind::Float { .. }
            | n::PatternKind::String(_)
            | n::PatternKind::Prefix { .. } => {}
            n::PatternKind::Typed { ty: t, .. } => self.ty(t),
            n::PatternKind::Ctor { args, .. } => {
                for item in args.iter().flatten() {
                    match item {
                        n::DPItem::Pos(p) => self.pattern(p),
                        n::DPItem::Key(l, p) => self.keyed(l, p),
                        n::DPItem::Rest(None) => {}
                        n::DPItem::Rest(Some(b)) => self.error(
                            b.span.0,
                            "a named rest `..name` in a pattern needs stage 6 of syntax three, which this compiler does not have yet",
                        ),
                    }
                }
            }
            n::PatternKind::Data(items) => {
                for item in items {
                    match item {
                        n::DPItem::Pos(p) => self.pattern(p),
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
            }
            n::PatternKind::List { items, .. } => {
                for i in items {
                    self.pattern(i);
                }
            }
            n::PatternKind::BitArray(segments) => {
                for s in segments {
                    self.pattern(&s.value);
                    self.format(&s.format);
                }
            }
            n::PatternKind::And(parts) => self.and(parts, None),
        }
    }

    fn and(&mut self, parts: &[n::Pattern], label: Option<&n::Ident>) {
        let mut core = false;
        let mut core_at = None;
        for p in parts {
            match &p.kind {
                n::PatternKind::Var(_) => {}
                n::PatternKind::Discard(_) => {}
                _ if core => self.error(
                    p.span.0,
                    "two patterns joined by `=` that are not names need stage 9 of syntax three, which this compiler does not have yet",
                ),
                _ => {
                    self.pattern(p);
                    core = true;
                    core_at = Some(p);
                }
            }
        }
        if let (Some(_), Some(core)) = (label, core_at)
            && !matches!(core.kind, n::PatternKind::Typed { .. })
        {
            self.error(
                core.span.0,
                "a pattern inside a labelled field needs stage 4 of syntax three, which this compiler does not have yet; bind the field to a name and match the name",
            );
        }
    }

    fn keyed(&mut self, l: &n::Ident, p: &n::Pattern) {
        match &p.kind {
            n::PatternKind::And(parts) => self.and(parts, Some(l)),
            n::PatternKind::Discard(_) => {
                let name = self.fresh("field");
                self.out.fields.insert(addr(p), name);
            }
            _ => self.and(std::slice::from_ref(p), Some(l)),
        }
    }

    fn needs_row(&mut self, f: &n::FnType) {
        let mut fail = false;
        let mut row: Option<String> = None;
        let mut need = |cx: &mut Cx, t: &n::Type, spread: bool| {
            if let n::TypeKind::Named {
                module: None,
                name,
                args,
            } = &t.kind
            {
                if name.name == "Fail" && args.len() == 1 && !spread {
                    if fail {
                        cx.error(t.span.0, "a need has one `Fail` entry: write `Fail(A | B)`");
                    }
                    fail = true;
                    cx.ty(&args[0]);
                    return;
                }
                if spread && args.is_empty() {
                    row = Some(name.name.clone());
                }
            }
            cx.ty(t);
        };
        match f.services.as_ref().map(|s| (&s.kind, s)) {
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
            Some((_, t)) => need(self, t, true),
        }
        if !fail
            && let Some(row) = row
            && let Some(found) = self.rows.iter_mut().rev().find(|r| r.name == row)
            && found.carries
        {
            found.used = true;
            self.out.row_err.insert(addr(f), row);
        }
    }

    fn fn_type(&mut self, f: &n::FnType, _span: Span, generics_taken: bool) {
        if !generics_taken && !f.generics.is_empty() {
            self.error(
                f.generics[0].span.0,
                "type parameters on a function type that is not the type of a binding need stage 7 of syntax three, which this compiler does not have yet",
            );
        }
        match &f.param.kind {
            n::TypeKind::Data(items) => {
                for item in items {
                    match item {
                        n::TItem::Pos(t) | n::TItem::Label(_, t) => self.ty(t),
                        n::TItem::Spread(t) => {
                            self.error(t.span.0, "`..` is only written in needs, after `<-`")
                        }
                    }
                }
            }
            _ => self.ty(&f.param),
        }
        self.needs_row(f);
        self.ty(&f.ret);
    }

    fn ty(&mut self, t: &n::Type) {
        if self.boundary(1, t.span.0) {
            self.standalone(1, t.span.0, |cx| cx.ty(t));
            return;
        }
        let span = t.span.0;
        match &t.kind {
            n::TypeKind::Lit(_) | n::TypeKind::ValueOf { .. } | n::TypeKind::Hole => {}
            n::TypeKind::Named { args, .. } | n::TypeKind::Call { args, .. } => {
                for a in args {
                    self.ty(a);
                }
            }
            n::TypeKind::Data(items) => {
                if items.iter().all(|i| matches!(i, n::TItem::Pos(_))) && !items.is_empty() {
                    for i in items {
                        if let n::TItem::Pos(t) = i {
                            self.ty(t);
                        }
                    }
                } else {
                    for item in items {
                        match item {
                            n::TItem::Label(_, t) => self.ty(t),
                            n::TItem::Pos(t) | n::TItem::Spread(t) => self.error(
                                t.span.0,
                                "a data type with labels has only labelled items (mixed data needs stage 6 of syntax three)",
                            ),
                        }
                    }
                }
            }
            n::TypeKind::Fn(f) => self.fn_type(f, span, false),
            n::TypeKind::Union(parts) | n::TypeKind::Inter(parts) | n::TypeKind::Diff(parts) => {
                for p in parts {
                    self.ty(p);
                }
            }
        }
    }
}

/// Several services of which one is not a name: each is bound to a fresh
/// local first, so that no service sees another (§2.10.5).
pub fn provide_apart(items: &[n::DItem]) -> bool {
    items.len() > 1
        && items.iter().any(|i| {
            !matches!(
                i,
                n::DItem::Pos(n::Expr {
                    kind: n::ExprKind::Name { .. },
                    ..
                })
            )
        })
}

pub fn pre_pattern(pattern: &n::Pattern) -> Pre {
    let mut cx = Cx {
        transition: Default::default(),
        fragment: None,
        generated_from: None,
        out: Pre { ..Pre::default() },
        rows: Vec::new(),
        fresh: 0,
    };
    cx.pattern(pattern);
    cx.out
}
pub fn pre_type(ty: &n::Type) -> Pre {
    let mut cx = Cx {
        transition: Default::default(),
        fragment: None,
        generated_from: None,
        out: Pre::default(),
        rows: Vec::new(),
        fresh: 0,
    };
    cx.ty(ty);
    cx.out
}

pub fn pre_items(items: &[n::Item]) -> Pre {
    let mut cx = Cx {
        transition: Default::default(),
        fragment: None,
        generated_from: None,
        out: Pre::default(),
        rows: Vec::new(),
        fresh: 0,
    };
    cx.items(items);
    cx.out
}
