//! The constraint solver: type variables with levels and bounds, as in
//! Simple-sub, extended with the unions, intersections, and differences of
//! §4.4 and with the variance of type parameters (§4.5).
//!
//! Every variable has a level, and lower and upper bounds. `constrain(l, r)`
//! keeps `l <: r` true for every bound that is added, so the bounds of a
//! variable are always consistent. A type at a deeper level is copied to
//! the level of the variable first (extrusion), so that generalization only
//! takes the variables of the binding.

use super::{FnTy, Head, Ty, V, Variance};
use crate::hir::{BuiltinType, TypeId, TypeRef};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub(crate) struct VarState {
    pub level: u32,
    pub lower: Vec<Ty>,
    pub upper: Vec<Ty>,
    /// The name written in an annotation (§4.7), kept when it is shown.
    pub name: Option<String>,
    /// Values may come to the variable from outside the constraints: it is
    /// the type of a parameter, a written type variable, or the result of a
    /// function that is not known. Its lower bounds are then not all its
    /// values (§7.3, §7.4).
    pub input: bool,
    /// The variables that have this one as an upper bound: their values
    /// come to it too, but they are not among its lower bounds.
    pub from: Vec<V>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FailKind {
    /// The left side is not a subtype of the right side.
    Mismatch,
    /// Function types with different numbers of parameters.
    Arity,
    /// Two forms of a type constructor whose parameter does not change, with
    /// arguments that cannot be equal (§4.4).
    Invariant,
    /// Two forms of one type constructor in an environment, with arguments
    /// that cannot be equal (§9.2).
    EnvForms,
    /// A value could go to more than one type variable of a union (§4.4).
    Ambiguous,
}

#[derive(Clone, Debug)]
pub(crate) struct Failure {
    pub kind: FailKind,
    pub lhs: Ty,
    pub rhs: Ty,
}

/// The polarity of a place: output (`Pos`), input (`Neg`), or both, for an
/// argument that does not change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Pol {
    Pos,
    Neg,
    Both,
}

impl Pol {
    pub fn flip(self) -> Pol {
        match self {
            Pol::Pos => Pol::Neg,
            Pol::Neg => Pol::Pos,
            Pol::Both => Pol::Both,
        }
    }

    pub fn by(self, v: Variance) -> Pol {
        match v {
            Variance::Co | Variance::Bi => self,
            Variance::Contra => self.flip(),
            Variance::Inv => Pol::Both,
        }
    }
}

pub(crate) struct Solver {
    pub vars: Vec<VarState>,
    /// The level of new variables: one more than the level of the binding
    /// whose value is being typed, for a binding that is generalized.
    pub level: u32,
    /// The constraints with a variable that are already made, each with
    /// whether it was made between environments.
    cache: HashSet<(Ty, Ty, bool)>,
    pub variance: HashMap<TypeId, Vec<Variance>>,
    pub failures: Vec<Failure>,
    /// The names of the custom types, by `TypeId`, to show types.
    pub type_names: Vec<String>,
}

impl Solver {
    pub fn new(variance: HashMap<TypeId, Vec<Variance>>, type_names: Vec<String>) -> Solver {
        Solver {
            vars: Vec::new(),
            level: 0,
            cache: HashSet::new(),
            variance,
            failures: Vec::new(),
            type_names,
        }
    }

    pub fn fresh(&mut self) -> Ty {
        Ty::Var(self.fresh_at(self.level))
    }

    /// A new variable whose values may come from outside (`input`).
    pub fn fresh_input(&mut self) -> Ty {
        let v = self.fresh_at(self.level);
        self.vars[v.0 as usize].input = true;
        Ty::Var(v)
    }

    /// Marks the variables made since `start` as inputs.
    pub fn mark_inputs(&mut self, start: usize) {
        for v in &mut self.vars[start..] {
            v.input = true;
        }
    }

    pub fn fresh_at(&mut self, level: u32) -> V {
        self.vars.push(VarState {
            level,
            lower: Vec::new(),
            upper: Vec::new(),
            name: None,
            input: false,
            from: Vec::new(),
        });
        V(self.vars.len() as u32 - 1)
    }

