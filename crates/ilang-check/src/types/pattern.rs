//! Patterns: pattern types, fitting (§4.8, after rule 14), "may fail"
//! (§7.3), and `case` by places (rule 11).

use super::infer::{Checker, Scheme};
use super::{Head, Ty};
use crate::code;
use crate::hir::*;
use ilang_syntax::span::Span;
use std::collections::HashMap;

/// A catch-all is a name, a discard, or `x = q` where `q` is one (§4.8).
pub(crate) fn catch_all(p: &Pattern) -> bool {
    match &p.kind {
        PatternKind::Var(_) | PatternKind::Discard => true,
        PatternKind::Named { pattern, .. } => catch_all(pattern),
        _ => false,
    }
}

/// The names a pattern binds.
pub(crate) fn binders(p: &Pattern, out: &mut Vec<VarId>) {
    match &p.kind {
        PatternKind::Var(b) => out.push(b.var),
        PatternKind::Typed { binder, .. } => out.extend(binder.as_ref().map(|b| b.var)),
        PatternKind::Named { binder, pattern } => {
            out.push(binder.var);
            binders(pattern, out);
        }
        PatternKind::Prefix { rest, .. } => out.extend(rest.as_ref().map(|b| b.var)),
        PatternKind::Constructor { args: ps, .. }
        | PatternKind::Tuple(ps)
        | PatternKind::Error(ps) => ps.iter().for_each(|q| binders(q, out)),
        PatternKind::List { items, rest } => {
            items.iter().for_each(|q| binders(q, out));
            if let Some(Some(b)) = rest {
                out.push(b.var);
            }
        }
        PatternKind::BitArray(segs) => segs.iter().for_each(|s| binders(&s.value, out)),
        _ => {}
    }
}

/// §7.3, by the form of the pattern: a catch-all, `x: T` (fitting makes
/// the value a `T`), a tuple of parts that cannot fail, and a constructor
/// of a type with one constructor whose parts cannot fail, cannot fail.
/// Every other pattern may fail.
///
/// TODO(M3b): exact, with the type of the value: `1 | _: Int` and a
/// literal for a type with one value cannot fail either.
pub(crate) fn may_fail(p: &Program, q: &Pattern) -> bool {
    match &q.kind {
        PatternKind::Var(_)
        | PatternKind::Discard
        | PatternKind::Typed { .. }
        | PatternKind::Error(_) => false,
        PatternKind::Named { pattern, .. } => may_fail(p, pattern),
        PatternKind::Tuple(ps) => ps.iter().any(|x| may_fail(p, x)),
        PatternKind::Constructor { ctor, args, .. } => {
            let one = matches!(&p.ty(ctor.ty).body, TypeBody::Custom(cs) if cs.len() == 1);
            !one || args.iter().any(|x| may_fail(p, x))
        }
        _ => true,
    }
}

/// The label of a part of a constructor pattern (§7.1): the last name in
/// the chain of `=` at its top.
fn part_label(p: &Pattern) -> Option<&str> {
    match &p.kind {
        PatternKind::Var(b) => Some(&b.name),
        PatternKind::Typed {
            binder: Some(b), ..
        } => Some(&b.name),
        PatternKind::Named { binder, pattern } => part_label(pattern).or(Some(&binder.name)),
        _ => None,
    }
}

/// A place of a `case` (rule 11).
struct Place {
    /// `B`: the type of the values at this place.
    ty: Ty,
    /// The pattern types of the patterns here that are not catch-alls.
    pats: Vec<Ty>,
    /// The arguments shared by the patterns here with one type constructor.
    heads: Vec<(Head, Vec<Ty>)>,
    /// A clause without a guard has a catch-all here.
    catch_here: bool,
    parent: Option<usize>,
    children: Vec<(Step, usize)>,
}

/// The step from a place to a place below it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Field(CtorRef, u32),
    Part(u32, u32),
    Item(u32),
    Rest,
}

