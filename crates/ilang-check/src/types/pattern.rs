//! Patterns: pattern types, fitting (§4.8, after rule 14), "may fail"
//! (§7.3), `case` by places (rule 11), narrowing (rule 12), coverage and
//! clauses that are never used (§7.4), and known constructors (§7.5).

use super::coverage::{self, Col, Ctor, Pat};
use super::infer::{Checker, Scheme};
use super::known;
use super::{FnTy, Head, Ty};
use crate::code;
use crate::hir::*;
use ilang_syntax::span::Span;
use rustc_hash::FxHashSet as HashSet;
use std::collections::HashMap;

const LITERAL_CANDIDATES: bool = true;

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
        PatternKind::Quote { captures, .. } => captures.iter().for_each(|(_, p)| binders(p, out)),
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

/// The label of a part of a constructor pattern (§7.1): the last name in
/// the chain of `=` at its top.
pub(crate) fn part_label(p: &Pattern) -> Option<&str> {
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
    /// The known type of the values at this place (§4.8).
    known: Ty,
    /// The pattern types of the patterns here that are not catch-alls.
    pats: Vec<Ty>,
    /// The arguments shared by the patterns here with one type constructor.
    heads: Vec<(Head, Vec<Ty>)>,
    /// A clause without a guard has a catch-all here.
    catch_here: bool,
    /// Earliest item index covered by an unguarded list rest.
    rest_from: Option<u32>,
    parent: Option<usize>,
    children: Vec<(Step, usize)>,
    /// The type constructors of the patterns here that are not catch-alls:
    /// the clause, whether it has no guard, the head, and its number of
    /// arguments.
    seen: Vec<(usize, bool, Head, usize)>,
    scalar_seen: Vec<(usize, bool, Ty, Pat)>,
    shared: bool,
}

/// A catch-all in a clause pattern, to be narrowed (rule 12).
struct Catch<'p> {
    pattern: &'p Pattern,
    clause: usize,
    choice: usize,
    place: usize,
}

/// A `case` whose coverage (§7.4) is checked when the types of its group
/// are found.
pub(crate) struct PendingCase {
    pub module: ModuleId,
    pub subject: Ty,
    pub span: Span,
    /// Each clause: its choices, whether it has a guard, and its place.
    pub clauses: Vec<(Pat, bool, Span)>,
    pub held: Vec<(super::solve::Failure, Span)>,
}