    /// Sets the upper bounds of a new variable.
    fn set_upper(&mut self, v: V, upper: Vec<Ty>) {
        for u in &upper {
            if let Ty::Var(w) = u {
                self.var_mut(*w).from.push(v);
            }
        }
        self.var_mut(v).upper = upper;
    }

    fn var(&self, v: V) -> &VarState {
        &self.vars[v.0 as usize]
    }

    fn var_mut(&mut self, v: V) -> &mut VarState {
        &mut self.vars[v.0 as usize]
    }

    /// The variances of the parameters of a head (§4.5 rules 6 and 7).
    pub fn variances(&self, h: Head, n: usize) -> Vec<Variance> {
        match h {
            Head::Named(TypeRef::Custom(id)) => self
                .variance
                .get(&id)
                .cloned()
                .unwrap_or_else(|| vec![Variance::Inv; n]),
            Head::Named(TypeRef::Builtin(b)) => builtin_variance(b),
            Head::Tuple(_) | Head::Fn => vec![Variance::Co; n],
        }
    }

    fn fail(&mut self, kind: FailKind, lhs: &Ty, rhs: &Ty) {
        self.failures.push(Failure {
            kind,
            lhs: lhs.clone(),
            rhs: rhs.clone(),
        });
    }

    /// The highest level of the variables in `t`.
    pub fn level_of(&self, t: &Ty) -> u32 {
        match t {
            Ty::Var(v) => self.var(*v).level,
            Ty::Con(_, args) => args.iter().map(|a| self.level_of(a)).max().unwrap_or(0),
            Ty::Fn(f) => f
                .params
                .iter()
                .map(|p| &p.1)
                .chain([&f.ret, &f.err, &f.env])
                .map(|a| self.level_of(a))
                .max()
                .unwrap_or(0),
            Ty::Union(ms) | Ty::Inter(ms) => ms.iter().map(|a| self.level_of(a)).max().unwrap_or(0),
            Ty::Diff(a, h) => h
                .iter()
                .map(|x| self.level_of(x))
                .max()
                .unwrap_or(0)
                .max(self.level_of(a)),
            Ty::Both(a, b) => self.level_of(a).max(self.level_of(b)),
            Ty::Never | Ty::Top => 0,
        }
    }

    // ---- Constraints ----

    /// Makes `l <: r` true, or records why it cannot be.
    pub fn constrain(&mut self, l: &Ty, r: &Ty) {
        self.constrain_in(l, r, false);
    }

    /// Makes `N` **in** `M` true for two environments (§4.5 rule 4): every
    /// member of `N` is a type variable that is a subtype of `M`, or has a
    /// member of `M` with the same type constructor and equal arguments,
    /// whatever the variance of the parameters.
    pub fn constrain_env(&mut self, l: &Ty, r: &Ty) {
        self.constrain_in(l, r, true);
    }

