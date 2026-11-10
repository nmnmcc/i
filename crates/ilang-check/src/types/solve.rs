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
// The maps and sets of the solver hold small keys and are on the hot path:
// a fast hash is much quicker than the default one (docs/performance.md).
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::collections::HashMap as StdHashMap;

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
    /// The variables that have this one as a lower bound.
    pub into: Vec<V>,
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
    /// The variables that got a bound, in order, for the occurs check.
    pub touched: Vec<V>,
    /// The variables of the cycles that the occurs check has reported.
    reported: HashSet<V>,
    /// The pairs of bounds whose parts `add_bound` has put together.
    compared: HashSet<(Ty, Ty, bool, bool)>,
    pub variance: StdHashMap<TypeId, Vec<Variance>>,
    pub failures: Vec<Failure>,
    /// The names of the custom types, by `TypeId`, to show types.
    pub type_names: Vec<String>,
}

impl Solver {
    pub fn new(variance: StdHashMap<TypeId, Vec<Variance>>, type_names: Vec<String>) -> Solver {
        Solver {
            vars: Vec::new(),
            level: 0,
            cache: HashSet::default(),
            compared: HashSet::default(),
            touched: Vec::new(),
            reported: HashSet::default(),
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
            into: Vec::new(),
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

    /// Sets the lower bounds of a new variable.
    fn set_lower(&mut self, v: V, lower: Vec<Ty>) {
        for l in &lower {
            if let Ty::Var(w) = l {
                self.var_mut(*w).into.push(v);
            }
        }
        self.var_mut(v).lower = lower;
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
        // §4.5 rule 2: these are inclusions, not recursive bounds such as
        // `n <: n | S(n)`. Recording them would invent an occurs cycle.
        if matches!(r, Ty::Union(ms) if ms.contains(l))
            || matches!(l, Ty::Inter(ms) if ms.contains(r))
        {
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
            (Ty::Var(_), Ty::Var(v)) if env && self.level_of(l) <= self.var(*v).level => {
                // Collect requirements in the receiving row. An upper
                // bound on its member can lead back through a service's
                // invariant argument: n <: row, S(n) <: row is finite.
                self.add_bound(*v, l.clone(), true, env);
                for ub in self.var(*v).upper.clone() {
                    self.constrain_in(l, &ub, env);
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
                let r2 = self.extrude(r, Pol::Neg, level, &mut HashMap::default());
                self.constrain_in(l, &r2, env);
            }
            (_, Ty::Var(v)) => {
                let level = self.var(*v).level;
                let l2 = self.extrude(l, Pol::Pos, level, &mut HashMap::default());
                self.constrain_in(&l2, r, env);
            }
            (Ty::Inter(ms), _) => {
                // `a & T <: R` is true when `T <: R` or when `a <: R`
                // (§4.5 rule 2). The members that are not type variables
                // are tried first; when they fail, each variable is tried.
                // Only the members with a type constructor are put together
                // into one: the others (variables, differences, unions that
                // stay) are tried one by one, so that the type tried is
                // always smaller than `l`.
                let (conc, vars): (Vec<Ty>, Vec<Ty>) = ms
                    .iter()
                    .cloned()
                    .partition(|m| matches!(m, Ty::Con(..) | Ty::Fn(_)));
                let mut tries = Vec::new();
                if !conc.is_empty() {
                    tries.push(self.inter(conc));
                }
                tries.extend(vars);
                let before = self.failures.len();
                let last = tries.len().saturating_sub(1);
                for (i, t) in tries.iter().enumerate() {
                    self.constrain_in(t, r, env);
                    if self.failures.len() == before || i == last {
                        break;
                    }
                    self.failures.truncate(before);
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
                        self.equal(x, y, FailKind::EnvForms);
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

    /// Adds a bound to a variable. Two bounds with the same type
    /// constructor are one member (§4.4): two upper bounds by intersection,
    /// two lower bounds by union. So arguments that do not change must be
    /// equal; in an environment, all arguments must be equal (§9.2); and
    /// function types must have the same number of parameters. This holds
    /// for every bound that the variable has through other variables, not
    /// only for its own: with `x <: y <: Cell(String)` and `x <: Cell(Int)`,
    /// `x` has both. Two lower bounds also have their parts put together,
    /// so that the environments of two function types are checked.
    ///
    /// The checks of two upper bounds only make arguments equal or check
    /// the number of parameters, which hold for all bounds when they hold
    /// for each with one other. Every two upper bounds with one type
    /// constructor that a variable has are already checked, so a new one
    /// is only checked with the first of them: else a variable with many
    /// upper bounds of one type constructor, as a parameter used at many
    /// calls, would make a number of constraints that grows with the
    /// square of their number.
    fn add_bound(&mut self, v: V, t: Ty, lower: bool, env: bool) {
        self.touched.push(v);
        let level = self.var(v).level;
        // The variables whose bounds now have `t` too, and their bounds.
        let preds = self.reach(vec![v], !lower).0;
        let mut others = self.reach(preds, lower).1;
        if !lower {
            let mut heads = HashSet::default();
            others.retain(|b| heads.insert(b.head()));
        }
        let new = match &t {
            Ty::Var(u) => self.reach(vec![*u], lower).1,
            Ty::Con(..) | Ty::Fn(_) => vec![t.clone()],
            _ => Vec::new(),
        };
        for a in &new {
            for b in &others {
                self.compare(a, b, lower, env, level);
            }
        }
        let s = self.var_mut(v);
        let bounds = if lower { &mut s.lower } else { &mut s.upper };
        if !bounds.contains(&t) {
            bounds.push(t.clone());
            if let Ty::Var(w) = &t {
                if lower {
                    self.var_mut(*w).into.push(v);
                } else {
                    self.var_mut(*w).from.push(v);
                }
            }
        }
    }

    /// The variables reached from `starts` through bounds that are
    /// variables, upward (`x <: u`) or downward (`w <: x`) as `lower` says,
    /// and the bounds of those variables that are not variables.
    fn reach(&self, starts: Vec<V>, lower: bool) -> (Vec<V>, Vec<Ty>) {
        let mut seen: Vec<V> = Vec::new();
        let mut visited: HashSet<V> = HashSet::default();
        let mut found: HashSet<&Ty> = HashSet::default();
        let mut stack = starts;
        let mut out = Vec::new();
        while let Some(x) = stack.pop() {
            if !visited.insert(x) {
                continue;
            }
            seen.push(x);
            let st = self.var(x);
            let (bounds, back) = if lower {
                (&st.lower, &st.from)
            } else {
                (&st.upper, &st.into)
            };
            for b in bounds {
                match b {
                    Ty::Var(u) => stack.push(*u),
                    Ty::Con(..) | Ty::Fn(_) if found.insert(b) => out.push(b.clone()),
                    _ => {}
                }
            }
            stack.extend(back.iter().copied());
        }
        (seen, out)
    }

    /// The checks of `add_bound` for two bounds on one side of a variable.
    fn compare(&mut self, a: &Ty, b: &Ty, lower: bool, env: bool, level: u32) {
        if a == b || a.head() != b.head() {
            return;
        }
        // Joins are made once for each pair; the other checks give their
        // problems at each place.
        let first = !self.compared.contains(&(b.clone(), a.clone(), lower, env))
            && self.compared.insert((a.clone(), b.clone(), lower, env));
        let lower = lower && first;
        match (a, b) {
            (Ty::Fn(f), Ty::Fn(g)) => {
                if g.params.len() != f.params.len() {
                    // All function types are one head; their parameters
                    // must agree in number to be put together.
                    self.fail(FailKind::Arity, b, a);
                } else if lower {
                    self.join(&f.ret, &g.ret, level, false);
                    self.join(&f.err, &g.err, level, false);
                    self.join(&f.env, &g.env, level, true);
                }
            }
            (Ty::Con(h, xs), Ty::Con(_, ys)) if xs.len() == ys.len() => {
                let vs = self.variances(*h, xs.len());
                for ((x, y), var) in xs.iter().zip(ys).zip(&vs) {
                    if env {
                        self.equal(x, y, FailKind::EnvForms);
                    } else if *var == Variance::Inv {
                        self.equal(x, y, FailKind::Invariant);
                    } else if *var == Variance::Co && lower {
                        self.join(x, y, level, false);
                    }
                }
            }
            _ => {}
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
                // `H | R`, flat and with no member twice, so that the same
                // constraint is not made again in a bigger form.
                let mut ms = Vec::new();
                flatten_union(&[Ty::Union(h.to_vec()), r.clone()], &mut ms);
                let mut out: Vec<Ty> = Vec::new();
                for m in ms {
                    if !out.contains(&m) {
                        out.push(m);
                    }
                }
                self.constrain_in(a, &Ty::Union(out), env);
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
                    self.set_lower(nv, lower);
                }
                if pol != Pol::Pos {
                    self.var_mut(*v).lower.push(Ty::Var(nv));
                    self.var_mut(nv).into.push(*v);
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
    pub fn freshen(&mut self, t: &Ty, above: u32, map: &mut StdHashMap<V, V>) -> Ty {
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
                self.set_lower(nv, lower);
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
        let before = self.failures.len();
        let t = self.union_in(ms, true);
        if let Ty::Union(ms) = &t
            && self.failures.len() == before
            && ms.iter().any(Ty::is_varlike)
        {
            // Check forms hidden behind a row variable too (§9.2). The
            // collector adds no requirement to the returned union; it
            // makes add_bound compare all its services, also later ones.
            let row = self.fresh();
            for m in ms {
                self.constrain_env(m, &row);
            }
        }
        t
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
                Some(i) if env => {
                    // Two forms of a service: their arguments must be equal
                    // (§9.2). When they are not, both stay, as they are.
                    let before = self.failures.len();
                    let old = std::mem::replace(&mut out[i], Ty::Never);
                    out[i] = self.merge(old, m.clone(), true, env);
                    if self.failures.len() > before {
                        out.push(m);
                    }
                }
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
        // `{ A | B } & C` is `A & C | B & C` (§4.4), for a union whose
        // members all have a type constructor. A union with a variable as a
        // member stays: its parts could not be told apart.
        if let Some(i) = flat.iter().position(|m| match m {
            Ty::Union(xs) => xs.iter().all(|x| x.head().is_some()),
            _ => false,
        }) && flat.len() > 1
        {
            let Ty::Union(xs) = flat.remove(i) else {
                unreachable!()
            };
            let parts = xs
                .into_iter()
                .map(|x| {
                    let mut a = flat.clone();
                    a.push(x);
                    self.inter_in(a, env)
                })
                .collect();
            return self.union_in(parts, env);
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
                self.diff_bound(&b, &hs, env);
                for m in hs {
                    if !h0.contains(&m) {
                        h0.push(m);
                    }
                }
                Ty::Diff(b, h0)
            }
            a => {
                self.diff_bound(&a, &hs, env);
                Ty::Diff(Box::new(a), hs)
            }
        }
    }

    /// `t` with each `Top` in it changed to a new variable at `level`.
    pub fn no_top(&mut self, t: &Ty, level: u32) -> Ty {
        match t {
            Ty::Top => Ty::Var(self.fresh_at(level)),
            _ => self.map_children(t, Pol::Pos, &mut |s, x, _| s.no_top(x, level)),
        }
    }

    /// §4.4: in `a - H`, a value of `a` whose type constructor is in `H`
    /// must be in that member of `H` (equal to it, in an environment). This
    /// is a constraint as soon as the difference is made: `a <: H | r`,
    /// with a new variable `r` for the other values.
    pub fn diff_bound(&mut self, a: &Ty, hs: &[Ty], env: bool) {
        if !matches!(a, Ty::Var(_)) {
            return;
        }
        let r = Ty::Var(self.fresh_at(self.level_of(a)));
        let mut ms = hs.to_vec();
        ms.push(r);
        self.constrain_in(a, &Ty::Union(ms), env);
    }
}

impl Solver {
    /// §4.7: whether a type has itself inside it through a type variable:
    /// from one of `roots`, as an output or as an input, the bounds that
    /// coalescing follows (lower bounds of outputs, upper bounds of inputs)
    /// come back to it through a type constructor. A cycle is reported
    /// once.
    pub fn occurs(&mut self, roots: Vec<V>) -> bool {
        // The search of `occurs_from` goes over the bounds reached from each
        // root, so it costs the size of that graph for every root; a binding
        // with many variables reaching a large graph would take time
        // quadratic in it. `first_cycle` finds the same first root with one
        // pass over the graph; the search then runs for that root only, so
        // that the variables marked as reported are the same.
        let mut cycle = self.first_cycle(&roots);
        if cycle.is_some() {
            // A constraint graph can have redundant constructor paths
            // even when its type is finite. Keep the usual fast path.
            self.normalize_bounds(&roots);
            cycle = self.first_cycle(&roots);
        }
        match cycle {
            Some(r) => {
                let found = self.occurs_from(vec![r]);
                debug_assert!(found, "the occurs check of a root disagrees");
                found
            }
            None => false,
        }
    }

    /// Remove redundant bounds before deciding that a type is recursive.
    /// If T <: v <: T (or v <: Never), v is exactly T (§4.5). Its old
    /// bounds have already been propagated through T by constrain_in.
    /// Otherwise v <: u implies v <: u | T, also through variable chains.
    /// These rewrites preserve every solution; they introduce no variables
    /// and use only finite reachability searches.
    fn normalize_bounds(&mut self, roots: &[V]) {
        let mut done = HashSet::default();
        for &v in roots {
            if !done.insert(v) {
                continue;
            }
            let upper = self.var(v).upper.clone();
            let lower = self.reach(vec![v], true).1;
            let exact = upper.iter().find(|t| match t {
                Ty::Never => true,
                Ty::Con(..) | Ty::Fn(_) => lower.contains(t),
                Ty::Union(ms) => ms.iter().all(|m| lower.contains(m)),
                _ => false,
            });
            if let Some(t) = exact {
                let old_lower = std::mem::take(&mut self.var_mut(v).lower);
                let old_upper = std::mem::take(&mut self.var_mut(v).upper);
                for b in old_lower {
                    if let Ty::Var(u) = b {
                        self.var_mut(u).into.retain(|w| *w != v);
                    }
                }
                for b in old_upper {
                    if let Ty::Var(u) = b {
                        self.var_mut(u).from.retain(|w| *w != v);
                    }
                }
                self.var_mut(v).lower = vec![t.clone()];
                self.var_mut(v).upper = vec![t.clone()];
            } else {
                let (vars, _) = self.reach(vec![v], false);
                self.var_mut(v).upper.retain(|b| {
                    !matches!(b, Ty::Union(ms) if ms.iter().any(|m| matches!(m, Ty::Var(u) if vars.contains(u))))
                });
            }
        }
    }

    /// The search of the occurs check, one root at a time.
    fn occurs_from(&mut self, roots: Vec<V>) -> bool {
        let mut done: HashSet<V> = HashSet::default();
        for r in roots {
            if !done.insert(r) || self.reported.contains(&r) {
                continue;
            }
            for pol in [true, false] {
                let mut seen: HashSet<(V, bool, bool)> = HashSet::default();
                let mut stack = vec![(r, pol, false)];
                while let Some((v, p, con)) = stack.pop() {
                    if !seen.insert((v, p, con)) {
                        continue;
                    }
                    let st = self.var(v);
                    let bounds = if p { &st.lower } else { &st.upper };
                    let mut out = Vec::new();
                    for b in bounds {
                        bound_vars(self, b, p, con, &mut out);
                    }
                    for (u, q, c) in out {
                        if u == r && c {
                            self.reported.insert(r);
                            self.reported.extend(seen.iter().map(|x| x.0));
                            return true;
                        }
                        stack.push((u, q, c));
                    }
                }
            }
        }
        false
    }

    /// The first of `roots` (not yet reported) for which `occurs_from`
    /// finds a cycle. The graph has a node for each variable and polarity,
    /// and an edge for each variable in a bound, which is marked when it is
    /// inside a type constructor. A root `r` has a cycle when a path with a
    /// marked edge goes from `(r, p)` to `(r, q)`, for some `p` and `q`.
    /// With the strongly connected components of the graph, that is: the
    /// component of `(r, p)` has a marked edge inside it, or a path of
    /// components with a marked edge goes from it to the component of
    /// `(r, not p)`.
    fn first_cycle(&self, roots: &[V]) -> Option<V> {
        let mut order: Vec<V> = Vec::new();
        let mut done: HashSet<V> = HashSet::default();
        for r in roots {
            if done.insert(*r) && !self.reported.contains(r) {
                order.push(*r);
            }
        }
        if order.is_empty() {
            return None;
        }
        let g = Graph::build(self, &order);
        // The component of each root and polarity, and of the other
        // polarity, which a path with a marked edge must go to. The roots
        // are done in groups, so that the sets of targets stay small.
        let ends: Vec<[(usize, usize); 2]> = order
            .iter()
            .map(|r| {
                let plus = g.comp[g.index[&(*r, true)]];
                let minus = g.comp[g.index[&(*r, false)]];
                [(plus, minus), (minus, plus)]
            })
            .collect();
        for (chunk, roots) in ends.chunks(512).zip(order.chunks(512)) {
            let targets: Vec<usize> = chunk.iter().flat_map(|e| [e[0].1, e[1].1]).collect();
            let reach = g.marked_paths(&targets);
            for (k, r) in roots.iter().enumerate() {
                for (j, (a, _)) in chunk[k].iter().enumerate() {
                    let t = 2 * k + j;
                    if g.marked[*a] || reach[*a][t / 64] & (1 << (t % 64)) != 0 {
                        return Some(*r);
                    }
                }
            }
        }
        None
    }
}

/// The graph of the occurs check, from some roots, with its strongly
/// connected components (`Solver::first_cycle`).
struct Graph {
    /// The number of each node: a variable with a polarity.
    index: HashMap<(V, bool), usize>,
    /// The edges of each node, to other nodes, with their mark.
    edges: Vec<Vec<(usize, bool)>>,
    /// The component of each node.
    comp: Vec<usize>,
    /// Whether a component has a marked edge inside it.
    marked: Vec<bool>,
    /// The nodes of each component.
    members: Vec<Vec<usize>>,
}

impl Graph {
    fn build(s: &Solver, roots: &[V]) -> Graph {
        let mut g = Graph {
            index: HashMap::default(),
            edges: Vec::new(),
            comp: Vec::new(),
            marked: Vec::new(),
            members: Vec::new(),
        };
        // Tarjan's algorithm, without recursion. A node is added (with its
        // edges) the first time it is reached.
        let mut low: Vec<usize> = Vec::new();
        let mut on_stack: Vec<bool> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        let mut calls: Vec<(usize, usize)> = Vec::new();
        let add = |g: &mut Graph, low: &mut Vec<usize>, on: &mut Vec<bool>, key: (V, bool)| {
            if let Some(&n) = g.index.get(&key) {
                return (n, false);
            }
            let n = g.edges.len();
            g.index.insert(key, n);
            g.edges.push(Vec::new());
            g.comp.push(usize::MAX);
            low.push(n);
            on.push(false);
            (n, true)
        };
        // The edges of a node as variables, until the search numbers them.
        let mut pending: Vec<Vec<(V, bool, bool)>> = Vec::new();
        let mut next_comp = 0;
        for r in roots {
            for pol in [true, false] {
                let (start, new) = add(&mut g, &mut low, &mut on_stack, (*r, pol));
                if !new {
                    continue;
                }
                pending.push(edge_vars(s, (*r, pol)));
                stack.push(start);
                on_stack[start] = true;
                calls.push((start, 0));
                while let Some(&mut (n, ref mut i)) = calls.last_mut() {
                    let at = *i;
                    if at < pending[n].len() {
                        *i += 1;
                        let (u, q, c) = pending[n][at];
                        let (m, new) = add(&mut g, &mut low, &mut on_stack, (u, q));
                        g.edges[n].push((m, c));
                        if new {
                            pending.push(edge_vars(s, (u, q)));
                            stack.push(m);
                            on_stack[m] = true;
                            calls.push((m, 0));
                        } else if on_stack[m] {
                            low[n] = low[n].min(low[m]);
                        }
                        continue;
                    }
                    calls.pop();
                    if let Some(&(parent, _)) = calls.last() {
                        low[parent] = low[parent].min(low[n]);
                    }
                    if low[n] == n {
                        loop {
                            let m = stack.pop().expect("the node is on the stack");
                            on_stack[m] = false;
                            g.comp[m] = next_comp;
                            if m == n {
                                break;
                            }
                        }
                        next_comp += 1;
                    }
                }
            }
        }
        g.marked = vec![false; next_comp];
        g.members = vec![Vec::new(); next_comp];
        for (n, es) in g.edges.iter().enumerate() {
            g.members[g.comp[n]].push(n);
            for &(m, c) in es {
                if c && g.comp[n] == g.comp[m] {
                    g.marked[g.comp[n]] = true;
                }
            }
        }
        g
    }

    /// For each component, the targets (indexes into `targets`, as bits)
    /// that a path from it with a marked edge goes to. Components are
    /// numbered in the order Tarjan's algorithm finishes them, so an edge
    /// between two components goes to a lower number, and the components
    /// can be done in order.
    fn marked_paths(&self, targets: &[usize]) -> Vec<Vec<u64>> {
        let words = targets.len().div_ceil(64);
        let n = self.marked.len();
        // `any`: the targets reached by any path; `con`: with a marked edge.
        let mut any = vec![vec![0u64; words]; n];
        let mut con = vec![vec![0u64; words]; n];
        for (t, &c) in targets.iter().enumerate() {
            any[c][t / 64] |= 1 << (t % 64);
        }
        for c in 0..n {
            let mut a = std::mem::take(&mut any[c]);
            let mut m = std::mem::take(&mut con[c]);
            for &node in &self.members[c] {
                for &(to, e) in &self.edges[node] {
                    let d = self.comp[to];
                    if d == c {
                        continue;
                    }
                    for w in 0..words {
                        a[w] |= any[d][w];
                        m[w] |= con[d][w] | if e { any[d][w] } else { 0 };
                    }
                }
            }
            if self.marked[c] {
                for w in 0..words {
                    m[w] |= a[w];
                }
            }
            any[c] = a;
            con[c] = m;
        }
        con
    }
}

/// The edges of a node of the occurs check graph, as variables.
fn edge_vars(s: &Solver, (v, p): (V, bool)) -> Vec<(V, bool, bool)> {
    let st = s.var(v);
    let bounds = if p { &st.lower } else { &st.upper };
    let mut out = Vec::new();
    for b in bounds {
        bound_vars(s, b, p, false, &mut out);
    }
    out
}

/// The variables in a bound at a place of polarity `pos`, each with its
/// polarity and whether it is inside a type constructor, as coalescing
/// goes into them.
fn bound_vars(s: &Solver, t: &Ty, pos: bool, con: bool, out: &mut Vec<(V, bool, bool)>) {
    match t {
        Ty::Var(v) => out.push((*v, pos, con)),
        Ty::Con(h, args) => {
            let vs = s.variances(*h, args.len());
            for (a, v) in args.iter().zip(vs) {
                match v {
                    Variance::Co | Variance::Bi => bound_vars(s, a, pos, true, out),
                    Variance::Contra => bound_vars(s, a, !pos, true, out),
                    Variance::Inv => {
                        bound_vars(s, a, pos, true, out);
                        bound_vars(s, a, !pos, true, out);
                    }
                }
            }
        }
        Ty::Fn(f) => {
            for p in &f.params {
                bound_vars(s, &p.1, !pos, true, out);
            }
            for x in [&f.ret, &f.err] {
                bound_vars(s, x, pos, true, out);
            }
            // Environment arguments are compared as equal.
            env_vars(s, &f.env, pos, out);
        }
        Ty::Union(ms) | Ty::Inter(ms) => ms.iter().for_each(|a| bound_vars(s, a, pos, con, out)),
        Ty::Diff(a, h) => {
            bound_vars(s, a, pos, con, out);
            h.iter().for_each(|x| bound_vars(s, x, pos, con, out));
        }
        Ty::Both(a, b) => {
            bound_vars(s, a, true, con, out);
            bound_vars(s, b, false, con, out);
        }
        Ty::Never | Ty::Top => {}
    }
}

/// `bound_vars` for an environment: the arguments of its members are on
/// both sides.
fn env_vars(s: &Solver, t: &Ty, pos: bool, out: &mut Vec<(V, bool, bool)>) {
    match t {
        Ty::Con(_, args) => {
            for a in args {
                bound_vars(s, a, true, true, out);
                bound_vars(s, a, false, true, out);
            }
        }
        Ty::Union(ms) | Ty::Inter(ms) => ms.iter().for_each(|a| env_vars(s, a, pos, out)),
        Ty::Diff(a, h) => {
            env_vars(s, a, pos, out);
            h.iter().for_each(|x| bound_vars(s, x, pos, true, out));
        }
        t => bound_vars(s, t, pos, true, out),
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