/// What a clause pass needs while it visits patterns.
struct Visit<'p> {
    fitted: bool,
    clause: usize,
    choice: usize,
    unguarded: bool,
    catches: Vec<Catch<'p>>,
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
        self.fit_in(q, b, false);
    }

    fn fit_in(&mut self, q: &Pattern, b: &Ty, in_case: bool) {
        if catch_all(q) {
            self.bind_catch_all(q, b, in_case);
        } else {
            let t = self.pat(q, in_case);
            self.sub(b, &t, q.span);
        }
    }

    /// The type of a parameter (rule 4): a new variable for a catch-all,
    /// and else the pattern type.
    pub fn param(&mut self, q: &Pattern) -> Ty {
        self.param_in(q, false)
    }

    fn param_in(&mut self, q: &Pattern, in_case: bool) -> Ty {
        if catch_all(q) {
            let v = self.s.fresh();
            self.bind_catch_all(q, &v, in_case);
            v
        } else {
            self.pat(q, in_case)
        }
    }

    /// The pattern type `Q` of a pattern that is not a catch-all; its parts
    /// are fitted to the parts of `Q`.
    fn pat(&mut self, q: &Pattern, in_case: bool) -> Ty {
        match &q.kind {
            PatternKind::Var(_) | PatternKind::Discard => self.param_in(q, in_case),
            PatternKind::Named { binder, pattern } => {
                let t = self.param_in(pattern, in_case);
                self.bind(binder, t.clone(), in_case);
                self.note_known(binder, pattern);
                t
            }
            PatternKind::Typed { binder, ty } => {
                let t = self.conv(ty, &HashMap::new());
                if let Some(b) = binder {
                    self.written.insert(b.var, ty.clone());
                    self.bind(b, t.clone(), in_case);
                }
                t
            }
            PatternKind::Constructor { ctor, args, spread } => {
                let targs = self.fresh_args(ctor.ty);
                let fields = self.ctor_fields(*ctor, &targs);
                for (arg, i) in self.match_fields(q.span, &fields, args, *spread) {
                    let ft = fields[i].1.clone();
                    self.fit_in(arg, &ft, in_case);
                }
                Ty::named(TypeRef::Custom(ctor.ty), targs)
            }
            PatternKind::Tuple(ps) => {
                Ty::tuple(ps.iter().map(|x| self.param_in(x, in_case)).collect())
            }
            PatternKind::List { items, rest } => {
                let e = self.s.fresh();
                for x in items {
                    self.fit_in(x, &e, in_case);
                }
                let list = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![e]);
                if let Some(Some(b)) = rest {
                    self.bind(b, list.clone(), in_case);
                }
                list
            }
            _ => self.leaf(q, in_case, false),
        }
    }

    /// The pattern types of literals, prefixes, and bit arrays, with their
    /// names bound.
    fn leaf(&mut self, q: &Pattern, in_case: bool, literal: bool) -> Ty {
        let prim = |b| Ty::named(TypeRef::Builtin(b), Vec::new());
        match &q.kind {
            PatternKind::Quote {
                syntax, captures, ..
            } => {
                let one = Ty::named(TypeRef::Custom(*syntax), Vec::new());
                for (many, pattern) in captures {
                    let ty = if *many {
                        Ty::named(TypeRef::Builtin(BuiltinType::List), vec![one.clone()])
                    } else {
                        one.clone()
                    };
                    if catch_all(pattern) {
                        self.bind_catch_all(pattern, &ty, in_case);
                    } else {
                        let actual = self.pat(pattern, in_case);
                        self.sub(&ty, &actual, pattern.span);
                    }
                }
                one
            }
            PatternKind::Int { negative, value } if literal => super::known::scalar_literal(
                BuiltinType::Int,
                super::lits::int_text(*negative, value),
            ),
            PatternKind::String(s) if literal => {
                super::known::scalar_literal(BuiltinType::String, s.clone())
            }
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
                self.check_bit_pattern(segs);
                for s in segs {
                    let ft = self.format(s.format.as_ref(), s.span);
                    if catch_all(&s.value) {
                        self.bind_catch_all(&s.value, &ft, in_case);
                    } else {
                        let t = self.pat(&s.value, in_case);
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

    /// §8.4: a string format in a pattern needs a string literal, and
    /// `bits` or `bytes` with no length may only be the last segment.
    fn check_bit_pattern(&mut self, segs: &[Segment<Pattern>]) {
        for (i, seg) in segs.iter().enumerate() {
            let Some(f) = &seg.format else { continue };
            let name = f.name.name.as_str();
            let string = matches!(name, "utf8" | "utf16be" | "utf16le" | "utf32be" | "utf32le");
            if string && !matches!(seg.value.kind, PatternKind::String(_)) {
                self.error(
                    code::BIT_PATTERN,
                    seg.value.span,
                    format!("the format `{name}` in a pattern needs a string literal, which gives the bits to match (§8.4)"),
                );
            }
            if matches!(name, "bits" | "bytes") && f.arg.is_none() && i + 1 < segs.len() {
                self.error(
                    code::BIT_PATTERN,
                    f.name.span,
                    format!("`{name}` with no length takes all the bits that are left, so it may only be the last segment (§8.4)"),
                );
            }
        }
    }

    /// §7.1: the parts of a constructor pattern and the fields they are
    /// matched to: by position to the fields without labels, then by label.
    pub fn match_fields<'p>(
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
        let mut indices = Vec::new();
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
            indices.push(field.map_or(u32::MAX, |i| i as u32));
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
        self.out.pattern_fields.insert((self.module, span), indices);
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

    /// §7.3: `q` may fail for the type `t` of the value, when some value of
    /// `t` does not match it. Only used after `q` is fitted to `t`.
    pub fn may_fail(&mut self, q: &Pattern, t: &Ty) -> bool {
        if catch_all(q) {
            return false;
        }
        let pat = coverage::fit_pat(self.p, q);
        self.useful(&[vec![pat]], &[Pat::Wild(None)], &[Col::of(t.clone())])
            .is_some()
    }

    /// A member that is a custom type with no constructors (§4.3).
    fn no_constructors(&self, m: &Ty) -> bool {
        matches!(m, Ty::Con(Head::Named(TypeRef::Custom(id)), _, _)
            if matches!(&self.p.ty(*id).body, TypeBody::Custom(cs) if cs.is_empty()))
    }

    /// `may_fail` for the pattern of a binding or a parameter, kept for the
    /// code generator (§6.10, §6.11).
    pub fn binding_may_fail(&mut self, q: &Pattern, t: &Ty) -> bool {
        let fails = self.may_fail(q, t);
        self.out.may_fail.insert((self.module, q.span), fails);
        fails
    }

    /// §7.5: `x = C(…)` in a binding or a parameter makes `x` known to be
    /// made by `C`. In a `case`, see `clause_known`.
    fn note_known(&mut self, binder: &Binder, inner: &Pattern) {
        if let Some(c) = top_ctor(inner) {
            self.known.insert(binder.var, c);
        }
    }

    /// §7.5 for the choices of a `case` clause: `x` is known to be made by
    /// `C` only when every choice binds `x` through `x = C(…)`. A choice
    /// that binds `x` in another way (`x`, `x: T`, `x = _`) may give it a
    /// value made by another constructor.
    fn clause_known(&mut self, choices: &[Pattern]) {
        let mut each: Vec<Vec<(VarId, Option<CtorRef>)>> = Vec::new();
        for p in choices {
            let mut out = Vec::new();
            ctor_binders(p, &mut out);
            each.push(out);
        }
        let Some((first, rest)) = each.split_first() else {
            return;
        };
        for (x, c) in first {
            let Some(c) = c else { continue };
            let all = rest
                .iter()
                .all(|o| o.iter().any(|(y, d)| y == x && *d == Some(*c)));
            if all {
                self.known.insert(*x, *c);
            }
        }
    }

    // ---- case (rules 11 and 12) ----

    /// Rule 11, and the known type of a `case` (§4.8): the union of the
    /// known types of its clause expressions, which have the expected type
    /// of the `case`.
    pub fn case(
        &mut self,
        subject: &Expr,
        clauses: &[Clause],
        span: Span,
        expected: &Ty,
    ) -> (Ty, Ty) {
        let fitted = clauses.len() == 1 && clauses[0].guard.is_none();
        let (a, subject_known) = self.infer_k(subject, &Ty::Top);
        let mut places = vec![Place {
            ty: a.clone(),
            known: subject_known,
            pats: Vec::new(),
            heads: Vec::new(),
            catch_here: false,
            rest_from: None,
            parent: None,
            children: Vec::new(),
            seen: Vec::new(),
            scalar_seen: Vec::new(),
            shared: false,
        }];
        let mut rows: Vec<Vec<Pat>> = Vec::new();
        let mut v = Visit {
            fitted,
            clause: 0,
            choice: 0,
            unguarded: true,
            catches: Vec::new(),
        };
        for (ci, c) in clauses.iter().enumerate() {
            v.clause = ci;
            v.unguarded = c.guard.is_none();
            let mut choices = Vec::new();
            for (j, p) in c.patterns.iter().enumerate() {
                v.choice = j;
                let (_, pat) = self.visit(p, 0, &mut v, &mut places);
                choices.push(pat);
            }
            self.clause_known(&c.patterns);
            rows.push(choices);
        }
        // `B <: P`, or `B <: P | r - P` when a clause without a guard has a
        // catch-all at the place or above it.
        let mut held = Vec::new();
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
            let at = if i == 0 { subject.span } else { span };
            if fitted {
                self.sub(&b, &want, at);
            } else {
                held.extend(self.sub_case(&b, &want, at));
            }
        }
        // Rule 12: each catch-all gets `B - H`, and its known type less the
        // members that are taken (§4.8).
        for k in 0..v.catches.len() {
            let c = &v.catches[k];
            let (pattern, ci, choice, place) = (c.pattern, c.clause, c.choice, c.place);
            let (t, taken) = self.narrow(&rows, clauses, ci, choice, k as u32, &places[place]);
            self.bind_catch_all(pattern, &t, true);
            let known = match taken {
                None => Ty::Never,
                Some(taken) => {
                    super::display::known_diff(&self.s, places[place].known.clone(), taken)
                }
            };
            self.fit_known(pattern, &known, true);
        }
        // §7.5: inside a clause `C(…)`, the name of the subject is known to
        // be made by `C`.
        let subject_var = match &subject.kind {
            ExprKind::Name(ValueRef::Local(x)) => Some(*x),
            ExprKind::Name(ValueRef::Def(d)) => match self.p.def(*d).kind {
                DefKind::Binding { var, .. } => Some(var),
                DefKind::External { .. } => None,
            },
            _ => None,
        };
        let r = self.s.fresh();
        let mut knowns = Vec::new();
        for c in clauses {
            // Every choice of the clause has `C` at its top.
            let known = subject_var.filter(|_| !fitted).and_then(|x| {
                let mut ctors = c.patterns.iter().map(top_ctor);
                let first = ctors.next()??;
                ctors.all(|o| o == Some(first)).then_some((x, first))
            });
            // Inside the clause, the value of the subject is in the pattern
            // type `T(b1, …)` at the top place (rule 11), so its fields have
            // the types of these arguments. The type of the name is not
            // changed: outside the clause, it may have other values.
            let before = known.map(|(x, ctor)| {
                let h = Head::Named(TypeRef::Custom(ctor.ty));
                let n = self.p.ty(ctor.ty).params.len();
                let args = self.shared(&mut places, 0, h, n);
                let old_args = self.known_args.insert(x, args);
                (x, self.known.insert(x, ctor), old_args)
            });
            if let Some(g) = &c.guard {
                let t = self.infer(g);
                let want = self.bool_ty();
                self.sub(&t, &want, g.span);
            }
            let (t, k) = self.infer_k(&c.body, expected);
            knowns.push(k);
            self.sub(&t, &r, c.body.span);
            if let Some((x, old, old_args)) = before {
                match old {
                    Some(o) => self.known.insert(x, o),
                    None => self.known.remove(&x),
                };
                match old_args {
                    Some(o) => self.known_args.insert(x, o),
                    None => self.known_args.remove(&x),
                };
            }
        }
        let module = self.module;
        if fitted {
            let clause = &clauses[0];
            let at = clause
                .patterns
                .first()
                .unwrap()
                .span
                .to(clause.patterns.last().unwrap().span);
            self.defer_failure(Pat::Or(rows[0].clone()), a, at);
        } else {
            self.cases.push(PendingCase {
                module,
                held,
                subject: a,
                span: subject.span,
                clauses: rows
                    .into_iter()
                    .zip(clauses)
                    .map(|(choices, c)| {
                        let span = match (c.patterns.first(), c.patterns.last()) {
                            (Some(f), Some(l)) => Span {
                                start: f.span.start,
                                end: l.span.end,
                            },
                            _ => c.span,
                        };
                        (Pat::Or(choices), c.guard.is_some(), span)
                    })
                    .collect(),
            });
        }
        (r, self.kunion(knowns))
    }

    /// Rule 12: the type of the catch-all `id` of choice `choice` of clause
    /// `ci`, at `place`: `B - H`, with `H` the type constructors that the
    /// earlier clauses without a guard take there. "Every value that the
    /// clause pattern matches" is any value at the other places: the types
    /// are not known yet, so the columns have no type.
    /// Also gives the type constructors that are taken, or `None` when every
    /// one is.
    fn narrow(
        &mut self,
        rows: &[Vec<Pat>],
        clauses: &[Clause],
        ci: usize,
        choice: usize,
        id: u32,
        place: &Place,
    ) -> (Ty, Option<Vec<Ty>>) {
        let earlier: Vec<Vec<Pat>> = (0..ci)
            .filter(|i| clauses[*i].guard.is_none())
            .map(|i| vec![Pat::Or(rows[i].clone())])
            .collect();
        // A choice whose values are all matched before: every type
        // constructor is taken there.
        if self
            .useful(
                &earlier,
                std::slice::from_ref(&rows[ci][choice]),
                &[Col::Any],
            )
            .is_none()
        {
            return (Ty::Never, None);
        }
        let mut candidates: Vec<(Head, usize)> = Vec::new();
        for (c, unguarded, h, n) in &place.seen {
            if *c < ci && *unguarded && !candidates.iter().any(|x| x.0 == *h) {
                candidates.push((*h, *n));
            }
        }
        let mut taken = Vec::new();
        // Under Any columns, a finite literal prefix cannot cover a
        // whole infinite scalar head. Keep the matrix for every other row.
        let literal_prefix = place.parent.is_none()
            && (0..ci)
                .filter(|i| clauses[*i].guard.is_none())
                .all(|i| rows[i].iter().all(coverage::scalar_literal_row));
        for (h, n) in candidates {
            if literal_prefix
                && matches!(
                    h,
                    Head::Named(TypeRef::Builtin(BuiltinType::Int | BuiltinType::String))
                )
            {
                continue;
            }
            let q = coverage::replace(&rows[ci][choice], id, &Pat::Head(h, n));
            if self.useful(&earlier, &[q], &[Col::Any]).is_none() {
                let args: Vec<Ty> = (0..n).map(|_| self.s.fresh()).collect();
                taken.push(match h {
                    Head::Fn => Ty::Fn(Box::new(FnTy {
                        params: args.into_iter().map(|a| (None, a)).collect(),
                        ret: self.s.fresh(),
                        err: self.s.fresh(),
                        env: self.s.fresh(),
                    })),
                    h => Ty::Con(h, super::lits::ValueSet::ALL, args),
                });
            }
        }
        if LITERAL_CANDIDATES {
            let mut scalar_taken: HashSet<Ty> = HashSet::default();
            for (earlier_clause, unguarded, candidate, pat) in &place.scalar_seen {
                if *earlier_clause >= ci || !unguarded || scalar_taken.contains(candidate) {
                    continue;
                }
                // The visit record already identifies the earlier clause.
                // Inspect its actual row, not every earlier row and not a
                // fitted row (which would erase a failing type test).
                let fast = place.parent.is_none() && rows[*earlier_clause].iter().any(|row| matches!((row,pat),(Pat::Ctor(Ctor::Lit(a),_),Pat::Ctor(Ctor::Lit(b),_)) if a == b));
                let q = coverage::replace(&rows[ci][choice], id, pat);
                if fast || self.useful(&earlier, &[q], &[Col::Any]).is_none() {
                    scalar_taken.insert(candidate.clone());
                    taken.push(candidate.clone());
                }
            }
        }
        if taken.is_empty() {
            return (place.ty.clone(), Some(taken));
        }
        let t = self.s.diff(place.ty.clone(), taken.clone());
        self.flush(place_span(clauses, ci), None);
        (t, Some(taken))
    }

    /// Checks the coverage (§7.4) of a `case`, and warns about the clauses
    /// that are never used, with the types that inference found.
    pub fn check_case(&mut self, c: PendingCase) {
        self.set_module(c.module);
        let col = [Col::of(c.subject.clone())];
        let mut rows: Vec<Vec<Pat>> = Vec::new();
        let mut scalar_rows = Some(coverage::ScalarRows::default());
        for (pat, guarded, span) in &c.clauses {
            let useful = match &scalar_rows {
                Some(index) => self.scalar_row_useful(index, pat, &c.subject, &rows),
                None => self
                    .useful(&rows, std::slice::from_ref(pat), &col)
                    .is_some(),
            };
            if !useful {
                let mut message = "this clause is never used: every value that its pattern matches is matched by an earlier clause without a guard (§7.4)".to_string();
                if let Ok(t) =
                    super::display::shown(&self.s, &c.subject, super::display::Opts::default())
                {
                    let ms = known::members(&t);
                    if let [m] = ms.as_slice()
                        && m.scalar().is_some()
                        && m.values().is_finite()
                        && m.values().items().count() == 1
                    {
                        message.push_str(&format!(
                            "; the value matched is always `{}`",
                            super::display::show_known(&self.s, m)
                        ));
                    }
                }
                self.warning(code::NEVER_USED, *span, message);
            }
            if !guarded {
                if let Some(index) = &mut scalar_rows
                    && !index.add(pat)
                {
                    scalar_rows = None;
                }
                rows.push(vec![pat.clone()]);
            }
        }
        let missing = match &scalar_rows {
            Some(index) => self.scalar_row_missing(index, &c.subject),
            None => self.useful(&rows, &[Pat::Wild(None)], &col),
        };
        if let Some(w) = missing {
            let text = self.show_pat(&w[0]);
            let mut message = format!(
                "the clauses do not cover every value of the subject (§7.4): no clause without a guard matches `{text}`"
            );
            if let Some(hint) = self.shared_hint(&w[0], &rows) {
                message.push_str("; ");
                message.push_str(&hint);
            }
            self.error(code::CASE_NOT_COVERED, c.span, message);
        } else {
            for (failure, span) in c.held {
                self.s.failures.push(failure);
                self.flush(span, None);
            }
        }
    }

    pub(super) fn field_shared(&mut self, ctor: CtorRef, index: usize) -> bool {
        if !self.shared_fields.contains_key(&ctor)
            && let TypeBody::Custom(ctors) = &self.p.ty(ctor.ty).body
        {
            let fields: Vec<Vec<Vec<TyVarId>>> = ctors
                .iter()
                .map(|c| {
                    c.fields
                        .iter()
                        .map(|f| {
                            let mut out = Vec::new();
                            field_vars(self.p, &f.ty, &mut out, 0);
                            out
                        })
                        .collect()
                })
                .collect();
            let mut counts = HashMap::new();
            for v in fields.iter().flatten().flatten() {
                *counts.entry(*v).or_insert(0usize) += 1;
            }
            for (i, fields) in fields.into_iter().enumerate() {
                self.shared_fields.insert(
                    CtorRef {
                        ty: ctor.ty,
                        index: i as u32,
                    },
                    fields
                        .into_iter()
                        .map(|vars| vars.iter().any(|v| counts[v] > 1))
                        .collect(),
                );
            }
        }
        self.shared_fields
            .get(&ctor)
            .and_then(|xs| xs.get(index))
            .copied()
            .unwrap_or(false)
    }

    /// The place below `at` by `step`, with the type `ty` and the known
    /// type `known` when it is new.
    fn child(
        &mut self,
        places: &mut Vec<Place>,
        at: usize,
        step: Step,
        ty: Ty,
        known: Ty,
    ) -> usize {
        if let Some((_, i)) = places[at].children.iter().find(|c| c.0 == step) {
            return *i;
        }
        let shared = places[at].shared
            || matches!(step, Step::Item(_))
            || match step {
                Step::Field(c, i) => self.field_shared(c, i as usize),
                _ => false,
            };
        let catch_here =
            matches!(step, Step::Item(i) if places[at].rest_from.is_some_and(|from| i>=from));
        places.push(Place {
            ty,
            known,
            pats: Vec::new(),
            heads: Vec::new(),
            catch_here,
            rest_from: None,
            parent: Some(at),
            children: Vec::new(),
            seen: Vec::new(),
            scalar_seen: Vec::new(),
            shared,
        });
        let i = places.len() - 1;
        places[at].children.push((step, i));
        i
    }

    /// The arguments shared at a place by the patterns with one head
    /// (rule 11).
    fn shared(&mut self, places: &mut [Place], at: usize, h: Head, n: usize) -> Vec<Ty> {
        if let Some((_, args)) = places[at].heads.iter().find(|x| x.0 == h) {
            return args.clone();
        }
        let args: Vec<Ty> = (0..n).map(|_| self.s.fresh()).collect();
        places[at].heads.push((h, args.clone()));
        args
    }

    /// Puts a clause pattern at the place `at`, binds its names (but not
    /// those of catch-alls, which wait for rule 12), and gives back its
    /// pattern type and its pattern for `coverage`.
    fn visit<'p>(
        &mut self,
        q: &'p Pattern,
        at: usize,
        v: &mut Visit<'p>,
        places: &mut Vec<Place>,
    ) -> (Ty, Pat) {
        if catch_all(q) {
            if v.unguarded {
                places[at].catch_here = true;
            }
            let id = v.catches.len() as u32;
            v.catches.push(Catch {
                pattern: q,
                clause: v.clause,
                choice: v.choice,
                place: at,
            });
            return (places[at].ty.clone(), Pat::Wild(Some(id)));
        }
        if let PatternKind::Named { binder, pattern } = &q.kind {
            let (t, pat) = self.visit(pattern, at, v, places);
            self.bind(binder, t.clone(), true);
            let k = self.member_for(pattern, &places[at].known);
            self.kbind(binder.var, k, true);
            return (t, pat);
        }
        let (t, pat) = match &q.kind {
            PatternKind::Typed { binder, ty } => {
                let t = self.conv(ty, &HashMap::new());
                let mut members = Vec::new();
                super::solve::flatten_union(std::slice::from_ref(&t), &mut members);
                let pat = if members
                    .iter()
                    .any(|m| m.head().is_none() && *m != Ty::Never)
                {
                    self.error(
                        code::CASE_TYPE_VAR,
                        ty.span,
                        "in a test, `x: T` tests the type constructor, so `T` may not have a type variable as a member (§7.1)",
                    );
                    Pat::Wild(None)
                } else {
                    Pat::Or(
                        members
                            .iter()
                            .filter_map(|m| match m {
                                m @ Ty::Con(..) if m.scalar().is_some() => {
                                    Some(coverage::scalar_pat(m))
                                }
                                Ty::Con(h, _, args) => Some(Pat::Head(*h, args.len())),
                                Ty::Fn(f) => Some(Pat::Head(Head::Fn, f.params.len())),
                                _ => None,
                            })
                            .collect(),
                    )
                };
                if matches!(&pat, Pat::Or(_)) {
                    let heads = members
                        .iter()
                        .filter_map(|m| {
                            m.head().map(|head| super::TestMember {
                                head,
                                values: if m.values().is_all() {
                                    super::TestValues::All
                                } else {
                                    let values = m
                                        .values()
                                        .items()
                                        .map(|v| match m.scalar().unwrap() {
                                            BuiltinType::Int => {
                                                super::LitValue::Int(v.parse().unwrap())
                                            }
                                            BuiltinType::String => {
                                                super::LitValue::Str(v.to_string())
                                            }
                                            _ => unreachable!(),
                                        })
                                        .collect();
                                    if m.values().is_finite() {
                                        super::TestValues::Only(values)
                                    } else {
                                        super::TestValues::Except(values)
                                    }
                                },
                            })
                        })
                        .collect();
                    self.out.type_tests.insert((self.module, q.span), heads);
                    for m in &members {
                        if m.scalar().is_some() && !m.values().is_all() {
                            places[at].scalar_seen.push((
                                v.clause,
                                v.unguarded,
                                m.clone(),
                                coverage::scalar_pat(m),
                            ));
                        } else if let Some(h) = m.head() {
                            let n = match m {
                                Ty::Con(_, _, xs) => xs.len(),
                                Ty::Fn(f) => f.params.len(),
                                _ => 0,
                            };
                            places[at].seen.push((v.clause, v.unguarded, h, n));
                        }
                    }
                    // Rule 11: the test takes every value of the type
                    // constructors of `T`, so those values must be in `T`:
                    // `B <: T | r - T`.
                    // §7.1: custom types with no constructors cannot be
                    // told apart at run time, so for a type test they all
                    // count as one type constructor: a test of one takes
                    // the values of all of them.
                    let mut taken = members.clone();
                    if members.iter().any(|m| self.no_constructors(m)) {
                        for (i, t) in self.p.types.iter().enumerate() {
                            if matches!(&t.body, TypeBody::Custom(cs) if cs.is_empty()) {
                                let m = Ty::named(TypeRef::Custom(TypeId(i as u32)), Vec::new());
                                if !taken.contains(&m) {
                                    taken.push(m);
                                }
                            }
                        }
                    }
                    let r = self.s.fresh();
                    let rest = self.s.diff(r, taken);
                    let mut want = members.clone();
                    want.push(rest);
                    let b = places[at].ty.clone();
                    self.sub(&b, &Ty::Union(want), ty.span);
                }
                if let Some(b) = binder {
                    self.bind(b, t.clone(), true);
                    let k = self.kconv(ty, &HashMap::new());
                    self.kbind(b.var, k, true);
                }
                (t, pat)
            }
            PatternKind::Constructor { ctor, args, spread } => {
                let h = Head::Named(TypeRef::Custom(ctor.ty));
                let n = self.p.ty(ctor.ty).params.len();
                let targs = self.shared(places, at, h, n);
                let fields = self.ctor_fields(*ctor, &targs);
                let kargs = match known::member(&places[at].known, Some(h)) {
                    Some(Ty::Con(_, _, xs)) => Some(xs),
                    _ => None,
                };
                let mut parts = vec![Pat::Wild(None); fields.len()];
                let matched = self.match_fields(q.span, &fields, args, *spread);
                // Rule 11: `..` counts as a catch-all at the places of the
                // fields that it leaves out.
                if *spread && v.unguarded {
                    for (i, field) in fields.iter().enumerate() {
                        if matched.iter().any(|m| m.1 == i) {
                            continue;
                        }
                        let k = match &kargs {
                            Some(xs) => self.field_known(*ctor, i, Some(xs)),
                            None => Ty::Top,
                        };
                        let c = self.child(
                            places,
                            at,
                            Step::Field(*ctor, i as u32),
                            field.1.clone(),
                            k,
                        );
                        places[c].catch_here = true;
                    }
                }
                for (arg, i) in matched {
                    let k = match &kargs {
                        Some(xs) => self.field_known(*ctor, i, Some(xs)),
                        None => Ty::Top,
                    };
                    let c = self.child(
                        places,
                        at,
                        Step::Field(*ctor, i as u32),
                        fields[i].1.clone(),
                        k,
                    );
                    parts[i] = self.visit(arg, c, v, places).1;
                }
                places[at].seen.push((v.clause, v.unguarded, h, n));
                (
                    Ty::named(TypeRef::Custom(ctor.ty), targs),
                    Pat::Ctor(Ctor::Custom(*ctor), parts),
                )
            }
            PatternKind::Tuple(ps) => {
                let n = ps.len() as u32;
                let h = Head::Tuple(n);
                let parts = self.shared(places, at, h, ps.len());
                let kparts = match known::member(&places[at].known, Some(h)) {
                    Some(Ty::Con(_, _, xs)) => xs,
                    _ => vec![Ty::Top; ps.len()],
                };
                let mut pats = Vec::new();
                for (i, x) in ps.iter().enumerate() {
                    let c = self.child(
                        places,
                        at,
                        Step::Part(n, i as u32),
                        parts[i].clone(),
                        kparts[i].clone(),
                    );
                    pats.push(self.visit(x, c, v, places).1);
                }
                places[at].seen.push((v.clause, v.unguarded, h, ps.len()));
                (Ty::tuple(parts), Pat::Ctor(Ctor::Tuple(n), pats))
            }
            PatternKind::List { items, rest } => {
                let h = Head::Named(TypeRef::Builtin(BuiltinType::List));
                let e = self.shared(places, at, h, 1).remove(0);
                let list = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![e.clone()]);
                let klist = known::member(&places[at].known, Some(h)).unwrap_or(Ty::Top);
                let kitem = match &klist {
                    Ty::Con(_, _, xs) if xs.len() == 1 => xs[0].clone(),
                    _ => Ty::Top,
                };
                let mut pats = Vec::new();
                for (i, x) in items.iter().enumerate() {
                    let c = self.child(places, at, Step::Item(i as u32), e.clone(), kitem.clone());
                    pats.push(self.visit(x, c, v, places).1);
                }
                if let Some(r) = rest {
                    if v.unguarded {
                        let from = items.len() as u32;
                        places[at].rest_from =
                            Some(places[at].rest_from.map_or(from, |old| old.min(from)));
                        let covered: Vec<_> = places[at]
                            .children
                            .iter()
                            .filter_map(|(step, i)| {
                                matches!(step,Step::Item(j) if *j>=from).then_some(*i)
                            })
                            .collect();
                        for i in covered {
                            places[i].catch_here = true;
                        }
                    }
                    let c = self.child(places, at, Step::Rest, list.clone(), klist.clone());
                    if let Some(b) = r {
                        self.bind(b, list.clone(), true);
                        self.kbind(b.var, klist, true);
                    }
                    if v.unguarded {
                        places[c].catch_here = true;
                    }
                }
                places[at].seen.push((v.clause, v.unguarded, h, 1));
                (list, coverage::list(pats, rest.is_some()))
            }
            _ => {
                let t = self.leaf(q, true, !v.fitted && !places[at].shared);
                let k = places[at].known.clone();
                self.fit_known(q, &k, true);
                let pat = match coverage::literal(q) {
                    Some(l) => {
                        if LITERAL_CANDIDATES {
                            let value = match &l {
                                coverage::Lit::Int(n) => Some((BuiltinType::Int, n.as_str())),
                                coverage::Lit::Str(s) => Some((BuiltinType::String, s.as_str())),
                                _ => None,
                            };
                            if let Some((b, text)) = value {
                                let t = Ty::lit(
                                    b,
                                    super::lits::ValueSet::only([std::sync::Arc::from(text)])
                                        .unwrap(),
                                );
                                places[at].scalar_seen.push((
                                    v.clause,
                                    v.unguarded,
                                    t,
                                    Pat::Ctor(Ctor::Lit(l.clone()), Vec::new()),
                                ));
                            }
                        }
                        let c = Ctor::Lit(l);
                        places[at].seen.push((v.clause, v.unguarded, c.head(), 0));
                        Pat::Ctor(c, Vec::new())
                    }
                    None => Pat::Wild(None),
                };
                (t, pat)
            }
        };
        places[at].pats.push(t.clone());
        (t, pat)
    }
}