    /// `l <: r`, or `l` in `r` when `env` (§4.5 rule 4). The mode goes
    /// through the bounds of variables, but not into type arguments: those
    /// are compared as normal types.
    fn constrain_in(&mut self, l: &Ty, r: &Ty, env: bool) {
        if l == r || matches!(l, Ty::Never) || matches!(r, Ty::Top) {
            return;
        }
        if (matches!(l, Ty::Var(_)) || matches!(r, Ty::Var(_)))
            && !self.cache.insert((l.clone(), r.clone(), env))
        {
            return;
        }
        match (l, r) {
            (Ty::Union(ms), _) => {
                for m in ms {
                    self.constrain_in(m, r, env);
                }
            }
            (_, Ty::Inter(ms)) => {
                for m in ms {
                    self.constrain_in(l, m, env);
                }
            }
            (Ty::Var(v), _) if self.level_of(r) <= self.var(*v).level => {
                self.add_bound(*v, r.clone(), false, env);
                for lb in self.var(*v).lower.clone() {
                    self.constrain_in(&lb, r, env);
                }
            }
            (_, Ty::Var(v)) if self.level_of(l) <= self.var(*v).level => {
                self.add_bound(*v, l.clone(), true, env);
                for ub in self.var(*v).upper.clone() {
                    self.constrain_in(l, &ub, env);
                }
            }
            (Ty::Var(v), _) => {
                let level = self.var(*v).level;
                let r2 = self.extrude(r, Pol::Neg, level, &mut HashMap::new());
                self.constrain_in(l, &r2, env);
            }
            (_, Ty::Var(v)) => {
                let level = self.var(*v).level;
                let l2 = self.extrude(l, Pol::Pos, level, &mut HashMap::new());
                self.constrain_in(&l2, r, env);
            }
            (Ty::Inter(ms), _) => {
                // `a & T <: R`: the members that are not type variables give
                // the value; with none, the first variable does.
                let (vars, conc): (Vec<Ty>, Vec<Ty>) =
                    ms.iter().cloned().partition(|m| matches!(m, Ty::Var(_)));
                if conc.is_empty() {
                    self.constrain_in(&vars[0], r, env);
                } else {
                    let c = self.inter(conc);
                    self.constrain_in(&c, r, env);
                }
            }
            (Ty::Diff(a, h), _) => self.diff_left(a, h, r, env),
            (Ty::Top, _) => self.fail(FailKind::Mismatch, l, r),
            (Ty::Both(a, _), _) => self.constrain_in(&a.clone(), r, env),
            (_, Ty::Both(_, b)) => self.constrain_in(l, &b.clone(), env),
            (_, Ty::Union(ms)) => self.route(l, ms, env),
            (_, Ty::Diff(a, h)) => {
                if h.iter().any(|m| m.head() == l.head()) {
                    self.fail(FailKind::Mismatch, l, r);
                } else {
                    self.constrain_in(l, a, env);
                }
            }
            (Ty::Con(h1, a1), Ty::Con(h2, a2)) if h1 == h2 && a1.len() == a2.len() => {
                if env {
                    // §4.5 rule 4: service arguments are equal.
                    for (x, y) in a1.iter().zip(a2) {
                        self.constrain(x, y);
                        self.constrain(y, x);
                    }
                    return;
                }
                let vs = self.variances(*h1, a1.len());
                for ((x, y), v) in a1.iter().zip(a2).zip(vs) {
                    self.constrain_by(x, y, v);
                }
            }
            (Ty::Fn(f), Ty::Fn(g)) => {
                if f.params.len() != g.params.len() {
                    self.fail(FailKind::Arity, l, r);
                    return;
                }
                for (p, q) in f.params.iter().zip(&g.params) {
                    self.constrain(&q.1, &p.1);
                }
                self.constrain(&f.ret, &g.ret);
                self.constrain(&f.err, &g.err);
                self.constrain_env(&f.env, &g.env);
            }
            _ => self.fail(FailKind::Mismatch, l, r),
        }
    }

    fn constrain_by(&mut self, x: &Ty, y: &Ty, v: Variance) {
        match v {
            Variance::Co => self.constrain(x, y),
            Variance::Contra => self.constrain(y, x),
            Variance::Inv => {
                self.constrain(x, y);
                self.constrain(y, x);
            }
            Variance::Bi => {}
        }
    }