impl Checker<'_> {
    /// Gives a name of a pattern its type. In a `case`, the choices of a
    /// clause bind one name together, so it gets the union of their types.
    fn bind(&mut self, b: &Binder, t: Ty, in_case: bool) {
        if in_case {
            if let Some(s) = self.locals.get(&b.var).cloned() {
                self.sub(&t, &s.ty, b.span);
                return;
            }
            let v = self.s.fresh();
            self.sub(&t, &v, b.span);
            self.locals.insert(b.var, Scheme::mono(v));
            return;
        }
        self.locals.insert(b.var, Scheme::mono(t));
    }

    /// Gives every name of a catch-all the type `t`.
    fn bind_catch_all(&mut self, p: &Pattern, t: &Ty, in_case: bool) {
        match &p.kind {
            PatternKind::Var(b) => self.bind(b, t.clone(), in_case),
            PatternKind::Named { binder, pattern } => {
                self.bind(binder, t.clone(), in_case);
                self.bind_catch_all(pattern, t, in_case);
            }
            _ => {}
        }
    }

    /// Fits `q` to `b` (§4.8): a catch-all gives its names `b`; any other
    /// pattern needs `b <: Q`, with its parts fitted to the parts of `Q`.
    pub fn fit(&mut self, q: &Pattern, b: &Ty) {
        if catch_all(q) {
            self.bind_catch_all(q, b, false);
        } else {
            let t = self.pat(q);
            self.sub(b, &t, q.span);
        }
    }

    /// The type of a parameter (rule 4): a new variable for a catch-all,
    /// and else the pattern type.
    pub fn param(&mut self, q: &Pattern) -> Ty {
        if catch_all(q) {
            let v = self.s.fresh();
            self.bind_catch_all(q, &v, false);
            v
        } else {
            self.pat(q)
        }
    }

    /// The pattern type `Q` of a pattern that is not a catch-all; its parts
    /// are fitted to the parts of `Q`.
    fn pat(&mut self, q: &Pattern) -> Ty {
        match &q.kind {
            PatternKind::Var(_) | PatternKind::Discard => self.param(q),
            PatternKind::Named { binder, pattern } => {
                let t = self.param(pattern);
                self.bind(binder, t.clone(), false);
                t
            }
            PatternKind::Typed { binder, ty } => {
                let t = self.conv(ty, &HashMap::new());
                if let Some(b) = binder {
                    self.written.insert(b.var, ty.clone());
                    self.bind(b, t.clone(), false);
                }
                t
            }
            PatternKind::Constructor { ctor, args, spread } => {
                let targs = self.fresh_args(ctor.ty);
                let fields = self.ctor_fields(*ctor, &targs);
                for (arg, i) in self.match_fields(q.span, &fields, args, *spread) {
                    let ft = fields[i].1.clone();
                    self.fit(arg, &ft);
                }
                Ty::named(TypeRef::Custom(ctor.ty), targs)
            }
            PatternKind::Tuple(ps) => Ty::tuple(ps.iter().map(|x| self.param(x)).collect()),
            PatternKind::List { items, rest } => {
                let e = self.s.fresh();
                for x in items {
                    self.fit(x, &e);
                }
                let list = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![e]);
                if let Some(Some(b)) = rest {
                    self.bind(b, list.clone(), false);
                }
                list
            }
            _ => self.leaf(q, false),
        }
    }

    /// The pattern types of literals, prefixes, and bit arrays, with their
    /// names bound.
    fn leaf(&mut self, q: &Pattern, in_case: bool) -> Ty {
        let prim = |b| Ty::named(TypeRef::Builtin(b), Vec::new());
        match &q.kind {
            PatternKind::Int { .. } => prim(BuiltinType::Int),
            PatternKind::Float { .. } => prim(BuiltinType::Float),
            PatternKind::String(_) => prim(BuiltinType::String),
            PatternKind::Prefix { rest, .. } => {
                if let Some(b) = rest {
                    self.bind(b, prim(BuiltinType::String), in_case);
                }
                prim(BuiltinType::String)
            }
            PatternKind::BitArray(segs) => {
                for s in segs {
                    let ft = self.format(s.format.as_ref(), s.span);
                    if catch_all(&s.value) {
                        self.bind_catch_all(&s.value, &ft, in_case);
                    } else {
                        let t = self.pat(&s.value);
                        self.sub(&ft, &t, s.value.span);
                    }
                }
                prim(BuiltinType::BitArray)
            }
            _ => {
                if let PatternKind::Error(ps) = &q.kind {
                    for x in ps {
                        let v = self.s.fresh();
                        self.fit(x, &v);
                    }
                }
                self.s.fresh()
            }
        }
    }

    /// §7.1: the parts of a constructor pattern and the fields they are
    /// matched to: by position to the fields without labels, then by label.
    fn match_fields<'p>(
        &mut self,
        span: Span,
        fields: &[(Option<String>, Ty)],
        args: &'p [Pattern],
        spread: bool,
    ) -> Vec<(&'p Pattern, usize)> {
        let unlabeled: Vec<usize> = (0..fields.len())
            .filter(|i| fields[*i].0.is_none())
            .collect();
        let mut out: Vec<(&Pattern, usize)> = Vec::new();
        for (j, a) in args.iter().enumerate() {
            let field = if j < unlabeled.len() {
                Some(unlabeled[j])
            } else {
                match part_label(a) {
                    Some(l) => match fields.iter().position(|f| f.0.as_deref() == Some(l)) {
                        Some(i) => Some(i),
                        None => {
                            self.error(
                                code::PATTERN_FIELDS,
                                a.span,
                                format!("the constructor has no field with the label `{l}` (§7.1)"),
                            );
                            None
                        }
                    },
                    None => {
                        self.error(
                            code::PATTERN_FIELDS,
                            a.span,
                            "this part is matched by label, so it needs a name at its top that is the label of a field (§7.1)",
                        );
                        None
                    }
                }
            };
            if let Some(i) = field {
                if out.iter().any(|o| o.1 == i) {
                    self.error(
                        code::PATTERN_FIELDS,
                        a.span,
                        "this field is matched more than once (§7.1)",
                    );
                } else {
                    out.push((a, i));
                }
            }
        }
        if !spread && out.len() < fields.len() && out.len() == args.len() {
            self.error(
                code::PATTERN_FIELDS,
                span,
                format!(
                    "the constructor has {} field{}, and every one must be matched, or `..` must leave the rest out (§7.1)",
                    fields.len(),
                    if fields.len() == 1 { "" } else { "s" }
                ),
            );
        }
        out
    }

    // ---- case (rule 11) ----

    pub fn case(&mut self, subject: &Expr, clauses: &[Clause], span: Span) -> Ty {
        let a = self.infer(subject);
        let mut places = vec![Place {
            ty: a,
            pats: Vec::new(),
            heads: Vec::new(),
            catch_here: false,
            parent: None,
            children: Vec::new(),
        }];
        for c in clauses {
            for p in &c.patterns {
                self.visit(p, 0, c.guard.is_none(), &mut places);
            }
        }
        // `B <: P`, or `B <: P | r - P` when a clause without a guard has a
        // catch-all at the place or above it.
        let mut slack = vec![false; places.len()];
        for i in 0..places.len() {
            slack[i] = places[i].catch_here || places[i].parent.is_some_and(|p| slack[p]);
            let pats = std::mem::take(&mut places[i].pats);
            let p = self.union(pats, span);
            let want = if slack[i] {
                let mut members = Vec::new();
                super::solve::flatten_union(std::slice::from_ref(&p), &mut members);
                let r = self.s.fresh();
                let rest = self.s.diff(r, members.clone());
                members.push(rest);
                Ty::Union(members)
            } else {
                p
            };
            let b = places[i].ty.clone();
            self.sub(&b, &want, if i == 0 { subject.span } else { span });
        }
        let r = self.s.fresh();
        for c in clauses {
            if let Some(g) = &c.guard {
                let t = self.infer(g);
                let want = self.bool_ty();
                self.sub(&t, &want, g.span);
            }
            let t = self.infer(&c.body);
            self.sub(&t, &r, c.body.span);
        }
        r
    }

    /// The place below `at` by `step`, with the type `ty` when it is new.
    fn child(&mut self, places: &mut Vec<Place>, at: usize, step: Step, ty: Ty) -> usize {
        if let Some((_, i)) = places[at].children.iter().find(|c| c.0 == step) {
            return *i;
        }
        places.push(Place {
            ty,
            pats: Vec::new(),
            heads: Vec::new(),
            catch_here: false,
            parent: Some(at),
            children: Vec::new(),
        });
        let i = places.len() - 1;
        places[at].children.push((step, i));
        i
    }

    /// The arguments shared at a place by the patterns with one head.
    fn shared(&mut self, places: &mut [Place], at: usize, h: Head, n: usize) -> Vec<Ty> {
        if let Some((_, args)) = places[at].heads.iter().find(|x| x.0 == h) {
            return args.clone();
        }
        let args: Vec<Ty> = (0..n).map(|_| self.s.fresh()).collect();
        places[at].heads.push((h, args.clone()));
        args
    }

    /// Puts a clause pattern at the place `at`, binds its names, and gives
    /// back its pattern type.
    fn visit(&mut self, q: &Pattern, at: usize, unguarded: bool, places: &mut Vec<Place>) -> Ty {
        if catch_all(q) {
            // TODO(M3b): rule 12 narrows a catch-all to `B - H`.
            let b = places[at].ty.clone();
            self.bind_catch_all(q, &b, true);
            if unguarded {
                places[at].catch_here = true;
            }
            return b;
        }
        let t = match &q.kind {
            PatternKind::Named { binder, pattern } => {
                let t = self.visit(pattern, at, unguarded, places);
                self.bind(binder, t.clone(), true);
                return t;
            }
            PatternKind::Typed { binder, ty } => {
                let t = self.conv(ty, &HashMap::new());
                let mut members = Vec::new();
                super::solve::flatten_union(std::slice::from_ref(&t), &mut members);
                if members
                    .iter()
                    .any(|m| m.head().is_none() && *m != Ty::Never)
                {
                    self.error(
                        code::CASE_TYPE_VAR,
                        ty.span,
                        "in a `case`, `x: T` tests the type constructor, so `T` may not have a type variable as a member (§7.1)",
                    );
                }
                if let Some(b) = binder {
                    self.bind(b, t.clone(), true);
                }
                t
            }
            PatternKind::Constructor { ctor, args, spread } => {
                let h = Head::Named(TypeRef::Custom(ctor.ty));
                let n = self.p.ty(ctor.ty).params.len();
                let targs = self.shared(places, at, h, n);
                let fields = self.ctor_fields(*ctor, &targs);
                for (arg, i) in self.match_fields(q.span, &fields, args, *spread) {
                    let c = self.child(
                        places,
                        at,
                        Step::Field(*ctor, i as u32),
                        fields[i].1.clone(),
                    );
                    self.visit(arg, c, unguarded, places);
                }
                Ty::named(TypeRef::Custom(ctor.ty), targs)
            }
            PatternKind::Tuple(ps) => {
                let n = ps.len() as u32;
                let parts = self.shared(places, at, Head::Tuple(n), ps.len());
                for (i, x) in ps.iter().enumerate() {
                    let c = self.child(places, at, Step::Part(n, i as u32), parts[i].clone());
                    self.visit(x, c, unguarded, places);
                }
                Ty::tuple(parts)
            }
            PatternKind::List { items, rest } => {
                let h = Head::Named(TypeRef::Builtin(BuiltinType::List));
                let e = self.shared(places, at, h, 1).remove(0);
                let list = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![e.clone()]);
                for (i, x) in items.iter().enumerate() {
                    let c = self.child(places, at, Step::Item(i as u32), e.clone());
                    self.visit(x, c, unguarded, places);
                }
                if let Some(r) = rest {
                    let c = self.child(places, at, Step::Rest, list.clone());
                    if let Some(b) = r {
                        self.bind(b, list.clone(), true);
                    }
                    if unguarded {
                        places[c].catch_here = true;
                    }
                }
                list
            }
            _ => self.leaf(q, true),
        };
        places[at].pats.push(t.clone());
        t
    }
}