/// The constructor at the top of a pattern, through `x = …`.
fn top_ctor(p: &Pattern) -> Option<CtorRef> {
    match &p.kind {
        PatternKind::Constructor { ctor, .. } => Some(*ctor),
        PatternKind::Named { pattern, .. } => top_ctor(pattern),
        _ => None,
    }
}

/// Every name that a pattern binds, with the constructor `C` when it is
/// bound through `x = C(…)` (§7.5).
fn ctor_binders(p: &Pattern, out: &mut Vec<(VarId, Option<CtorRef>)>) {
    match &p.kind {
        PatternKind::Quote { captures, .. } => {
            captures.iter().for_each(|(_, p)| ctor_binders(p, out))
        }
        PatternKind::Named { binder, pattern } => {
            out.push((binder.var, top_ctor(pattern)));
            ctor_binders(pattern, out);
        }
        PatternKind::Constructor { args: ps, .. }
        | PatternKind::Tuple(ps)
        | PatternKind::Error(ps) => ps.iter().for_each(|q| ctor_binders(q, out)),
        PatternKind::List { items, .. } => items.iter().for_each(|q| ctor_binders(q, out)),
        PatternKind::BitArray(segs) => segs.iter().for_each(|s| ctor_binders(&s.value, out)),
        _ => {
            let mut vs = Vec::new();
            binders(p, &mut vs);
            out.extend(vs.into_iter().map(|v| (v, None)));
        }
    }
}