    /// Adds a bound to a variable. Two bounds of one variable with the same
    /// type constructor are one member (§4.4), so arguments that do not
    /// change must be equal; in an environment, all arguments must be equal
    /// (§9.2). Two lower bounds that are put together by union also have
    /// their parts put together, so that the environments of two function
    /// types are checked in the same way.
    fn add_bound(&mut self, v: V, t: Ty, lower: bool, env: bool) {
        let level = self.var(v).level;
        let bounds = if lower {
            self.var(v).lower.clone()
        } else {
            self.var(v).upper.clone()
        };
        match &t {
            Ty::Fn(f) => {
                for b in &bounds {
                    let Ty::Fn(g) = b else { continue };
                    if g.params.len() != f.params.len() {
                        // All function types are one head; their parameters
                        // must agree in number to be put together.
                        self.fail(FailKind::Arity, b, &t);
                    } else if lower {
                        self.join(&f.ret, &g.ret, level, false);
                        self.join(&f.err, &g.err, level, false);
                        self.join(&f.env, &g.env, level, true);
                    }
                }
            }
            Ty::Con(h, args) => {
                let vs = self.variances(*h, args.len());
                for b in &bounds {
                    let Ty::Con(h2, other) = b else { continue };
                    if h2 != h || other.len() != args.len() {
                        continue;
                    }
                    for ((x, y), var) in args.iter().zip(other).zip(&vs) {
                        if env {
                            self.equal(x, y, FailKind::EnvForms);
                        } else if *var == Variance::Inv {
                            self.equal(x, y, FailKind::Invariant);
                        } else if *var == Variance::Co && lower {
                            self.join(x, y, level, false);
                        }
                    }
                }
            }
            _ => {}
        }
        let s = self.var_mut(v);
        let bounds = if lower { &mut s.lower } else { &mut s.upper };
        if !bounds.contains(&t) {
            bounds.push(t.clone());
            if let (false, Ty::Var(w)) = (lower, &t) {
                self.var_mut(*w).from.push(v);
            }
        }
    }

    /// Puts two types together by union in a new variable, only for the
    /// checks of `add_bound`: the variable is used nowhere else.
    fn join(&mut self, x: &Ty, y: &Ty, level: u32, env: bool) {
        if x == y || !(needs_join(x) && needs_join(y)) {
            return;
        }
        let w = Ty::Var(self.fresh_at(level));
        self.constrain_in(x, &w, env);
        self.constrain_in(y, &w, env);
    }

    /// `x` and `y` must be equal, as arguments that do not change, or as the
    /// arguments of two forms of a service in an environment; `kind` says
    /// which.
    fn equal(&mut self, x: &Ty, y: &Ty, kind: FailKind) {
        let before = self.failures.len();
        self.constrain(x, y);
        self.constrain(y, x);
        if self.failures.len() > before {
            self.failures.truncate(before);
            self.fail(kind, x, y);
        }
    }

    /// `l <: m1 | m2 | …` for an `l` with a type constructor: the member
    /// with the same type constructor, or else the one member that is a type
    /// variable (§4.4).
    fn route(&mut self, l: &Ty, ms: &[Ty], env: bool) {
        let mut flat = Vec::new();
        flatten_union(ms, &mut flat);
        let h = l.head();
        if h.is_some()
            && let Some(m) = flat.iter().find(|m| m.head() == h)
        {
            let m = m.clone();
            self.constrain_in(l, &m, env);
            return;
        }
        let vars: Vec<&Ty> = flat
            .iter()
            .filter(|m| m.is_varlike() || matches!(m, Ty::Inter(_)))
            .collect();
        match vars.len() {
            1 => {
                let m = vars[0].clone();
                self.constrain_in(l, &m, env);
            }
            0 => self.fail(FailKind::Mismatch, l, &Ty::Union(ms.to_vec())),
            _ => self.fail(FailKind::Ambiguous, l, &Ty::Union(ms.to_vec())),
        }
    }

    /// `A - H <: R`, where `R` is not a variable. For a variable `a`, its
    /// values whose type constructor is in `H` go to `H`, and the rest go
    /// to `R`: `a <: H | R`, with the members of `H` first.
    fn diff_left(&mut self, a: &Ty, h: &[Ty], r: &Ty, env: bool) {
        match a {
            Ty::Never => {}
            Ty::Union(ms) => {
                for m in ms {
                    self.diff_left(m, h, r, env);
                }
            }
            Ty::Con(..) | Ty::Fn(_) => match h.iter().find(|m| m.head() == a.head()) {
                Some(m) => {
                    let m = m.clone();
                    self.constrain_in(a, &m, env);
                }
                None => self.constrain_in(a, r, env),
            },
            _ => {
                let mut ms = h.to_vec();
                ms.push(r.clone());
                self.constrain_in(a, &Ty::Union(ms), env);
            }
        }
    }

    /// Copies the variables of `t` that are deeper than `level` to that
    /// level, with bounds that link them to the originals (Simple-sub).
    fn extrude(&mut self, t: &Ty, pol: Pol, level: u32, cache: &mut HashMap<(V, Pol), V>) -> Ty {
        if self.level_of(t) <= level {
            return t.clone();
        }
        match t {
            Ty::Var(v) => {
                if let Some(nv) = cache.get(&(*v, pol)) {
                    return Ty::Var(*nv);
                }
                let nv = self.fresh_at(level);
                self.vars[nv.0 as usize].input = self.var(*v).input;
                cache.insert((*v, pol), nv);
                if pol != Pol::Neg {
                    self.var_mut(*v).upper.push(Ty::Var(nv));
                    self.var_mut(nv).from.push(*v);
                    let lower = self.var(*v).lower.clone();
                    let lower = lower
                        .iter()
                        .map(|b| self.extrude(b, Pol::Pos, level, cache))
                        .collect();
                    self.var_mut(nv).lower = lower;
                }
                if pol != Pol::Pos {
                    self.var_mut(*v).lower.push(Ty::Var(nv));
                    let upper = self.var(*v).upper.clone();
                    let upper = upper
                        .iter()
                        .filter(|b| **b != Ty::Var(nv))
                        .map(|b| self.extrude(b, Pol::Neg, level, cache))
                        .collect();
                    self.set_upper(nv, upper);
                }
                Ty::Var(nv)
            }
            _ => self.map_children(t, pol, &mut |s, x, p| s.extrude(x, p, level, cache)),
        }
    }

    /// Instantiates a type scheme: copies the variables deeper than `above`
    /// to new variables at the current level, with their bounds.
    pub fn freshen(&mut self, t: &Ty, above: u32, map: &mut HashMap<V, V>) -> Ty {
        if self.level_of(t) <= above {
            return t.clone();
        }
        match t {
            Ty::Var(v) => {
                if let Some(nv) = map.get(v) {
                    return Ty::Var(*nv);
                }
                let nv = self.fresh_at(self.level);
                map.insert(*v, nv);
                let (lower, upper) = (self.var(*v).lower.clone(), self.var(*v).upper.clone());
                let lower = lower.iter().map(|b| self.freshen(b, above, map)).collect();
                let upper = upper.iter().map(|b| self.freshen(b, above, map)).collect();
                self.var_mut(nv).lower = lower;
                self.set_upper(nv, upper);
                Ty::Var(nv)
            }
            _ => self.map_children(t, Pol::Pos, &mut |s, x, _| s.freshen(x, above, map)),
        }
    }

    /// `t` with each type directly inside it changed by `f`, which is given
    /// the polarity of the place.
    pub fn map_children(
        &mut self,
        t: &Ty,
        pol: Pol,
        f: &mut dyn FnMut(&mut Solver, &Ty, Pol) -> Ty,
    ) -> Ty {
        match t {
            Ty::Var(_) | Ty::Never | Ty::Top => t.clone(),
            Ty::Con(h, args) => {
                let vs = self.variances(*h, args.len());
                let args = args
                    .iter()
                    .zip(vs)
                    .map(|(a, v)| f(self, a, pol.by(v)))
                    .collect();
                Ty::Con(*h, args)
            }
            Ty::Fn(fun) => Ty::Fn(Box::new(FnTy {
                params: fun
                    .params
                    .iter()
                    .map(|(l, p)| (l.clone(), f(self, p, pol.flip())))
                    .collect(),
                ret: f(self, &fun.ret, pol),
                err: f(self, &fun.err, pol),
                env: f(self, &fun.env, pol),
            })),
            Ty::Union(ms) => Ty::Union(ms.iter().map(|m| f(self, m, pol)).collect()),
            Ty::Inter(ms) => Ty::Inter(ms.iter().map(|m| f(self, m, pol)).collect()),
            Ty::Diff(a, h) => Ty::Diff(
                Box::new(f(self, a, pol)),
                h.iter().map(|m| f(self, m, Pol::Both)).collect(),
            ),
            Ty::Both(a, b) => Ty::Both(
                Box::new(f(self, a, Pol::Pos)),
                Box::new(f(self, b, Pol::Neg)),
            ),
        }
    }