/// The place of the patterns of a clause, for problems.
fn place_span(clauses: &[Clause], ci: usize) -> Span {
    let c = &clauses[ci];
    c.patterns.first().map_or(c.span, |p| p.span)
}

fn field_vars(p: &Program, t: &Type, out: &mut Vec<TyVarId>, depth: usize) {
    if depth > 64 {
        return;
    }
    match &t.kind {
        TypeKind::Var(v) => out.push(*v),
        TypeKind::Named {
            ty: TypeRef::Custom(id),
            args,
        } if matches!(p.ty(*id).body, TypeBody::Alias(_)) => {
            let TypeBody::Alias(body) = &p.ty(*id).body else {
                unreachable!()
            };
            let map = p
                .ty(*id)
                .params
                .iter()
                .map(|(_, v)| *v)
                .zip(args.iter())
                .collect();
            field_vars(p, &crate::resolve::subst(body, &map), out, depth + 1);
        }
        TypeKind::Named { args, .. }
        | TypeKind::Tuple(args)
        | TypeKind::Union(args)
        | TypeKind::Inter(args)
        | TypeKind::Diff(args) => {
            for t in args {
                field_vars(p, t, out, depth);
            }
        }
        TypeKind::Fn {
            params,
            ret,
            err,
            env,
        } => {
            for pty in params {
                field_vars(p, &pty.ty, out, depth);
            }
            field_vars(p, ret, out, depth);
            for t in [err, env].into_iter().flatten() {
                field_vars(p, t, out, depth);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod shared_place_tests {
    use super::*;
    #[test]
    fn written_occurrences_include_other_constructors_and_aliases() {
        // literal-types §5.3: count every written occurrence after aliases,
        // across every constructor of the family, before inference.
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("i.toml"), "name = \"app\"\n").unwrap();
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(
            root.path().join("src/main.i"),
            r#"
Pair(A) { Pair(A, A) }
Event(A) { Start(A) Stop(A) }
Two(A) = (A, A)
Wrapped(A) { Wrapped(Two(A)) }
Solo(A) { Solo(A, Int) }
"#,
        )
        .unwrap();
        let (graph, diags) = ilang_package::load(root.path());
        assert!(diags.is_empty());
        let checked = crate::check(&graph);
        assert!(checked.diags.is_empty(), "{:?}", checked.diags);
        let mut p = checked.program.unwrap();
        let wrapped = p.types.iter().position(|t| t.name == "Wrapped").unwrap();
        if let TypeBody::Custom(cs) = &mut p.types[wrapped].body {
            for _ in 0..80 {
                let inner = cs[0].fields[0].ty.clone();
                cs[0].fields[0].ty.kind = TypeKind::Tuple(vec![inner]);
            }
        }
        let mut c = Checker::new(&p, &graph.sources);
        let id = |name| {
            TypeId(
                p.types
                    .iter()
                    .position(|t| t.name == name && p.module(t.module).path == "app/main")
                    .unwrap() as u32,
            )
        };
        assert!(c.field_shared(
            CtorRef {
                ty: id("Pair"),
                index: 0
            },
            0
        ));
        assert!(c.field_shared(
            CtorRef {
                ty: id("Pair"),
                index: 0
            },
            1
        ));
        assert!(c.field_shared(
            CtorRef {
                ty: id("Event"),
                index: 0
            },
            0
        ));
        assert!(c.field_shared(
            CtorRef {
                ty: id("Event"),
                index: 1
            },
            0
        ));
        assert!(c.field_shared(
            CtorRef {
                ty: id("Wrapped"),
                index: 0
            },
            0
        ));
        assert!(!c.field_shared(
            CtorRef {
                ty: id("Solo"),
                index: 0
            },
            0
        ));
        assert!(!c.field_shared(
            CtorRef {
                ty: id("Solo"),
                index: 0
            },
            1
        ));
    }
}