    // ---- Unions, intersections, and differences (§4.4) ----

    /// `m1 | m2 | …`, with the members of one type constructor put together:
    /// arguments that go in the same direction by union, in the other
    /// direction by intersection, and arguments that do not change must be
    /// equal.
    pub fn union(&mut self, ms: Vec<Ty>) -> Ty {
        self.union_in(ms, false)
    }

    /// The union of environments (§9.2): two forms of one type constructor
    /// must have equal arguments, whatever their variance.
    pub fn env_union(&mut self, ms: Vec<Ty>) -> Ty {
        self.union_in(ms, true)
    }

    fn union_in(&mut self, ms: Vec<Ty>, env: bool) -> Ty {
        let mut flat = Vec::new();
        flatten_union(&ms, &mut flat);
        let mut out: Vec<Ty> = Vec::new();
        for m in flat {
            if matches!(m, Ty::Never) || out.contains(&m) {
                continue;
            }
            if matches!(m, Ty::Top) {
                return Ty::Top;
            }
            match m
                .head()
                .and_then(|h| out.iter().position(|o| o.head() == Some(h)))
            {
                Some(i) => {
                    let old = std::mem::replace(&mut out[i], Ty::Never);
                    out[i] = self.merge(old, m, true, env);
                }
                None => out.push(m),
            }
        }
        match out.len() {
            0 => Ty::Never,
            1 => out.pop().unwrap(),
            _ => Ty::Union(out),
        }
    }

    /// `m1 & m2 & …`: members with different type constructors give
    /// `Never`; members with one are put together the other way round
    /// (§4.4). `a & T` with a type variable stays.
    pub fn inter(&mut self, ms: Vec<Ty>) -> Ty {
        self.inter_in(ms, false)
    }

    fn inter_in(&mut self, ms: Vec<Ty>, env: bool) -> Ty {
        let mut flat = Vec::new();
        for m in ms {
            match m {
                Ty::Inter(inner) => flat.extend(inner),
                m => flat.push(m),
            }
        }
        let mut out: Vec<Ty> = Vec::new();
        let mut con: Option<Ty> = None;
        for m in flat {
            match m {
                Ty::Top => {}
                Ty::Never => return Ty::Never,
                Ty::Con(..) | Ty::Fn(_) => {
                    con = Some(match con.take() {
                        None => m,
                        Some(c) if c.head() == m.head() => self.merge(c, m, false, env),
                        Some(_) => return Ty::Never,
                    });
                }
                m if !out.contains(&m) => out.push(m),
                _ => {}
            }
        }
        out.extend(con);
        match out.len() {
            0 => Ty::Top,
            1 => out.pop().unwrap(),
            _ => Ty::Inter(out),
        }
    }

    /// Two members with one type constructor, as one: by union when `union`,
    /// and by intersection when not. In an environment (`env`), the
    /// arguments must be equal (§9.2).
    fn merge(&mut self, a: Ty, b: Ty, union: bool, env: bool) -> Ty {
        match (a, b) {
            (Ty::Con(h, xs), Ty::Con(_, ys)) if xs.len() == ys.len() => {
                if env {
                    for (x, y) in xs.iter().zip(&ys) {
                        self.equal(x, y, FailKind::EnvForms);
                    }
                    return Ty::Con(h, xs);
                }
                let vs = self.variances(h, xs.len());
                let args = xs
                    .into_iter()
                    .zip(ys)
                    .zip(vs)
                    .map(|((x, y), v)| self.merge_arg(x, y, v, union))
                    .collect();
                Ty::Con(h, args)
            }
            (Ty::Fn(f), Ty::Fn(g)) if f.params.len() == g.params.len() => {
                let f = *f;
                let g = *g;
                let params = f
                    .params
                    .into_iter()
                    .zip(g.params)
                    .map(|((l1, p), (l2, q))| {
                        let label = if l1 == l2 { l1 } else { None };
                        (label, self.merge_arg(p, q, Variance::Contra, union))
                    })
                    .collect();
                // The environments of two function types are put together
                // as environments (§4.4, §9.2).
                let env = if union {
                    self.union_in(vec![f.env, g.env], true)
                } else {
                    self.inter_in(vec![f.env, g.env], true)
                };
                Ty::Fn(Box::new(FnTy {
                    params,
                    ret: self.merge_arg(f.ret, g.ret, Variance::Co, union),
                    err: self.merge_arg(f.err, g.err, Variance::Co, union),
                    env,
                }))
            }
            (a, b) => {
                self.fail(FailKind::Arity, &a, &b);
                a
            }
        }
    }

    fn merge_arg(&mut self, x: Ty, y: Ty, v: Variance, union: bool) -> Ty {
        match v {
            Variance::Co | Variance::Bi if union => self.union(vec![x, y]),
            Variance::Co | Variance::Bi => self.inter(vec![x, y]),
            Variance::Contra if union => self.inter(vec![x, y]),
            Variance::Contra => self.union(vec![x, y]),
            Variance::Inv => {
                self.equal(&x, &y, FailKind::Invariant);
                x
            }
        }
    }

    /// `a - h1 - h2 - …` (§4.4). A member of `a` whose type constructor is
    /// in `h` is taken out, and it must be a subtype of that member of `h`.
    pub fn diff(&mut self, a: Ty, h: Vec<Ty>) -> Ty {
        self.diff_in(a, h, false)
    }

    /// `M - T` for an environment `M` (§4.8 rule 9): the member of `M` that
    /// is taken out must be equal to its member of `T` (§9.2).
    pub fn env_diff(&mut self, a: Ty, h: Vec<Ty>) -> Ty {
        self.diff_in(a, h, true)
    }

    fn diff_in(&mut self, a: Ty, h: Vec<Ty>, env: bool) -> Ty {
        let mut hs = Vec::new();
        flatten_union(&h, &mut hs);
        hs.retain(|m| !matches!(m, Ty::Never));
        if hs.is_empty() {
            return a;
        }
        match a {
            Ty::Never => Ty::Never,
            Ty::Union(ms) => {
                let parts = ms
                    .into_iter()
                    .map(|m| self.diff_in(m, hs.clone(), env))
                    .collect();
                self.union_in(parts, env)
            }
            Ty::Con(..) | Ty::Fn(_) => match hs.iter().find(|m| m.head() == a.head()) {
                Some(m) => {
                    let m = m.clone();
                    if env {
                        self.equal(&a, &m, FailKind::EnvForms);
                    } else {
                        self.constrain(&a, &m);
                    }
                    Ty::Never
                }
                None => a,
            },
            Ty::Diff(b, mut h0) => {
                for m in hs {
                    if !h0.contains(&m) {
                        h0.push(m);
                    }
                }
                Ty::Diff(b, h0)
            }
            a => Ty::Diff(Box::new(a), hs),
        }
    }
}

/// Whether two parts of bounds may hold function types, whose
/// environments must be checked when the bounds are put together.
fn needs_join(t: &Ty) -> bool {
    match t {
        Ty::Con(_, args) => !args.is_empty(),
        Ty::Never | Ty::Top => false,
        _ => true,
    }
}

pub(crate) fn flatten_union(ms: &[Ty], out: &mut Vec<Ty>) {
    for m in ms {
        match m {
            Ty::Union(inner) => flatten_union(inner, out),
            m => out.push(m.clone()),
        }
    }
}

/// §4.5 rule 7.
pub(crate) fn builtin_variance(b: BuiltinType) -> Vec<Variance> {
    use Variance::*;
    match b {
        BuiltinType::List => vec![Co],
        BuiltinType::Boundary => vec![Inv, Inv, Inv],
        BuiltinType::Continuation => vec![Contra, Co, Co, Co],
        _ => Vec::new(),
    }
}
