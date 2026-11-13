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

/// Row identity is persistent. Error rows preserve scalar set inclusion;
/// service environments additionally require equal member value sets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Mode {
    #[default]
    Value,
    Row,
    Env,
}

/// Small parameter descriptions are traversed without allocating a vector.
pub(crate) enum ParamModes<T> {
    Repeated(std::iter::RepeatN<T>),
    Inline(std::iter::Take<std::array::IntoIter<T, 4>>),
    Stored(std::vec::IntoIter<T>),
}
impl<T: Copy + Eq> ParamModes<T> {
    fn repeated(value: T, n: usize) -> Self {
        Self::Repeated(std::iter::repeat_n(value, n))
    }
    fn from_slice(ms: &[T]) -> Self {
        let Some(&first) = ms.first() else {
            return Self::Stored(Vec::new().into_iter());
        };
        if ms.iter().all(|m| *m == first) {
            return Self::repeated(first, ms.len());
        }
        if ms.len() <= 4 {
            let mut values = [first; 4];
            values[..ms.len()].copy_from_slice(ms);
            Self::Inline(values.into_iter().take(ms.len()))
        } else {
            // Constraints may mutate the solver while this iterator lives.
            let stored = ms.to_vec();
            Self::Stored(stored.into_iter())
        }
    }
}
impl<T: Clone> Iterator for ParamModes<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self {
            Self::Repeated(ms) => ms.next(),
            Self::Inline(ms) => ms.next(),
            Self::Stored(ms) => ms.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Repeated(ms) => ms.size_hint(),
            Self::Inline(ms) => ms.size_hint(),
            Self::Stored(ms) => ms.size_hint(),
        }
    }
}
impl<T: Clone> ExactSizeIterator for ParamModes<T> {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VarState {
    pub level: u32,
    /// A row keeps rule 4's member comparison even inside type arguments.
    pub row: Mode,
    /// A difference imposes a same-form obligation even if an annotation
    /// hides the difference from the result type.
    pub difference: bool,
    pub handled: Vec<Ty>,
    /// A deferred need meet, with the service forms used for E0515.
    pub need_meet: Option<(Ty, Ty)>,
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
    /// Only an Int/String value-set inclusion failed.
    Values,
    /// Two forms of a type constructor whose parameter does not change, with
    /// arguments that cannot be equal (§4.4).
    Invariant,
    /// Two forms of one type constructor in an environment, with arguments
    /// that cannot be equal (§9.2).
    EnvForms,
    /// A need meet has incompatible concrete upper bounds (§9.2).
    NeedMeet,
    /// A value could go to more than one type variable of a union (§4.4).
    Ambiguous,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Solver {
    observed_shape: bool,
    /// Allocation ownership observed during a demanded source group.
    pub observation: Option<(usize, HashSet<V>, std::cell::RefCell<HashSet<V>>)>,
    pub vars: Vec<VarState>,
    pub(super) rigid_boundary: usize,
    rigid_active: HashSet<(Ty, Ty, Mode)>,
    rigid_proved: HashSet<(Ty, Ty, Mode)>,
    /// Peak concrete bounds of one head, on either side of one variable.
    #[cfg(test)]
    peak_bound_count: usize,
    /// The level of new variables: one more than the level of the binding
    /// whose value is being typed, for a binding that is generalized.
    pub level: u32,
    /// The constraints with a variable that are already made, each with
    /// whether it was made between environments.
    cache: HashSet<(Ty, Ty, Mode)>,
    /// The variables that got a bound, in order, for the occurs check.
    pub touched: Vec<V>,
    /// Counts the changes of bounds and inputs that `touched` does not
    /// record.
    pub rewrites: u64,
    /// The variables of the cycles that the occurs check has reported.
    reported: HashSet<V>,
    /// The pairs of bounds whose parts `add_bound` has put together.
    compared: HashSet<(Ty, Ty, bool, Mode)>,
    pub variance: StdHashMap<TypeId, Vec<Variance>>,
    /// Internal comparison wrappers have exactly the values of their one
    /// argument, tagged with a disjoint identity.
    pub(super) comparison_wrappers: HashSet<TypeId>,
    pub(super) has_handlers: bool,
    pub(super) has_need_meets: bool,
    pub row_params: StdHashMap<TypeId, Vec<Mode>>,
    pub failures: Vec<Failure>,
    /// The names of the custom types, by `TypeId`, to show types.
    pub type_names: Vec<String>,
    /// The declared aliases whose expansion is a union, to show such a
    /// union by its name.
    pub alias_folds: std::sync::Arc<Vec<super::display::AliasFold>>,
    /// A failed coalescence is diagnostic recovery, never an unknown type
    /// that may silently make a program pass. The checker drains this at
    /// binding and artifact boundaries, including known-type conversions.
    pub coalescing_failed: std::cell::Cell<bool>,
    /// Scratch marks of `reach`; not part of the solver's state.
    reach_marks: ReachMarks,
}

/// The variables that one `reach` has visited: a mark per variable, which
/// is the number of that `reach`, so no set is made or cleared per call.
/// Not part of the solver's state: equal, cloned and shown as empty.
#[derive(Default)]
struct ReachMarks(std::cell::RefCell<(u32, Vec<u32>)>);

impl ReachMarks {
    /// Starts a new walk over `n` variables.
    fn start(&self, n: usize) -> Visited<'_> {
        let mut marks = self.0.borrow_mut();
        let (stamp, ms) = &mut *marks;
        if *stamp == u32::MAX {
            *stamp = 0;
            ms.iter_mut().for_each(|m| *m = 0);
        }
        *stamp += 1;
        if ms.len() < n {
            ms.resize(n, 0);
        }
        Visited(marks)
    }
}

struct Visited<'a>(std::cell::RefMut<'a, (u32, Vec<u32>)>);

impl Visited<'_> {
    /// Marks `v`; false when it was marked already in this walk.
    fn insert(&mut self, v: V) -> bool {
        let (stamp, ms) = &mut *self.0;
        let i = v.0 as usize;
        if i >= ms.len() {
            ms.resize(i + 1, 0);
        }
        let new = ms[i] != *stamp;
        ms[i] = *stamp;
        new
    }
}

impl Clone for ReachMarks {
    fn clone(&self) -> Self {
        ReachMarks::default()
    }
}

impl PartialEq for ReachMarks {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for ReachMarks {}

impl std::fmt::Debug for ReachMarks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReachMarks")
    }
}

/// One member per scalar head, joined before the ordinary union loop.
/// Display uses the same batch operation; no solver state is consulted.
pub(crate) fn join_literals(ms: Vec<Ty>) -> Vec<Ty> {
    let mut groups: [Option<(usize, Vec<super::lits::ValueSet>)>; 2] = [None, None];
    let mut out = Vec::new();
    for m in ms {
        let i = match m.scalar() {
            Some(BuiltinType::Int) => Some(0),
            Some(BuiltinType::String) => Some(1),
            _ => None,
        };
        if let Some(i) = i {
            if let Some((_, sets)) = &mut groups[i] {
                sets.push(m.values().clone());
                continue;
            }
            groups[i] = Some((out.len(), vec![m.values().clone()]));
        }
        out.push(m);
    }
    for (i, sets) in groups.into_iter().flatten() {
        out[i] = out[i].with_values(Some(super::lits::ValueSet::join_all(&sets)));
    }
    out
}

impl Solver {
    /// An interface needs the graph and retained constraints, not transient
    /// proof caches or diagnostic observations from the inference run.
    pub(super) fn scheme_snapshot(&self) -> Solver {
        let mut snapshot = Solver::new(self.variance.clone(), self.type_names.clone());
        snapshot.vars = self.vars.clone();
        snapshot.row_params = self.row_params.clone();
        snapshot.has_handlers = self.has_handlers;
        snapshot.comparison_wrappers = self.comparison_wrappers.clone();
        snapshot.alias_folds = self.alias_folds.clone();
        snapshot
    }
    /// Discard inference caches: a cached solved goal is not a rigid proof.
    pub(super) fn freeze(&mut self) {
        self.rigid_boundary = self.vars.len();
        self.cache.clear();
        self.compared.clear();
        self.rigid_active.clear();
        self.rigid_proved.clear();
    }
    fn frozen(&self, t: &Ty) -> bool {
        matches!(t, Ty::Var(v) if (v.0 as usize) < self.rigid_boundary)
    }
    fn rigid_bounds(&self, root: V, lower: bool) -> (HashSet<V>, Vec<Ty>) {
        let mut seen = HashSet::default();
        let mut work = vec![root];
        let mut bounds = Vec::new();
        while let Some(v) = work.pop() {
            if !seen.insert(v) {
                continue;
            }
            let st = self.var(v);
            for t in if lower { &st.lower } else { &st.upper } {
                if let Ty::Var(w) = t {
                    if (w.0 as usize) < self.rigid_boundary {
                        work.push(*w);
                    }
                } else {
                    bounds.push(t.clone());
                }
            }
            work.extend(
                if lower { &st.from } else { &st.into }
                    .iter()
                    .copied()
                    .filter(|v| (v.0 as usize) < self.rigid_boundary),
            );
        }
        (seen, bounds)
    }
    fn rigid_constraint(&mut self, l: &Ty, r: &Ty, mode: Mode) -> bool {
        let mode = mode.max(self.mode(l)).max(self.mode(r));
        if !self.frozen(l) && !self.frozen(r) {
            return false;
        }
        if l == r || matches!(l, Ty::Never) || matches!(r, Ty::Top) {
            return true;
        }
        // A direct fresh edge records a bound only on the fresh endpoint.
        if let Ty::Var(v) = r
            && !self.frozen(r)
        {
            self.mark_mode(r, mode);
            if self.var(*v).lower.contains(l) {
                return true;
            }
            if self.add_bound(*v, l.clone(), true, mode) {
                for ub in self.var(*v).upper.clone() {
                    self.constrain_in(l, &ub, mode);
                }
            }
            return true;
        }
        if let Ty::Var(v) = l
            && !self.frozen(l)
        {
            self.mark_mode(l, mode);
            if self.var(*v).upper.contains(r) {
                return true;
            }
            if self.add_bound(*v, r.clone(), false, mode) {
                for lb in self.var(*v).lower.clone() {
                    self.constrain_in(&lb, r, mode);
                }
            }
            return true;
        }
        let goal = (l.clone(), r.clone(), mode);
        if self.rigid_proved.contains(&goal) {
            return true;
        }
        if !self.rigid_active.insert(goal.clone()) {
            self.fail(FailKind::Mismatch, l, r);
            return true;
        }
        let mut trial = self.clone();
        let before = trial.failures.len();
        let proved = if let Ty::Var(v) = l {
            let (edges, bounds) = self.rigid_bounds(*v, false);
            if matches!(r, Ty::Var(w) if edges.contains(w)) {
                true
            } else if bounds.is_empty() {
                false
            } else {
                let upper = trial.inter(bounds);
                trial.constrain_in(&upper, r, mode);
                trial.failures.len() == before
            }
        } else if let Ty::Var(v) = r {
            let (_, bounds) = self.rigid_bounds(*v, true);
            let lower = trial.union(bounds);
            trial.constrain_in(l, &lower, mode);
            trial.failures.len() == before
        } else {
            unreachable!("rigid endpoint")
        };
        if proved {
            trial.rigid_active.remove(&goal);
            trial.rigid_proved.insert(goal);
            *self = trial;
        } else {
            self.rigid_active.remove(&goal);
            self.fail(FailKind::Mismatch, l, r);
        }
        true
    }
    pub fn new(variance: StdHashMap<TypeId, Vec<Variance>>, type_names: Vec<String>) -> Solver {
        Solver {
            coalescing_failed: std::cell::Cell::new(false),
            observed_shape: false,
            observation: None,
            vars: Vec::new(),
            rigid_boundary: 0,
            rigid_active: HashSet::default(),
            rigid_proved: HashSet::default(),
            #[cfg(test)]
            peak_bound_count: 0,
            level: 0,
            cache: HashSet::default(),
            compared: HashSet::default(),
            touched: Vec::new(),
            rewrites: 0,
            reported: HashSet::default(),
            variance,
            comparison_wrappers: HashSet::default(),
            row_params: StdHashMap::new(),
            has_handlers: false,
            has_need_meets: false,
            failures: Vec::new(),
            type_names,
            alias_folds: Default::default(),
            reach_marks: ReachMarks::default(),
        }
    }

    /// A scratch solver for closed types keeps constructor row modes.
    pub fn for_closed_types(&self) -> Solver {
        let mut solver = Solver::new(self.variance.clone(), self.type_names.clone());
        solver.row_params = self.row_params.clone();
        solver.alias_folds = self.alias_folds.clone();
        solver
    }

    fn observe(&self, v: V) {
        if let Some((boundary, fixed, seen)) = &self.observation
            && (v.0 as usize) < *boundary
            && !fixed.contains(&v)
        {
            seen.borrow_mut().insert(v);
        }
    }

    fn observe_type(&self, t: &Ty) {
        match t {
            Ty::Var(v) => self.observe(*v),
            Ty::Con(_, _, ts) | Ty::Union(ts) | Ty::Inter(ts) => {
                for t in ts {
                    self.observe_type(t);
                }
            }
            Ty::Fn(f) => {
                for t in [&f.param, &f.ret, &f.err, &f.env] {
                    self.observe_type(t);
                }
            }
            Ty::Diff(t, ts) => {
                self.observe_type(t);
                for t in ts {
                    self.observe_type(t);
                }
            }
            Ty::Both(a, b) => {
                self.observe_type(a);
                self.observe_type(b);
            }
            Ty::Never | Ty::Top => {}
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
        self.rewrites += 1;
        for v in &mut self.vars[start..] {
            v.input = true;
        }
    }

    pub fn fresh_at(&mut self, level: u32) -> V {
        self.vars.push(VarState {
            level,
            row: Mode::Value,
            difference: false,
            handled: Vec::new(),
            need_meet: None,
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
            if let Ty::Var(w) = u
                && (w.0 as usize) >= self.rigid_boundary
            {
                self.var_mut(*w).from.push(v);
            }
        }
        self.var_mut(v).upper = upper;
        #[cfg(test)]
        self.observe_bounds(v, false);
    }

    /// Sets the lower bounds of a new variable.
    fn set_lower(&mut self, v: V, lower: Vec<Ty>) {
        for l in &lower {
            if let Ty::Var(w) = l
                && (w.0 as usize) >= self.rigid_boundary
            {
                self.var_mut(*w).into.push(v);
            }
        }
        self.var_mut(v).lower = lower;
        #[cfg(test)]
        self.observe_bounds(v, true);
    }

    pub(super) fn var(&self, v: V) -> &VarState {
        &self.vars[v.0 as usize]
    }

    fn var_mut(&mut self, v: V) -> &mut VarState {
        assert!(
            (v.0 as usize) >= self.rigid_boundary,
            "typed expansion mutated frozen solver state"
        );
        &mut self.vars[v.0 as usize]
    }

    /// The variances of the parameters of a head (§4.5 rules 6 and 7).
    pub fn variances(&self, h: Head, n: usize) -> ParamModes<Variance> {
        if n == 0 {
            return ParamModes::repeated(Variance::Inv, 0);
        }
        match h {
            Head::Named(TypeRef::Custom(id)) => self.variance.get(&id).map_or_else(
                || ParamModes::repeated(Variance::Inv, n),
                |ms| ParamModes::from_slice(ms),
            ),
            Head::Named(TypeRef::Builtin(b)) => ParamModes::from_slice(builtin_variance_values(b)),
            Head::Shape(_) | Head::Fn => ParamModes::repeated(Variance::Co, n),
        }
    }

    pub(super) fn fail(&mut self, kind: FailKind, lhs: &Ty, rhs: &Ty) {
        if self
            .failures
            .iter()
            .any(|f| f.kind == kind && f.lhs == *lhs && f.rhs == *rhs)
        {
            return;
        }
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
            Ty::Con(_, _, args) => args.iter().map(|a| self.level_of(a)).max().unwrap_or(0),
            Ty::Fn(f) => [&f.param, &f.ret, &f.err, &f.env]
                .into_iter()
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
        self.constrain_in(l, r, Mode::Value);
    }

    /// Observed skeletons contain derived unions of existing arguments, not
    /// written choices. Reflexive inclusions need no bound insertion/routing.
    pub fn constrain_observed(&mut self, l: &Ty, r: &Ty) {
        self.observed_shape = true;
        self.constrain(l, r);
        self.observed_shape = false;
    }

    /// Makes `N` **in** `M` true for two environments (§4.5 rule 4): every
    /// member of `N` is a type variable that is a subtype of `M`, or has a
    /// member of `M` with the same type constructor that satisfies it as
    /// a value. Needs compare services in reverse (§4.5 rule 4).
    pub fn constrain_env(&mut self, l: &Ty, r: &Ty) {
        self.constrain_in(l, r, Mode::Env);
    }

    /// The row parameters of a constructor, determined from its definition.
    pub fn rows(&self, h: Head, n: usize) -> ParamModes<Mode> {
        if n == 0 {
            return ParamModes::repeated(Mode::Value, 0);
        }
        match h {
            Head::Named(TypeRef::Custom(id)) => self.row_params.get(&id).map_or_else(
                || ParamModes::repeated(Mode::Value, n),
                |ms| ParamModes::from_slice(ms),
            ),
            Head::Named(TypeRef::Builtin(BuiltinType::Continuation)) => {
                ParamModes::from_slice(&[Mode::Value, Mode::Value, Mode::Row, Mode::Env])
            }
            Head::Named(TypeRef::Builtin(BuiltinType::Boundary)) => {
                ParamModes::from_slice(&[Mode::Value, Mode::Row, Mode::Env])
            }
            _ => ParamModes::repeated(Mode::Value, n),
        }
    }

    /// Row identity belongs to variables. Upgrade their whole bound
    /// component and recheck existing bounds, including ones added before
    /// the variable was discovered to be a row.
    pub fn mark_row(&mut self, t: &Ty) {
        self.mark_mode(t, Mode::Env);
    }

    pub fn mark_mode(&mut self, t: &Ty, mode: Mode) {
        if matches!(t, Ty::Var(v) if (v.0 as usize) < self.rigid_boundary) {
            return;
        }
        if mode == Mode::Value
            || matches!(t, Ty::Var(v) if self.var(*v).row >= mode)
            || matches!(t, Ty::Con(..) | Ty::Fn(_) | Ty::Never | Ty::Top)
        {
            return;
        }
        if let Ty::Var(v) = t {
            let st = self.var(*v);
            if st.lower.is_empty()
                && st.upper.is_empty()
                && st.from.is_empty()
                && st.into.is_empty()
            {
                self.observe(*v);
                self.var_mut(*v).row = mode;
                return;
            }
        }
        let mut todo = vec![t.clone()];
        let mut added = Vec::new();
        while let Some(t) = todo.pop() {
            match t {
                Ty::Var(v) if (v.0 as usize) >= self.rigid_boundary && self.var(v).row < mode => {
                    self.observe(v);
                    self.var_mut(v).row = mode;
                    added.push(v);
                    let st = self.var(v);
                    todo.extend(st.lower.iter().chain(&st.upper).cloned());
                    todo.extend(st.from.iter().chain(&st.into).copied().map(Ty::Var));
                }
                Ty::Union(ms) | Ty::Inter(ms) => todo.extend(ms),
                Ty::Diff(a, _) => todo.push(*a),
                Ty::Both(a, b) => {
                    todo.push(*a);
                    todo.push(*b);
                }
                _ => {}
            }
        }
        for v in added {
            for b in self.var(v).lower.clone() {
                self.constrain_in(&b, &Ty::Var(v), mode);
            }
            for b in self.var(v).upper.clone() {
                self.constrain_in(&Ty::Var(v), &b, mode);
            }
        }
    }

    pub fn is_row(&self, t: &Ty) -> bool {
        match t {
            Ty::Var(v) => self.var(*v).row != Mode::Value,
            Ty::Union(ms) | Ty::Inter(ms) => ms.iter().any(|m| self.is_row(m)),
            Ty::Diff(a, _) => self.is_row(a),
            _ => false,
        }
    }

    fn mode(&self, t: &Ty) -> Mode {
        match t {
            Ty::Var(v) => self.var(*v).row,
            Ty::Union(ms) | Ty::Inter(ms) => {
                ms.iter().map(|m| self.mode(m)).max().unwrap_or_default()
            }
            Ty::Diff(a, _) => self.mode(a),
            _ => Mode::Value,
        }
    }

    pub fn constrain_row(&mut self, l: &Ty, r: &Ty) {
        self.constrain_in(l, r, Mode::Row);
    }

    /// Hidden difference constraints cannot be thrown away by rebuilding
    /// only the visible type. Keep the bounded scheme in this case.
    pub fn has_obligations(&self, t: &Ty) -> bool {
        if !self.has_handlers {
            return false;
        }
        let mut edges = Vec::new();
        bound_vars(self, t, true, false, &mut edges);
        let mut todo: Vec<_> = edges.drain(..).map(|e| e.0).collect();
        let mut seen = HashSet::default();
        while let Some(v) = todo.pop() {
            if !seen.insert(v) {
                continue;
            }
            let st = self.var(v);
            if st.difference {
                return true;
            }
            for b in st.lower.iter().chain(&st.upper) {
                bound_vars(self, b, true, false, &mut edges);
            }
            todo.extend(edges.drain(..).map(|e| e.0));
        }
        false
    }

    /// An obligation on a row also constrains each of its member rows.
    /// Store it there, so generalizing a member cannot lose a restriction
    /// that was reached only through a reverse inclusion link.
    fn handle(&mut self, v: V, h: &Ty) {
        // Error-row scalar subtraction removes values (§4.4, §4.5 rule 8).
        // It does not install a service with an equal value set.
        if self.var(v).row == Mode::Row && h.scalar().is_some() {
            return;
        }
        // There is nothing to retain for a constructor with just one form.
        if matches!(h, Ty::Con(_, values, args) if values.is_all() && args.is_empty())
            && h.scalar().is_none()
        {
            return;
        }
        if (v.0 as usize) < self.rigid_boundary && !self.var(v).handled.contains(h) {
            self.fail(FailKind::Mismatch, &Ty::Var(v), h);
            return;
        }
        // A row occurring inside its own directed handler cannot later take
        // that same service without a structural cycle. Check its known
        // members and keep the recursive callback row open. Other directed
        // handlers retain the obligation below for later instantiations.
        if self.var(v).row == Mode::Env
            && let Ty::Con(hh, _, args) = h
            && !args.is_empty()
            && self.variances(*hh, args.len()).all(|x| x != Variance::Inv)
            && super::typed_macro::variables(h)
                .iter()
                .any(|w| self.same_form(&Ty::Var(v), &Ty::Var(*w)))
        {
            for b in self.reach(vec![v], true).1 {
                if b.head() == h.head() {
                    self.constrain_in(&b, h, Mode::Env);
                }
            }
            return;
        }
        self.has_handlers = true;
        for w in self.reach(vec![v], true).0 {
            let installed = self.var(w).handled.contains(h);
            if (w.0 as usize) < self.rigid_boundary || installed && self.var(w).row != Mode::Env {
                continue;
            }
            if !installed {
                self.observe(w);
                self.var_mut(w).difference = true;
                self.var_mut(w).handled.push(h.clone());
            }
            if self.var(w).row == Mode::Env {
                // Rule B: a need for `h`'s constructor that reaches the
                // row goes to the handler `h`, which must provide it.
                for b in self.reach(vec![w], true).1 {
                    if b.head() == h.head() {
                        self.constrain_in(&b, h, Mode::Env);
                    }
                }
                continue;
            }
            let mut bounds = self.reach(vec![w], true).1;
            bounds.extend(self.reach(vec![w], false).1);
            for b in bounds {
                if b.head() == h.head() {
                    self.equal(&b, h, FailKind::EnvForms);
                }
            }
        }
    }

    /// Equality used only to prove a scheme's handler witness. Variable
    /// inclusions must hold both ways; row member order is immaterial.
    fn same_form(&self, a: &Ty, b: &Ty) -> bool {
        self.same_form_in(a, b, &mut HashSet::default())
    }

    fn same_form_in(&self, a: &Ty, b: &Ty, expanding: &mut HashSet<V>) -> bool {
        if a == b {
            return true;
        }
        let sets = |xs: &[Ty], ys: &[Ty], expanding: &mut HashSet<V>| {
            xs.iter()
                .all(|x| ys.iter().any(|y| self.same_form_in(x, y, expanding)))
                && ys
                    .iter()
                    .all(|y| xs.iter().any(|x| self.same_form_in(x, y, expanding)))
        };
        match (a, b) {
            (Ty::Var(v), Ty::Var(w))
                if self.reach(vec![*v], true).0.contains(w)
                    && self.reach(vec![*v], false).0.contains(w) =>
            {
                true
            }
            (Ty::Var(v), t) | (t, Ty::Var(v)) => {
                // L <: v <: U proves v = T when both L and U equal T.
                // A hole fixed by a concrete service form is such a case.
                self.observe(*v);
                let st = self.var(*v);
                if (st.lower.is_empty() && st.from.is_empty() && !matches!(t, Ty::Never))
                    || (st.upper.is_empty() && st.into.is_empty() && !matches!(t, Ty::Top))
                {
                    return false;
                }
                if !expanding.insert(*v) {
                    return false;
                }
                let lower = self.reach(vec![*v], true).0;
                let upper = self.reach(vec![*v], false).0;
                let equal = (matches!(t, Ty::Never)
                    || lower.iter().any(|w| {
                        self.var(*w)
                            .lower
                            .iter()
                            .any(|b| !matches!(b, Ty::Var(_)) && self.same_form_in(b, t, expanding))
                    }))
                    && (matches!(t, Ty::Top)
                        || upper.iter().any(|w| {
                            self.var(*w).upper.iter().any(|b| {
                                !matches!(b, Ty::Var(_)) && self.same_form_in(b, t, expanding)
                            })
                        }));
                expanding.remove(v);
                equal
            }
            (Ty::Con(h, values, xs), Ty::Con(k, other, ys)) => {
                h == k
                    && values == other
                    && xs.len() == ys.len()
                    && xs
                        .iter()
                        .zip(ys)
                        .all(|(x, y)| self.same_form_in(x, y, expanding))
            }
            (Ty::Union(xs), Ty::Union(ys)) | (Ty::Inter(xs), Ty::Inter(ys)) => {
                sets(xs, ys, expanding)
            }
            (Ty::Diff(a, hs), Ty::Diff(b, ks)) => {
                self.same_form_in(a, b, expanding) && sets(hs, ks, expanding)
            }
            (Ty::Both(a, b), Ty::Both(c, d)) => {
                self.same_form_in(a, c, expanding) && self.same_form_in(b, d, expanding)
            }
            (Ty::Fn(f), Ty::Fn(g)) => {
                self.same_form_in(&f.param, &g.param, expanding)
                    && self.same_form_in(&f.ret, &g.ret, expanding)
                    && self.same_form_in(&f.err, &g.err, expanding)
                    && self.same_form_in(&f.env, &g.env, expanding)
            }
            _ => false,
        }
    }

    /// A shown scheme must express every handler restriction. A visible
    /// difference or a row containing both the variable and the supplied
    /// form expresses it. A form tied to that variable, as S(N) in a
    /// parameter S(N), also keeps the obligation on that same row.
    pub fn hidden_handlers(&self, t: &Ty, level: u32) -> Vec<Ty> {
        if !self.has_handlers {
            return Vec::new();
        }
        fn parts(t: &Ty, out: &mut Vec<Ty>) {
            out.push(t.clone());
            match t {
                Ty::Con(_, _, xs) | Ty::Union(xs) | Ty::Inter(xs) => {
                    for x in xs {
                        parts(x, out);
                    }
                }
                Ty::Fn(f) => {
                    for x in [&f.param, &f.ret, &f.err, &f.env] {
                        parts(x, out);
                    }
                }
                Ty::Diff(a, hs) => {
                    parts(a, out);
                    for h in hs {
                        parts(h, out);
                    }
                }
                Ty::Both(a, b) => {
                    parts(a, out);
                    parts(b, out);
                }
                _ => {}
            }
        }
        let mut all = Vec::new();
        parts(t, &mut all);
        let mut hidden = Vec::new();
        for v in all
            .iter()
            .filter_map(|t| if let Ty::Var(v) = t { Some(*v) } else { None })
        {
            if self.var(v).level <= level || self.var(v).row == Mode::Value {
                continue;
            }
            let above = self.reach(vec![v], false).0;
            let below = self.reach(vec![v], true).0;
            let aliases: Vec<_> = above
                .iter()
                .filter(|w| below.contains(w))
                .copied()
                .collect();
            let is_var = |t: &Ty| matches!(t, Ty::Var(w) if aliases.contains(w));
            for w in &above {
                for h in &self.var(*w).handled {
                    // A non-scalar constructor with no parameters has only
                    // one form, so this obligation holds for every row.
                    if matches!(h, Ty::Con(_, values, args) if values.is_all() && args.is_empty())
                        && h.scalar().is_none()
                    {
                        continue;
                    }
                    let witnessed = all.iter().any(|part| match part {
                        Ty::Diff(a, hs) => is_var(a) && hs.iter().any(|m| self.same_form(m, h)),
                        Ty::Union(ms) => {
                            ms.iter().any(&is_var) && ms.iter().any(|m| self.same_form(m, h))
                        }
                        part if self.same_form(part, h) => {
                            let mut edges = Vec::new();
                            bound_vars(self, h, true, false, &mut edges);
                            edges.iter().any(|e| aliases.contains(&e.0))
                        }
                        _ => false,
                    });
                    if !witnessed && !hidden.contains(h) {
                        hidden.push(h.clone());
                    }
                }
            }
        }
        hidden
    }

    /// `l <: r`, or `l` in `r` when `env` (§4.5 rule 4). The mode goes
    /// through the bounds of variables, but not into type arguments: those
    /// are compared as normal types.
    fn constrain_in(&mut self, l: &Ty, r: &Ty, env: Mode) {
        // A row also used as an ordinary value must retain that value's
        // checked form. Keep both inclusions: fitting the need alone would
        // lose the shared value constraint (§4.7, §4.5 rule 4, §9.2).
        if env == Mode::Value && l.is_headed() && self.mode(r) == Mode::Env {
            self.constrain_in(r, l, Mode::Env);
            self.constrain_in(l, r, Mode::Env);
            return;
        }
        if self.rigid_boundary > 0 && self.rigid_constraint(l, r, env) {
            return;
        }
        let env = env.max(self.mode(l)).max(self.mode(r));
        if env != Mode::Value {
            self.mark_mode(l, env);
            self.mark_mode(r, env);
        }

        if l == r || matches!(l, Ty::Never) || matches!(r, Ty::Top) {
            return;
        }
        if matches!(l, Ty::Inter(ms) if ms.contains(r))
            || matches!(r, Ty::Union(ms) if ms.contains(l))
        {
            return;
        }
        if self.observed_shape
            && (matches!(r, Ty::Union(ts) if ts.contains(l))
                || matches!(l,Ty::Inter(ts) if ts.contains(r)))
        {
            return;
        }
        if l != r {
            self.observe_type(l);
            self.observe_type(r);
        }
        if l == r || matches!(l, Ty::Never) || matches!(r, Ty::Top) {
            return;
        }
        // §4.5 rule 3: subtracting members never adds values. The
        // difference's separate same-form obligation remains in the scheme.
        if let Ty::Diff(a, _) = l
            && (a.as_ref() == r
                || matches!((a.as_ref(), r), (Ty::Var(u), Ty::Var(v))
                if self.reach(vec![*u], false).0.contains(v)))
        {
            return;
        }
        // §4.5 rule 2: these are inclusions, not recursive bounds such as
        // `n <: n | S(n)`. Recording them would invent an occurs cycle.
        if matches!(l, Ty::Var(_)) && matches!(r, Ty::Union(ms) if ms.contains(l)) {
            return;
        }
        // A row is also contained in H | (row - H). Recording this
        // tautology as an upper bound makes generalization split its two
        // occurrences into unrelated variables, closing an open callback row.
        // The difference already retains H's same-form obligations.
        if let (Ty::Var(_), Ty::Union(ms)) = (l, r) {
            let mut flat = Vec::new();
            flatten_union(ms, &mut flat);
            if flat.iter().any(|m| {
                matches!(m, Ty::Diff(base, heads) if base.as_ref() == l
                    && heads.iter().all(|h| flat.iter().any(|m| self.same_form(m, h))))
            }) {
                return;
            }
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
            (Ty::Diff(a, h), _) if env == Mode::Env && h.iter().all(|m| self.directed(m)) => {
                // Route the handled entries before storing a row bound.
                // Keeping `M - S(N)` as a bound of N would retain S(N)
                // inside N, although rule B has already handled its key.
                self.diff_left(a, h, r, env);
            }
            (Ty::Var(_), Ty::Var(v)) if env != Mode::Value => {
                // Collect requirements in the receiving row. An upper
                // bound on its member can lead back through a service's
                // invariant argument: n <: row, S(n) <: row is finite.
                // Do this across levels too, after copying the member to
                // the receiver's level; do not make its upper bound the
                // whole receiving row.
                let level = self.var(*v).level;
                if self.level_of(l) > level {
                    let l2 = self.extrude(l, Pol::Pos, level, env, &mut HashMap::default());
                    self.constrain_in(&l2, r, env);
                    return;
                }
                self.add_bound(*v, l.clone(), true, env);
                for ub in self.var(*v).upper.clone() {
                    self.constrain_in(l, &ub, env);
                }
            }
            (Ty::Var(v), _) if self.level_of(r) <= self.var(*v).level => {
                if self.add_bound(*v, r.clone(), false, env) {
                    for lb in self.var(*v).lower.clone() {
                        self.constrain_in(&lb, r, env);
                    }
                }
            }
            (_, Ty::Var(v)) if self.level_of(l) <= self.var(*v).level => {
                if self.add_bound(*v, l.clone(), true, env) {
                    for ub in self.var(*v).upper.clone() {
                        self.constrain_in(l, &ub, env);
                    }
                }
            }
            (Ty::Var(v), _) => {
                let level = self.var(*v).level;
                let r2 = self.extrude(r, Pol::Neg, level, env, &mut HashMap::default());
                self.constrain_in(l, &r2, env);
            }
            (_, Ty::Var(v)) => {
                let level = self.var(*v).level;
                let l2 = self.extrude(l, Pol::Pos, level, env, &mut HashMap::default());
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
                let (conc, vars): (Vec<Ty>, Vec<Ty>) = ms.iter().cloned().partition(Ty::is_headed);
                let mut tries = Vec::new();
                if !conc.is_empty() {
                    tries.push(self.normalize_mode(Ty::Inter(conc), env));
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
                if h.iter()
                    .any(|m| m.value_head() == l.value_head() && !m.values().disjoint(l.values()))
                {
                    let partial = !l.values().is_all()
                        || h.iter()
                            .any(|m| m.head() == l.head() && !m.values().is_all());
                    self.fail(
                        if partial && env == Mode::Value {
                            FailKind::Values
                        } else {
                            FailKind::Mismatch
                        },
                        l,
                        r,
                    );
                } else {
                    self.constrain_in(l, a, env);
                }
            }
            (Ty::Con(h1, s1, a1), Ty::Con(h2, s2, a2)) if h1 == h2 && a1.len() == a2.len() => {
                if (env == Mode::Env && s1 != s2) || (env != Mode::Env && !s1.subset(s2)) {
                    self.fail(
                        if env != Mode::Value {
                            FailKind::EnvForms
                        } else {
                            FailKind::Values
                        },
                        l,
                        r,
                    );
                    return;
                }
                if env == Mode::Env {
                    // Rule A: the need `l` fits in `r` when the form of `r`
                    // is a subtype of the form of `l` as a value: argument
                    // constraints go the other way round from values.
                    let before = self.failures.len();
                    let need = self.known_need_form(l);
                    let provider = self.known_need_form(r);
                    let vs = self.variances(*h1, a1.len());
                    for (((x, y), v), row) in
                        a1.iter().zip(a2).zip(vs).zip(self.rows(*h1, a1.len()))
                    {
                        self.constrain_by(y, x, v, row);
                    }
                    if self.failures.len() > before {
                        self.failures.truncate(before);
                        self.fail(FailKind::EnvForms, &need, &provider);
                    }
                    return;
                }
                if env != Mode::Value {
                    // Error rows: arguments are equal.
                    for (x, y) in a1.iter().zip(a2) {
                        self.equal(x, y, FailKind::EnvForms);
                    }
                    return;
                }
                let vs = self.variances(*h1, a1.len());
                for (((x, y), v), row) in a1.iter().zip(a2).zip(vs).zip(self.rows(*h1, a1.len())) {
                    self.constrain_by(x, y, v, row);
                }
            }
            (Ty::Fn(f), Ty::Fn(g)) => {
                // syntax-3 §6.1: one data parameter; a call of another
                // shape is a mismatch of the parameter data.
                self.constrain(&g.param, &f.param);
                self.constrain(&f.ret, &g.ret);
                self.constrain_row(&f.err, &g.err);
                self.constrain_env(&f.env, &g.env);
            }
            _ => self.fail(FailKind::Mismatch, l, r),
        }
    }

    fn constrain_by(&mut self, x: &Ty, y: &Ty, v: Variance, row: Mode) {
        match v {
            Variance::Co => self.constrain_in(x, y, row),
            Variance::Contra => self.constrain_in(y, x, row),
            Variance::Inv => {
                self.constrain_in(x, y, row);
                self.constrain_in(y, x, row);
            }
            Variance::Bi => {}
        }
    }

    /// Adds a bound to a variable. Two bounds with the same type
    /// constructor are one member (§4.4): two upper bounds by intersection,
    /// two lower bounds by union. Arguments that do not change must be
    /// equal, needs merge by the value intersection (§9.2), and function
    /// types must have the same number of parameters. This holds
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
    fn add_bound(&mut self, v: V, t: Ty, lower: bool, env: Mode) -> bool {
        assert!(
            (v.0 as usize) >= self.rigid_boundary,
            "typed expansion mutated a frozen variable"
        );
        self.observe(v);
        // Only argumentless Con bounds can fuse without touching variables,
        // variance, or service argument comparisons.
        let fusion = if let Ty::Con(h, values, args) = &t {
            let state = self.var(v);
            let bounds = if lower { &state.lower } else { &state.upper };
            if args.is_empty() {
                if let Some((i, old)) = bounds
                    .iter()
                    .enumerate()
                    .find(|(_, b)| matches!(b, Ty::Con(g, _, xs) if g == h && xs.is_empty()))
                {
                    let old_values = old.values();
                    if env == Mode::Value
                        && if lower {
                            values.subset(old_values)
                        } else {
                            old_values.subset(values)
                        }
                    {
                        return false;
                    }
                    if env != Mode::Value {
                        None
                    } else if lower {
                        Some((i, old_values.union(values)))
                    } else {
                        // Never is skipped by reach(), so preserve both bounds
                        // when their intersection is empty.
                        old_values.meet(values).map(|set| (i, set))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        self.touched.push(v);
        let new = match &t {
            Ty::Var(u) => self.reach(vec![*u], lower).1,
            bound if bound.is_headed() => vec![bound.clone()],
            _ => Vec::new(),
        };
        // No constructor pairs need checking when the new bound has none.
        if !new.is_empty() {
            let level = self.var(v).level;
            // The variables whose bounds now have `t` too, and their bounds.
            let preds = self.reach(vec![v], !lower).0;
            let marked: Vec<_> = if lower || !self.has_need_meets {
                Vec::new()
            } else {
                preds
                    .iter()
                    .filter_map(|w| self.var(*w).need_meet.clone().map(|forms| (*w, forms)))
                    .collect()
            };
            let mut others = self.reach(preds, lower).1;
            for (meet, (left, right)) in marked {
                let bounds = self.reach(vec![meet], false).1;
                if new.iter().any(|a| {
                    bounds.iter().any(|b| {
                        a.head() != b.head()
                            || (a.scalar().is_some() && a.values().disjoint(b.values()))
                    })
                }) {
                    self.var_mut(meet).need_meet = None;
                    self.fail(FailKind::NeedMeet, &left, &right);
                }
            }
            if !lower {
                let mut heads = HashSet::default();
                others.retain(|b| heads.insert(b.head()));
            }
            for a in &new {
                for b in &others {
                    self.compare(a, b, lower, env, level);
                }
            }
        }
        if env != Mode::Value
            && lower
            && self.has_handlers
            && let Ty::Var(u) = &t
        {
            let handlers: Vec<_> = self
                .reach(vec![v], false)
                .0
                .into_iter()
                .flat_map(|w| self.var(w).handled.clone())
                .collect();
            for h in handlers {
                self.handle(*u, &h);
            }
        }
        if env != Mode::Value && !new.is_empty() && self.has_handlers {
            let mut handlers = Vec::new();
            for w in self.reach(vec![v], false).0 {
                handlers.extend(self.var(w).handled.clone());
            }
            for a in &new {
                for h in &handlers {
                    if a.head() != h.head() {
                        continue;
                    }
                    if env == Mode::Env {
                        // Rule B, as in `handle`.
                        if lower {
                            self.constrain_in(a, h, Mode::Env);
                        }
                    } else {
                        self.equal(a, h, FailKind::EnvForms);
                    }
                }
            }
        }
        let s = self.var_mut(v);
        let bounds = if lower { &mut s.lower } else { &mut s.upper };
        if let Some((i, values)) = fusion {
            let Ty::Con(h, _, args) = &t else {
                unreachable!()
            };
            bounds[i] = Ty::Con(*h, values, args.clone());
            return true;
        }
        if !bounds.contains(&t) {
            bounds.push(t.clone());
            #[cfg(test)]
            if t.head().is_some() {
                self.observe_bounds(v, lower);
            }
            if let Ty::Var(w) = &t
                && (w.0 as usize) >= self.rigid_boundary
            {
                if lower {
                    self.var_mut(*w).into.push(v);
                } else {
                    self.var_mut(*w).from.push(v);
                }
            }
        }
        true
    }

    #[cfg(test)]
    fn observe_bounds(&mut self, v: V, lower: bool) {
        let bounds = if lower {
            &self.var(v).lower
        } else {
            &self.var(v).upper
        };
        let mut counts = HashMap::default();
        let mut peak = 0;
        for h in bounds.iter().filter_map(Ty::head) {
            let n = counts.entry(h).or_insert(0);
            *n += 1;
            peak = peak.max(*n);
        }
        self.peak_bound_count = self.peak_bound_count.max(peak);
    }

    /// The variables reached from `starts` through bounds that are
    /// variables, upward (`x <: u`) or downward (`w <: x`) as `lower` says,
    /// and the bounds of those variables that are not variables.
    fn reach(&self, starts: Vec<V>, lower: bool) -> (Vec<V>, Vec<Ty>) {
        let mut seen: Vec<V> = Vec::new();
        let mut visited = self.reach_marks.start(self.vars.len());
        // Graph keys hash their immutable pool identity and node ID.
        #[allow(clippy::mutable_key_type)]
        let mut found: HashSet<&Ty> = HashSet::default();
        let mut stack = starts;
        let mut out = Vec::new();
        while let Some(x) = stack.pop() {
            if !visited.insert(x) {
                continue;
            }
            seen.push(x);
            self.observe(x);
            let st = self.var(x);
            let (bounds, back) = if lower {
                (&st.lower, &st.from)
            } else {
                (&st.upper, &st.into)
            };
            for b in bounds {
                match b {
                    Ty::Var(u) => stack.push(*u),
                    b if b.is_headed() && found.insert(b) => out.push(b.clone()),
                    _ => {}
                }
            }
            stack.extend(back.iter().copied());
        }
        (seen, out)
    }

    /// The checks of `add_bound` for two bounds on one side of a variable.
    fn compare(&mut self, a: &Ty, b: &Ty, lower: bool, env: Mode, level: u32) {
        if a == b || a.head() != b.head() {
            return;
        }
        // Joins are made once for each pair; the other checks give their
        // problems at each place.
        let first = !self.compared.contains(&(b.clone(), a.clone(), lower, env))
            && self.compared.insert((a.clone(), b.clone(), lower, env));
        let is_lower_bound = lower;
        let lower = lower && first;
        match (a, b) {
            (Ty::Fn(f), Ty::Fn(g)) => {
                if lower {
                    self.join(&f.ret, &g.ret, level, Mode::Value);
                    self.join(&f.err, &g.err, level, Mode::Row);
                    self.join(&f.env, &g.env, level, Mode::Env);
                }
            }
            (Ty::Con(h, _, xs), Ty::Con(_, _, ys)) if xs.len() == ys.len() => {
                let vs = self.variances(*h, xs.len());
                for (((x, y), var), row) in xs.iter().zip(ys).zip(vs).zip(self.rows(*h, xs.len())) {
                    if env == Mode::Env {
                        // Two needs of one row are one entry: by value
                        // meet for lower bounds, by value join for upper
                        // ones (rule A). Only invariant arguments can
                        // disagree; the rest is checked through the
                        // bounds that the entries reach.
                        // The meet of two same-direction arguments (§4.4) is
                        // an error when it is `Never` and neither is. With
                        // an argument not known yet, `a & T` stays as it is
                        // and is checked when the binding is done.
                        if var == Variance::Inv {
                            self.equal(x, y, FailKind::EnvForms);
                        } else if var == Variance::Co
                            && is_lower_bound
                            && !matches!(x, Ty::Never)
                            && !matches!(y, Ty::Never)
                            && first
                        {
                            self.need_meet(x, y, a, b, row, level);
                        }
                    } else if env != Mode::Value {
                        self.equal(x, y, FailKind::EnvForms);
                    } else if var == Variance::Inv {
                        self.equal(x, y, FailKind::Invariant);
                    } else if var == Variance::Co && lower {
                        self.join(x, y, level, row);
                    }
                }
            }
            _ => {}
        }
    }

    /// A service form whose parameters all have a direction.
    fn directed(&self, t: &Ty) -> bool {
        matches!(t, Ty::Con(h, _, args) if !args.is_empty()
            && self.variances(*h, args.len()).all(|x| x != Variance::Inv))
    }

    /// Keep an undetermined intersection as two upper bounds (§4.4).
    /// A later concrete bound is checked by `add_bound`, also through links.
    fn need_meet(&mut self, x: &Ty, y: &Ty, a: &Ty, b: &Ty, row: Mode, level: u32) -> Ty {
        // Idempotence and absorption apply to row aliases and inclusions
        // too; introducing a derived variable can invent a structural cycle.
        if self.same_form(x, y) {
            return x.clone();
        }
        if row != Mode::Value
            && let (Ty::Var(u), Ty::Var(v)) = (x, y)
        {
            if self.reach(vec![*u], false).0.contains(v) {
                return x.clone();
            }
            if self.reach(vec![*v], false).0.contains(u) {
                return y.clone();
            }
        }
        let meet = self.normalize_mode(Ty::Inter(vec![x.clone(), y.clone()]), row);
        if matches!(meet, Ty::Never) {
            self.fail(FailKind::NeedMeet, a, b);
        }
        if !matches!(meet, Ty::Inter(_)) {
            return meet;
        }
        let v = self.fresh_at(level);
        self.has_need_meets = true;
        self.var_mut(v).need_meet = Some((a.clone(), b.clone()));
        self.constrain_in(&Ty::Var(v), x, row);
        self.constrain_in(&Ty::Var(v), y, row);
        // Row inclusion stores variable edges at the receiver. A derived
        // meet must also retain its actual operands as upper bounds: later
        // lower bounds must reach both, and coalescing must not confuse
        // the meet's other receiving rows with its operands (§4.4, §9.2).
        for operand in [x, y] {
            if matches!(operand, Ty::Var(_)) && !self.var(v).upper.contains(operand) {
                self.var_mut(v).upper.push(operand.clone());
            }
        }
        Ty::Var(v)
    }

    /// A value-produced argument is known at completion from its lower
    /// bounds. Inputs and still-open sources remain undetermined (§4.7).
    fn known_need_members(&self, t: &Ty) -> Option<Vec<Ty>> {
        match t {
            Ty::Never => None,
            Ty::Var(v) => {
                let (vars, members) = self.reach(vec![*v], true);
                if members.is_empty()
                    || vars.iter().any(|v| {
                        let state = self.var(*v);
                        state.need_meet.is_none()
                            && (state.input || (state.lower.is_empty() && state.from.is_empty()))
                    })
                {
                    None
                } else {
                    Some(members)
                }
            }
            Ty::Union(ms) => {
                let mut out = Vec::new();
                for m in ms {
                    out.extend(self.known_need_members(m)?);
                }
                Some(out)
            }
            t if t.is_headed() => Some(vec![t.clone()]),
            _ => None,
        }
    }

    fn known_need_form(&self, t: &Ty) -> Ty {
        let Ty::Con(h, values, args) = t else {
            return t.clone();
        };
        let args = args
            .iter()
            .map(|arg| match self.known_need_members(arg) {
                Some(mut ms) if ms.len() == 1 => ms.pop().unwrap(),
                _ => arg.clone(),
            })
            .collect();
        Ty::Con(*h, values.clone(), args)
    }

    /// Check deferred meets whose arguments became closed values during
    /// this binding, as well as the concrete upper bounds checked on insertion.
    pub fn check_need_meets(&mut self) {
        if !self.has_need_meets {
            return;
        }
        let meets: Vec<_> = self
            .vars
            .iter()
            .enumerate()
            .filter_map(|(i, state)| state.need_meet.clone().map(|forms| (V(i as u32), forms)))
            .collect();
        for (v, (a, b)) in meets {
            let parts: Vec<_> = self
                .var(v)
                .upper
                .iter()
                .filter_map(|t| self.known_need_members(t))
                .collect();
            let empty = parts.iter().enumerate().any(|(i, xs)| {
                parts[..i].iter().any(|ys| {
                    !xs.iter().any(|x| {
                        ys.iter().any(|y| {
                            x.head() == y.head()
                                && (x.scalar().is_none() || !x.values().disjoint(y.values()))
                        })
                    })
                })
            });
            if empty {
                let a = self.known_need_form(&a);
                let b = self.known_need_form(&b);
                // An invalid binding is reported once; no successful scheme
                // can carry this failed obligation into another binding.
                self.var_mut(v).need_meet = None;
                self.fail(FailKind::NeedMeet, &a, &b);
            }
        }
    }

    /// Puts two types together by union in a new variable, only for the
    /// checks of `add_bound`: the variable is used nowhere else.
    fn join(&mut self, x: &Ty, y: &Ty, level: u32, env: Mode) {
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
    fn route(&mut self, l: &Ty, ms: &[Ty], env: Mode) {
        let mut flat = Vec::new();
        flatten_union(ms, &mut flat);
        let h = l.value_head();
        let mut rest = None;
        if h.is_some()
            && let Some(m) = flat.iter().find(|m| m.value_head() == h)
        {
            if l.scalar().is_some() && env != Mode::Env {
                // A raw union need not have passed union_in. Join all scalar
                // members of this head before splitting its uncovered part.
                let sets: Vec<_> = flat
                    .iter()
                    .filter(|m| m.head() == h)
                    .map(|m| m.values().clone())
                    .collect();
                let covered = super::lits::ValueSet::join_all(&sets);
                if l.values().subset(&covered) {
                    return;
                }
                rest = Some(l.with_values(l.values().minus(&covered)));
            } else {
                let m = m.clone();
                self.constrain_in(l, &m, env);
                return;
            }
        }
        let vars: Vec<&Ty> = flat
            .iter()
            .filter(|m| (m.is_varlike() && m.value_head().is_none()) || matches!(m, Ty::Inter(_)))
            .collect();
        match vars.len() {
            1 => {
                let m = vars[0].clone();
                self.constrain_in(rest.as_ref().unwrap_or(l), &m, env);
            }
            0 => self.fail(
                if rest.is_some() {
                    FailKind::Values
                } else {
                    FailKind::Mismatch
                },
                l,
                &Ty::Union(ms.to_vec()),
            ),
            _ => self.fail(FailKind::Ambiguous, l, &Ty::Union(ms.to_vec())),
        }
    }

    /// `A - H <: R`, where `R` is not a variable. For a variable `a`, its
    /// values whose type constructor is in `H` go to `H`, and the rest go
    /// to `R`: `a <: H | R`, with the members of `H` first.
    fn diff_left(&mut self, a: &Ty, h: &[Ty], r: &Ty, env: Mode) {
        match a {
            Ty::Never => {}
            Ty::Union(ms) => {
                for m in ms {
                    self.diff_left(m, h, r, env);
                }
            }
            Ty::Con(..) if a.scalar().is_some() => {
                let sets: Vec<_> = h
                    .iter()
                    .filter(|m| m.head() == a.head())
                    .map(|m| m.values().clone())
                    .collect();
                if sets.is_empty() {
                    self.constrain_in(a, r, env);
                } else {
                    let excluded = super::lits::ValueSet::join_all(&sets);
                    if !a.values().subset(&excluded) {
                        let rest = a.with_values(a.values().minus(&excluded));
                        self.constrain_in(&rest, r, env);
                    }
                }
            }
            Ty::Con(..) | Ty::Fn(_) => match h.iter().find(|m| m.head() == a.head()) {
                Some(m) => {
                    let m = m.clone();
                    self.constrain_in(a, &m, env);
                }
                None => self.constrain_in(a, r, env),
            },
            Ty::Var(v)
                if env == Mode::Env
                    && h.iter().all(|m| self.directed(m))
                    && !self
                        .reach(vec![*v], true)
                        .1
                        .iter()
                        .any(|b| h.iter().any(|m| m.head() == b.head())) =>
            {
                // Rule B: the needs of `a` with a constructor of `H` go to
                // `H` (routing and `handle`); the rest of `a` is in `R`. Not
                // putting `H` into the bound of `a` keeps a handler that
                // mentions `a`'s own row (`! S(N) = …` for a need
                // `(..M, S(M))`) from making a type that has itself inside.
                self.constrain_in(a, r, env);
            }
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
    fn extrude(
        &mut self,
        t: &Ty,
        pol: Pol,
        level: u32,
        env: Mode,
        cache: &mut HashMap<(V, Pol, Mode), V>,
    ) -> Ty {
        let env = env.max(self.mode(t));
        if self.frozen(t) {
            return t.clone();
        }
        self.observe_type(t);
        if self.level_of(t) <= level {
            return t.clone();
        }
        match t {
            Ty::Var(v) => {
                if let Some(nv) = cache.get(&(*v, pol, env)) {
                    return Ty::Var(*nv);
                }
                let nv = self.fresh_at(level);
                self.vars[nv.0 as usize].input = self.var(*v).input;
                self.vars[nv.0 as usize].row = self.var(*v).row;
                self.vars[nv.0 as usize].difference = self.var(*v).difference;
                cache.insert((*v, pol, env), nv);
                let handled = self.var(*v).handled.clone();
                let handled = handled
                    .iter()
                    .map(|h| self.extrude(h, Pol::Both, level, Mode::Value, cache))
                    .collect();
                self.var_mut(nv).handled = handled;
                if let Some((a, b)) = self.var(*v).need_meet.clone() {
                    let a = self.extrude(&a, Pol::Both, level, Mode::Value, cache);
                    let b = self.extrude(&b, Pol::Both, level, Mode::Value, cache);
                    self.var_mut(nv).need_meet = Some((a, b));
                }
                if pol != Pol::Neg {
                    self.var_mut(*v).upper.push(Ty::Var(nv));
                    self.var_mut(nv).from.push(*v);
                    let lower = self.var(*v).lower.clone();
                    let lower = lower
                        .iter()
                        .map(|b| self.extrude(b, Pol::Pos, level, env, cache))
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
                        .map(|b| self.extrude(b, Pol::Neg, level, env, cache))
                        .collect();
                    self.set_upper(nv, upper);
                }
                Ty::Var(nv)
            }
            Ty::Con(h, values, args) => {
                // Keep both bounds across levels for a need comparison;
                // inside each argument, its definition supplies the mode.
                Ty::Con(
                    *h,
                    values.clone(),
                    args.iter()
                        .zip(self.variances(*h, args.len()))
                        .zip(self.rows(*h, args.len()))
                        .map(|((a, variance), row)| {
                            self.extrude(
                                a,
                                if env != Mode::Value {
                                    Pol::Both
                                } else {
                                    pol.by(variance)
                                },
                                level,
                                row,
                                cache,
                            )
                        })
                        .collect(),
                )
            }
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                param: self.extrude(&f.param, pol.flip(), level, Mode::Value, cache),
                labels: f.labels.clone(),
                ret: self.extrude(&f.ret, pol, level, Mode::Value, cache),
                err: self.extrude(&f.err, pol, level, Mode::Row, cache),
                env: self.extrude(&f.env, pol, level, Mode::Env, cache),
            })),
            Ty::Union(ms) if env != Mode::Value || self.is_row(t) => {
                let mode = env.max(self.mode(t));
                let ms = ms
                    .iter()
                    .map(|m| self.extrude(m, pol, level, mode, cache))
                    .collect();
                self.union_mode(ms, mode)
            }
            _ => self.map_children(t, pol, &mut |s, x, p| s.extrude(x, p, level, env, cache)),
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
                self.var_mut(nv).row = self.var(*v).row;
                self.var_mut(nv).difference = self.var(*v).difference;
                let handled = self.var(*v).handled.clone();
                let handled = handled
                    .iter()
                    .map(|h| self.freshen(h, above, map))
                    .collect();
                self.var_mut(nv).handled = handled;
                if let Some((a, b)) = self.var(*v).need_meet.clone() {
                    let a = self.freshen(&a, above, map);
                    let b = self.freshen(&b, above, map);
                    self.var_mut(nv).need_meet = Some((a, b));
                }
                let (lower, upper) = (self.var(*v).lower.clone(), self.var(*v).upper.clone());
                let lower = lower.iter().map(|b| self.freshen(b, above, map)).collect();
                let upper = upper.iter().map(|b| self.freshen(b, above, map)).collect();
                self.set_lower(nv, lower);
                self.set_upper(nv, upper);
                Ty::Var(nv)
            }
            Ty::Union(ms) if self.is_row(t) => {
                // Rebuild the row's collector for these fresh variables.
                // Otherwise generalization can leave its only receiver
                // behind, losing comparisons of forms instantiated later.
                let mode = self.mode(t);
                let ms = ms.iter().map(|m| self.freshen(m, above, map)).collect();
                self.union_mode(ms, mode)
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
            Ty::Con(h, values, args) => {
                let vs = self.variances(*h, args.len());
                let args = args
                    .iter()
                    .zip(vs)
                    .map(|(a, v)| f(self, a, pol.by(v)))
                    .collect();
                Ty::Con(*h, values.clone(), args)
            }
            Ty::Fn(fun) => Ty::Fn(Box::new(FnTy {
                param: f(self, &fun.param, pol.flip()),
                labels: fun.labels.clone(),
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
        self.union_mode(ms, Mode::Value)
    }

    /// Merge needs by their value intersection (§9.2, §4.4). The row is
    /// represented internally by a union of its keyed service entries.
    pub fn env_union(&mut self, ms: Vec<Ty>) -> Ty {
        self.union_mode(ms, Mode::Env)
    }

    pub fn union_mode(&mut self, ms: Vec<Ty>, mode: Mode) -> Ty {
        let mode = mode.max(ms.iter().map(|m| self.mode(m)).max().unwrap_or_default());
        let ms = if mode == Mode::Row {
            join_literals(ms)
        } else {
            ms
        };
        let before = self.failures.len();
        let t = self.union_in(ms, mode);
        if mode != Mode::Value
            && let Ty::Union(ms) = &t
            && self.failures.len() == before
            && ms.iter().any(Ty::is_varlike)
            && (ms.iter().filter(|m| m.is_varlike()).take(2).count() > 1
                || ms
                    .iter()
                    .any(|m| matches!(m, Ty::Con(_, _, args) if !args.is_empty())))
        {
            // Check forms hidden behind a row variable too (§9.2). The
            // collector adds no requirement to the returned union; it
            // makes add_bound compare all its services, also later ones.
            // With only one unknown row and services without arguments,
            // no unequal pair can be introduced by this union.
            // At the level of the members: a collector below them would
            // copy their variables to its level and link every later
            // instance of a scheme to that shared copy.
            let level = ms
                .iter()
                .map(|m| self.level_of(m))
                .max()
                .unwrap_or(self.level);
            let row = Ty::Var(self.fresh_at(level));
            for m in ms {
                self.constrain_in(m, &row, mode);
            }
        }
        t
    }

    fn union_in(&mut self, ms: Vec<Ty>, mode: Mode) -> Ty {
        let env = mode != Mode::Value;
        let mut flat = Vec::new();
        flatten_union(&ms, &mut flat);
        if !env {
            flat = join_literals(flat);
        }
        let mut out: Vec<Ty> = Vec::new();
        for m in flat {
            if mode == Mode::Env
                && let Ty::Diff(a, hs) = &m
            {
                // A cached handler installation is not a diagnostic check of
                // this use. Validate its current needs when forming the row.
                self.diff_bound_mode(a, hs, mode);
            }
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
                    if out[i].values() != m.values() {
                        out.push(m);
                        continue;
                    }
                    // Two forms of a service: their arguments must be equal
                    // (§9.2). When they are not, both stay, as they are.
                    let before = self.failures.len();
                    let old = std::mem::replace(&mut out[i], Ty::Never);
                    out[i] = self.merge(old, m.clone(), true, mode);
                    if self.failures.len() > before {
                        out.push(m);
                    }
                }
                Some(i) => {
                    let old = std::mem::replace(&mut out[i], Ty::Never);
                    out[i] = self.merge(old, m, true, mode);
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
        self.normalize_mode(Ty::Inter(ms), Mode::Value)
    }

    fn inter_in(&mut self, ms: Vec<Ty>, mode: Mode) -> Ty {
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
                    self.inter_in(a, mode)
                })
                .collect();
            return self.union_in(parts, mode);
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
                        Some(c) if c.head() == m.head() => self.merge(c, m, false, mode),
                        Some(_) => return Ty::Never,
                    });
                    if matches!(con, Some(Ty::Never)) {
                        return Ty::Never;
                    }
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
    /// and by intersection when not. A union of need entries instead takes
    /// the value intersection of the services (§9.2).
    fn merge(&mut self, a: Ty, b: Ty, union: bool, mode: Mode) -> Ty {
        match (a, b) {
            (Ty::Con(h, values, xs), Ty::Con(_, other, ys))
                if xs.len() == ys.len() && mode == Mode::Env =>
            {
                // Rule A: two forms of a service in a row are one entry,
                // the value meet for a union of needs and the value join
                // for an intersection.
                if values != other {
                    self.fail(
                        FailKind::EnvForms,
                        &Ty::Con(h, values.clone(), xs.clone()),
                        &Ty::Con(h, other, ys.clone()),
                    );
                    return Ty::Con(h, values, xs);
                }
                let vs = self.variances(h, xs.len());
                let rows = self.rows(h, xs.len());
                let mut args = Vec::with_capacity(xs.len());
                for (((x, y), v), row) in xs.iter().zip(&ys).zip(vs).zip(rows) {
                    let empty_before = matches!(x, Ty::Never) || matches!(y, Ty::Never);
                    let arg = if union && v == Variance::Co && !empty_before {
                        let level = self.level_of(x).max(self.level_of(y));
                        self.need_meet(
                            x,
                            y,
                            &Ty::Con(h, values.clone(), xs.clone()),
                            &Ty::Con(h, values.clone(), ys.clone()),
                            row,
                            level,
                        )
                    } else {
                        self.merge_arg(x.clone(), y.clone(), v, !union, row)
                    };
                    args.push(arg);
                }
                Ty::Con(h, values, args)
            }
            (Ty::Con(h, values, xs), Ty::Con(_, other, ys)) if xs.len() == ys.len() => {
                if mode != Mode::Value {
                    if values != other {
                        self.fail(
                            FailKind::EnvForms,
                            &Ty::Con(h, values.clone(), xs.clone()),
                            &Ty::Con(h, other, ys.clone()),
                        );
                    }
                    for (x, y) in xs.iter().zip(&ys) {
                        self.equal(x, y, FailKind::EnvForms);
                    }
                    return Ty::Con(h, values, xs);
                }
                let values = if union {
                    values.union(&other)
                } else {
                    let Some(values) = values.meet(&other) else {
                        return Ty::Never;
                    };
                    values
                };
                let vs = self.variances(h, xs.len());
                let rows = self.rows(h, xs.len());
                let args = xs
                    .into_iter()
                    .zip(ys)
                    .zip(vs)
                    .zip(rows)
                    .map(|(((x, y), v), row)| self.merge_arg(x, y, v, union, row))
                    .collect();
                Ty::Con(h, values, args)
            }
            (Ty::Fn(f), Ty::Fn(g)) => {
                let f = *f;
                let g = *g;
                // Functions of different shapes put together by union have
                // the parameter `Never` (syntax-3 §6.1).
                let labels = f
                    .labels
                    .iter()
                    .zip(&g.labels)
                    .map(|(l1, l2)| if l1 == l2 { l1.clone() } else { None })
                    .collect();
                let param = self.merge_arg(f.param, g.param, Variance::Contra, union, Mode::Value);
                // The environments of two function types are put together
                // as environments (§4.4, §9.2).
                let env = if union {
                    self.union_mode(vec![f.env, g.env], Mode::Env)
                } else {
                    self.normalize_mode(Ty::Inter(vec![f.env, g.env]), Mode::Env)
                };
                Ty::Fn(Box::new(FnTy {
                    param,
                    labels,
                    ret: self.merge_arg(f.ret, g.ret, Variance::Co, union, Mode::Value),
                    err: self.merge_arg(f.err, g.err, Variance::Co, union, Mode::Row),
                    env,
                }))
            }
            (a, b) => unreachable!("members of one head put together: {a:?} and {b:?}"),
        }
    }

    fn merge_arg(&mut self, x: Ty, y: Ty, v: Variance, union: bool, row: Mode) -> Ty {
        match v {
            Variance::Co | Variance::Bi if union => self.union_mode(vec![x, y], row),
            Variance::Co | Variance::Bi => self.normalize_mode(Ty::Inter(vec![x, y]), row),
            Variance::Contra if union => self.normalize_mode(Ty::Inter(vec![x, y]), row),
            Variance::Contra => self.union_mode(vec![x, y], row),
            Variance::Inv => {
                self.equal(&x, &y, FailKind::Invariant);
                x
            }
        }
    }

    /// `a - h1 - h2 - …` (§4.4). A member of `a` whose type constructor is
    /// in `h` is taken out, and it must be a subtype of that member of `h`.
    pub fn diff(&mut self, a: Ty, h: Vec<Ty>) -> Ty {
        self.diff_mode(a, h, Mode::Value)
    }

    /// Remove the services provided by `T` from the need `M`; the provider
    /// must satisfy each removed entry as a value (§9.2, §4.5 rule 4).
    pub fn env_diff(&mut self, a: Ty, h: Vec<Ty>) -> Ty {
        self.diff_mode(a, h, Mode::Env)
    }

    pub(super) fn diff_mode(&mut self, a: Ty, h: Vec<Ty>, mode: Mode) -> Ty {
        let mode = mode.max(self.mode(&a));
        let env = mode != Mode::Value;
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
                    .map(|m| self.diff_mode(m, hs.clone(), mode))
                    .collect();
                self.union_in(parts, mode)
            }
            Ty::Con(..) if a.scalar().is_some() && mode != Mode::Env => {
                let sets: Vec<_> = hs
                    .iter()
                    .filter(|m| m.head() == a.head())
                    .map(|m| m.values().clone())
                    .collect();
                if sets.is_empty() {
                    a
                } else {
                    let excluded = super::lits::ValueSet::join_all(&sets);
                    a.with_values(a.values().minus(&excluded))
                }
            }
            Ty::Con(..) | Ty::Fn(_) => match hs.iter().find(|m| m.head() == a.head()) {
                Some(m) => {
                    let m = m.clone();
                    if mode == Mode::Env {
                        // The need `a` goes to the handler `m` (rule B).
                        self.constrain_in(&a, &m, Mode::Env);
                    } else if env {
                        self.equal(&a, &m, FailKind::EnvForms);
                    } else {
                        self.constrain(&a, &m);
                    }
                    Ty::Never
                }
                None => a,
            },
            Ty::Diff(b, mut h0) => {
                self.diff_bound_mode(&b, &hs, mode);
                for m in hs {
                    if !h0.contains(&m) {
                        h0.push(m);
                    }
                }
                Ty::Diff(b, h0)
            }
            a => {
                self.diff_bound_mode(&a, &hs, mode);
                Ty::Diff(Box::new(a), hs)
            }
        }
    }

    /// Normalize closed or open type algebra without losing the row
    /// mode of constructor arguments or function effects.
    pub fn normalize_mode(&mut self, t: Ty, mode: Mode) -> Ty {
        let mode = mode.max(self.mode(&t));
        match t {
            Ty::Con(h, values, args) => {
                let modes = self.rows(h, args.len());
                Ty::Con(
                    h,
                    values,
                    args.into_iter()
                        .zip(modes)
                        .map(|(t, mode)| self.normalize_mode(t, mode))
                        .collect(),
                )
            }
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                param: self.normalize_mode(f.param, Mode::Value),
                labels: f.labels,
                ret: self.normalize_mode(f.ret, Mode::Value),
                err: self.normalize_mode(f.err, Mode::Row),
                env: self.normalize_mode(f.env, Mode::Env),
            })),
            Ty::Union(ms) => {
                let ms = ms
                    .into_iter()
                    .map(|t| self.normalize_mode(t, mode))
                    .collect();
                self.union_mode(ms, mode)
            }
            Ty::Inter(ms) => {
                let ms: Vec<_> = ms
                    .into_iter()
                    .map(|t| self.normalize_mode(t, mode))
                    .collect();
                let scalar = ms.iter().any(|t| t.scalar().is_some());
                self.inter_in(
                    ms,
                    if mode == Mode::Row && scalar {
                        Mode::Value
                    } else {
                        mode
                    },
                )
            }
            Ty::Diff(t, hs) => {
                let t = self.normalize_mode(*t, mode);
                let hs = hs
                    .into_iter()
                    .map(|t| self.normalize_mode(t, mode))
                    .collect();
                self.diff_mode(t, hs, mode)
            }
            Ty::Both(a, b) => Ty::Both(
                Box::new(self.normalize_mode(*a, mode)),
                Box::new(self.normalize_mode(*b, mode)),
            ),
            t => t,
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
        self.diff_bound_mode(a, hs, if env { Mode::Env } else { Mode::Value });
    }

    fn diff_bound_mode(&mut self, a: &Ty, hs: &[Ty], mode: Mode) {
        let mode = mode.max(self.mode(a));
        if !matches!(a, Ty::Var(_)) {
            return;
        }
        if let Ty::Var(v) = a
            && mode != Mode::Value
        {
            self.mark_mode(a, mode);
            for h in hs {
                self.handle(*v, h);
            }
            return;
        }
        let r = Ty::Var(self.fresh_at(self.level_of(a)));
        let mut ms = hs.to_vec();
        ms.push(r);
        self.constrain_in(a, &Ty::Union(ms), mode);
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
            self.normalize_type(&Ty::Union(roots.iter().copied().map(Ty::Var).collect()));
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
    fn normalize_bounds(&mut self, roots: &[V]) -> bool {
        let mut changed = false;
        let mut done = HashSet::default();
        for &v in roots {
            if (v.0 as usize) < self.rigid_boundary || !done.insert(v) {
                continue;
            }
            let (below, lower) = self.reach(vec![v], true);
            let before = self.var(v).lower.len();
            self.var_mut(v).lower.retain(|b| {
                !matches!(b, Ty::Diff(a, _) if matches!(a.as_ref(), Ty::Var(u) if below.contains(u)))
            });
            changed |= before != self.var(v).lower.len();
            let upper = self.var(v).upper.clone();
            let exact = upper.iter().find(|t| match t {
                Ty::Never => true,
                Ty::Con(..) | Ty::Fn(_) => lower.contains(t),
                Ty::Union(ms) => ms.iter().all(|m| lower.contains(m)),
                _ => false,
            });
            if let Some(t) = exact {
                if self.var(v).lower == [t.clone()] && self.var(v).upper == [t.clone()] {
                    continue;
                }
                changed = true;
                let old_lower = std::mem::take(&mut self.var_mut(v).lower);
                let old_upper = std::mem::take(&mut self.var_mut(v).upper);
                for b in old_lower {
                    if let Ty::Var(u) = b
                        && (u.0 as usize) >= self.rigid_boundary
                    {
                        self.var_mut(u).into.retain(|w| *w != v);
                    }
                }
                for b in old_upper {
                    if let Ty::Var(u) = b
                        && (u.0 as usize) >= self.rigid_boundary
                    {
                        self.var_mut(u).from.retain(|w| *w != v);
                    }
                }
                self.var_mut(v).lower = vec![t.clone()];
                self.var_mut(v).upper = vec![t.clone()];
            } else {
                let (vars, _) = self.reach(vec![v], false);
                let before = self.var(v).upper.len();
                self.var_mut(v).upper.retain(|b| {
                    !matches!(b, Ty::Union(ms) if ms.iter().any(|m| matches!(m, Ty::Var(u) if vars.contains(u))))
                });
                changed |= self.var(v).upper.len() != before;
            }
        }
        self.rewrites += u64::from(changed);
        changed
    }

    /// Normalize the whole component that coalescing can reach, including
    /// variable aliases stored in the reverse links. Removing one bound
    /// can expose another redundant bound, so continue to a fixed point.
    fn normalize_type(&mut self, t: &Ty) -> bool {
        let mut edges = Vec::new();
        bound_vars(self, t, true, false, &mut edges);
        let mut todo: Vec<V> = edges.drain(..).map(|e| e.0).collect();
        let mut seen = HashSet::default();
        let mut roots = Vec::new();
        while let Some(v) = todo.pop() {
            if !seen.insert(v) {
                continue;
            }
            roots.push(v);
            let st = self.var(v);
            todo.extend(st.from.iter().chain(&st.into).copied());
            for b in st.lower.iter().chain(&st.upper) {
                bound_vars(self, b, true, false, &mut edges);
            }
            todo.extend(edges.drain(..).map(|e| e.0));
        }
        let mut changed = false;
        while self.normalize_bounds(&roots) {
            changed = true;
        }
        changed
    }

    /// A canonical view for coalescing. Occurs checking and coalescing use
    /// the same reductions. A view keeps variable ids and levels, but needs
    /// no constraint caches: normalization neither adds constraints nor
    /// allocates variables. Only the rare failed coalescence copies state.
    pub fn normalized(&self, t: &Ty) -> Option<Solver> {
        let mut view = Solver::new(self.variance.clone(), self.type_names.clone());
        view.vars = self.vars.clone();
        view.has_need_meets = self.has_need_meets;
        view.row_params = self.row_params.clone();
        view.normalize_type(t).then_some(view)
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
                        if st.row != Mode::Value {
                            env_vars(self, b, p, con, &mut out);
                        } else {
                            bound_vars(self, b, p, con, &mut out);
                        }
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
        if st.row != Mode::Value {
            env_vars(s, b, p, false, &mut out);
        } else {
            bound_vars(s, b, p, false, &mut out);
        }
    }
    out
}

/// The variables in a bound at a place of polarity `pos`, each with its
/// polarity and whether it is inside a type constructor, as coalescing
/// goes into them.
fn bound_vars(s: &Solver, t: &Ty, pos: bool, con: bool, out: &mut Vec<(V, bool, bool)>) {
    match t {
        Ty::Var(v) => out.push((*v, pos, con)),
        Ty::Con(h, _, args) => {
            let vs = s.variances(*h, args.len());
            for ((a, v), row) in args.iter().zip(vs).zip(s.rows(*h, args.len())) {
                if row != Mode::Value {
                    env_vars(s, a, pos, true, out);
                    continue;
                }
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
            bound_vars(s, &f.param, !pos, true, out);
            bound_vars(s, &f.ret, pos, true, out);
            env_vars(s, &f.err, pos, true, out);
            env_vars(s, &f.env, pos, true, out);
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
fn env_vars(s: &Solver, t: &Ty, pos: bool, con: bool, out: &mut Vec<(V, bool, bool)>) {
    match t {
        Ty::Con(_, _, args) => {
            for a in args {
                bound_vars(s, a, true, true, out);
                bound_vars(s, a, false, true, out);
            }
        }
        Ty::Union(ms) | Ty::Inter(ms) => ms.iter().for_each(|a| env_vars(s, a, pos, con, out)),
        Ty::Diff(a, h) => {
            env_vars(s, a, pos, con, out);
            h.iter().for_each(|x| bound_vars(s, x, pos, true, out));
        }
        t => bound_vars(s, t, pos, con, out),
    }
}

/// Whether two parts of bounds may hold function types, whose
/// environments must be checked when the bounds are put together.
fn needs_join(t: &Ty) -> bool {
    match t {
        Ty::Con(_, _, args) => !args.is_empty(),
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
    builtin_variance_values(b).to_vec()
}

fn builtin_variance_values(b: BuiltinType) -> &'static [Variance] {
    use Variance::*;
    match b {
        BuiltinType::List | BuiltinType::Request => &[Co],
        BuiltinType::Key => &[Inv],
        BuiltinType::Boundary => &[Inv, Inv, Inv],
        BuiltinType::Continuation => &[Contra, Co, Co, Co],
        _ => &[],
    }
}

#[cfg(test)]
mod primitive_variance_tests {
    use super::*;

    #[test]
    fn typed_keys_are_invariant() {
        assert_eq!(builtin_variance(BuiltinType::Key), vec![Variance::Inv]);
    }
}

#[cfg(test)]
mod literal_tests {
    use super::*;
    use crate::types::lits::ValueSet;
    use std::sync::Arc;

    fn solver() -> Solver {
        Solver::new(StdHashMap::new(), Vec::new())
    }

    #[test]
    fn closed_data_preserves_need_direction_inside_nominal_arguments() {
        // The former record-prover probe now uses closed labelled data.
        // §4.5 rule 4 compares the provider in reverse as a value.
        let mut s = solver();
        let (outer, service) = (TypeId(10), TypeId(11));
        s.variance.insert(outer, vec![Variance::Co]);
        s.variance.insert(service, vec![Variance::Co]);
        s.row_params.insert(outer, vec![Mode::Env]);
        let int = Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]);
        let string = Ty::named(TypeRef::Builtin(BuiltinType::String), vec![]);
        let wide_arg = s.union(vec![int.clone(), string]);
        let form = |arg| {
            Ty::record(vec![(
                "x".into(),
                Ty::named(
                    TypeRef::Custom(outer),
                    vec![Ty::named(TypeRef::Custom(service), vec![arg])],
                ),
            )])
        };
        let (narrow, wide) = (form(int), form(wide_arg));
        let mut forward = s.clone();
        forward.constrain(&narrow, &wide);
        assert!(!forward.failures.is_empty());
        s.constrain(&wide, &narrow);
        assert!(s.failures.is_empty());
    }

    fn strings(values: &[&str]) -> Ty {
        ValueSet::only(values.iter().map(|s| Arc::from(*s)))
            .map_or(Ty::Never, |set| Ty::lit(BuiltinType::String, set))
    }

    fn except(values: &[&str]) -> Ty {
        Ty::lit(
            BuiltinType::String,
            ValueSet::except(values.iter().map(|s| Arc::from(*s))),
        )
    }

    fn variable(t: &Ty) -> V {
        let Ty::Var(v) = t else {
            panic!("not a variable")
        };
        *v
    }

    #[test]
    fn literal_subtyping_and_right_difference() {
        // literal-types §5.1 rule 1 and §10's solver cases.
        for (a, b, ok) in [
            (strings(&["a"]), strings(&["a", "b"]), true),
            (strings(&["a", "b"]), except(&[]), true),
            (except(&[]), strings(&["a"]), false),
            (except(&["a"]), strings(&["b"]), false),
            (strings(&["b"]), except(&["a"]), true),
            (strings(&["a"]), except(&["a"]), false),
            (
                strings(&["a"]),
                Ty::Diff(Box::new(except(&[])), vec![strings(&["a"])]),
                false,
            ),
            (
                strings(&["b"]),
                Ty::Diff(Box::new(except(&[])), vec![strings(&["a"])]),
                true,
            ),
        ] {
            let mut s = solver();
            s.constrain(&a, &b);
            assert_eq!(
                s.failures.is_empty(),
                ok,
                "{a:?} <: {b:?}: {:?}",
                s.failures
            );
        }
    }

    #[test]
    fn route_only_the_uncovered_values() {
        // §5.1 rule 3: S <: X | v iff S - X <: v.
        let mut s = solver();
        let v = s.fresh();
        s.constrain(&except(&[]), &Ty::Union(vec![strings(&["a"]), v.clone()]));
        assert!(s.failures.is_empty(), "{:?}", s.failures);
        assert_eq!(s.var(variable(&v)).lower, [except(&["a"])]);

        let mut s = solver();
        let v = s.fresh();
        // This raw union deliberately has two members of the same head.
        s.constrain(
            &except(&[]),
            &Ty::Union(vec![strings(&["a"]), strings(&["b"]), v.clone()]),
        );
        assert_eq!(s.var(variable(&v)).lower, [except(&["a", "b"])]);
        assert!(s.failures.is_empty());

        let mut s = solver();
        let (v, w) = (s.fresh(), s.fresh());
        s.constrain(&except(&[]), &Ty::Union(vec![strings(&["a"]), v, w]));
        assert_eq!(s.failures.len(), 1);
        assert_eq!(s.failures[0].kind, FailKind::Ambiguous);

        let mut s = solver();
        s.constrain(
            &except(&[]),
            &Ty::Union(vec![strings(&["a"]), Ty::tuple(Vec::new())]),
        );
        assert_eq!(s.failures.len(), 1);
        assert_eq!(s.failures[0].kind, FailKind::Values);
        assert_eq!(s.failures[0].lhs, except(&[]));
    }

    #[test]
    fn difference_left_all_three_splits_and_later_bounds() {
        // §5.1 rule 4: subset, disjoint, and partially overlapping sets.
        for (source, expected) in [
            (strings(&["a"]), Vec::new()),
            (strings(&["b"]), vec![strings(&["b"])]),
            (strings(&["a", "b"]), vec![strings(&["b"])]),
        ] {
            let mut s = solver();
            let out = s.fresh();
            s.diff_left(&source, &[strings(&["a"])], &out, Mode::Value);
            assert!(s.failures.is_empty());
            assert_eq!(s.var(variable(&out)).lower, expected);
        }
        let mut s = solver();
        let (v, out) = (s.fresh(), s.fresh());
        s.diff_left(&v, &[strings(&["a"])], &out, Mode::Value);
        s.constrain(&except(&[]), &v);
        assert_eq!(s.var(variable(&out)).lower, [except(&["a"])]);
        assert!(s.failures.is_empty());
        // The public difference constructor leaves its variable live too.
        let mut s = solver();
        let v = s.fresh();
        let diff = s.diff(v.clone(), vec![strings(&["a"])]);
        s.constrain(&diff, &except(&["a"]));
        s.constrain(&except(&[]), &v);
        assert!(s.failures.is_empty(), "{:?}", s.failures);
    }

    #[test]
    fn scalar_boolean_algebra_and_never_intersections() {
        // §5.1 rules 5–7; assertions follow the set equations in §4.
        let mut s = solver();
        assert_eq!(s.inter(vec![strings(&["a"]), strings(&["b"])]), Ty::Never);
        assert_eq!(
            s.inter(vec![strings(&["a", "b"]), except(&["a"])]),
            strings(&["b"])
        );
        assert_eq!(s.diff(except(&[]), vec![strings(&["a"])]), except(&["a"]));
        assert_eq!(s.diff(strings(&["a"]), vec![except(&[])]), Ty::Never);
        assert_eq!(s.union(vec![strings(&["a"]), except(&["a"])]), except(&[]));
        assert_eq!(
            s.union(vec![strings(&["a"]), strings(&["b"]), strings(&["a"])]),
            strings(&["a", "b"])
        );
        assert_eq!(
            s.diff(except(&[]), vec![strings(&["a"]), strings(&["b"])]),
            except(&["a", "b"])
        );
        let v = s.fresh();
        assert_eq!(
            s.inter(vec![v, strings(&["a"]), strings(&["b"])]),
            Ty::Never
        );
        assert!(s.failures.is_empty());
    }

    #[test]
    fn bounds_fuse_without_losing_constraints() {
        // §5.1 rule 8: one bound for 1000 literals, and all propagated checks.
        let mut s = solver();
        let v = s.fresh();
        for i in 0..1000 {
            s.constrain(&strings(&[&i.to_string()]), &v);
        }
        let bounds = &s.var(variable(&v)).lower;
        assert_eq!(bounds.len(), 1);
        assert_eq!(bounds[0].values().items().count(), 1000);
        assert!(s.failures.is_empty());
        s.constrain(&v, &strings(&["x", "y"]));
        assert!(!s.failures.is_empty());

        let mut s = solver();
        let v = s.fresh();
        s.constrain(&v, &strings(&["a", "b", "c"]));
        s.constrain(&v, &strings(&["b", "c", "d"]));
        assert_eq!(s.var(variable(&v)).upper, [strings(&["b", "c"])]);
        s.constrain(&strings(&["b"]), &v);
        assert!(s.failures.is_empty());
        s.constrain(&strings(&["a"]), &v);
        assert_eq!(s.failures.len(), 1);

        let mut s = solver();
        let v = s.fresh();
        s.constrain(&v, &strings(&["a"]));
        s.constrain(&v, &strings(&["b"]));
        assert_eq!(s.var(variable(&v)).upper.len(), 2);
        s.constrain(&strings(&["b"]), &v);
        assert_eq!(s.failures.len(), 1);
    }

    #[test]
    fn fused_bounds_propagate_through_variable_links() {
        let mut s = solver();
        let (v, w) = (s.fresh(), s.fresh());
        s.constrain(&v, &w);
        s.constrain(&strings(&["a"]), &v);
        s.constrain(&strings(&["b"]), &v);
        s.constrain(&w, &strings(&["a"]));
        assert!(!s.failures.is_empty());
        // A late link must check the complete fused lower bound.
        let mut s = solver();
        let (v, w) = (s.fresh(), s.fresh());
        s.constrain(&strings(&["a"]), &v);
        s.constrain(&strings(&["b"]), &v);
        s.constrain(&w, &strings(&["a"]));
        s.constrain(&v, &w);
        assert!(!s.failures.is_empty());
    }

    #[test]
    fn deferred_need_meet_keeps_both_upper_bounds_and_reports_late_conflicts() {
        let service = TypeRef::Custom(TypeId(0));
        let mut s = solver();
        let (x, y) = (s.fresh(), s.fresh());
        let a = Ty::named(service, vec![x.clone()]);
        let b = Ty::named(service, vec![y.clone()]);
        let meet = s.need_meet(&x, &y, &a, &b, Mode::Value, 0);
        assert!(s.failures.is_empty());
        let Ty::Var(v) = meet else {
            panic!("a deferred meet is a variable")
        };
        assert_eq!(s.var(v).upper, [x.clone(), y.clone()]);
        // The arguments remain independent: the meet does not equate them.
        assert!(s.var(variable(&x)).upper.is_empty());
        assert!(s.var(variable(&y)).upper.is_empty());
        let int = Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new());
        let string = Ty::named(TypeRef::Builtin(BuiltinType::String), Vec::new());
        let link = s.fresh();
        s.constrain(&x, &link);
        s.constrain(&link, &int);
        assert!(s.failures.is_empty());
        s.constrain(&y, &string);
        assert!(
            s.failures
                .iter()
                .any(|f| f.kind == FailKind::NeedMeet && f.lhs == a && f.rhs == b)
        );
    }

    #[test]
    fn service_providers_do_not_widen_needed_literal_arguments() {
        // §4.5 rule 4: Counter(Int) is not a subtype of the needed Counter(0).
        let counter = TypeId(0);
        let make = |arg| Ty::named(TypeRef::Custom(counter), vec![arg]);
        let zero = Ty::lit(BuiltinType::Int, ValueSet::only([Arc::from("0")]).unwrap());
        let int = Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new());
        let mut s = solver();
        s.variance.insert(counter, vec![Variance::Co]);
        s.constrain_env(&make(zero.clone()), &make(int.clone()));
        assert!(!s.failures.is_empty());
        assert!(s.failures.iter().all(|f| f.kind == FailKind::EnvForms));
        s.failures.clear();
        s.constrain_env(&make(int.clone()), &make(int.clone()));
        assert!(s.failures.is_empty());
        s.constrain(&make(zero), &make(int));
        assert!(s.failures.is_empty());
    }

    #[test]
    fn ordinary_bounds_do_not_skip_a_later_environment_check() {
        // §5.1 rule 1 requires equality in env mode, even if ordinary
        // subtyping already accepted the same literal as a lower bound.
        let mut s = solver();
        let v = s.fresh();
        s.constrain(&strings(&["a"]), &v);
        s.constrain(&v, &except(&[]));
        assert!(s.failures.is_empty());
        s.constrain_env(&strings(&["a"]), &v);
        assert_eq!(s.failures.len(), 1);
        assert_eq!(s.failures[0].kind, FailKind::EnvForms);
    }
    #[test]
    fn environment_difference_checks_each_use_without_reversing_provider_direction() {
        // §9.2: P must be a subtype of S. A cached handler installation
        // must not suppress the conflict when the row is used again.
        let reader = TypeId(0);
        let make = |arg| Ty::named(TypeRef::Custom(reader), vec![arg]);
        let need = make(Ty::named(TypeRef::Builtin(BuiltinType::String), Vec::new()));
        let bad = make(Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new()));
        let good = make(Ty::lit(
            BuiltinType::String,
            ValueSet::only([Arc::from("ok")]).unwrap(),
        ));
        for (provider, valid) in [(bad, false), (good, true)] {
            let mut s = solver();
            s.variance.insert(reader, vec![Variance::Co]);
            let row = s.fresh();
            let rest = s.env_diff(row.clone(), vec![provider]);
            s.constrain_env(&need, &row);
            assert_eq!(s.failures.is_empty(), valid);
            for _ in 0..2 {
                s.failures.clear();
                s.env_union(vec![rest.clone()]);
                assert_eq!(s.failures.is_empty(), valid);
                assert!(s.failures.iter().all(|f| f.kind == FailKind::EnvForms));
            }
        }
    }
}

#[cfg(test)]
mod literal_stress_tests {
    use crate::hir::{ExprKind, PatternKind, Stmt};
    use crate::types::infer::Checker;
    use crate::types::{Head, Ty};

    fn stress(part: &str, n: usize) {
        // literal-types §9: check real programs and observe peak counts,
        // not just the final (possibly simplified) type of a definition.
        let dir = tempfile::tempdir().unwrap();
        let script =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/t1/gen-stress.py");
        assert!(
            std::process::Command::new("python3")
                .arg(script)
                .arg(dir.path())
                .arg(n.to_string())
                .arg(part)
                .status()
                .unwrap()
                .success()
        );
        let (graph, ds) = ilang_package::load(dir.path());
        assert!(ds.is_empty(), "{ds:?}");
        let (p, ds) = crate::resolve_program(&graph);
        assert!(ds.is_empty(), "{ds:?}");
        let p = p.unwrap();
        let mut checker = Checker::new(&p, &graph.sources);
        checker.keep_locals = true;
        let start = std::time::Instant::now();
        let (typed, ds) = checker.run();
        let elapsed = start.elapsed();
        assert!(ds.is_empty(), "{ds:?}");
        let locals = typed.locals.unwrap();
        let solver = locals.s.lock().unwrap();
        assert!(
            solver.peak_bound_count <= 2,
            "{part}({n}): peak {}",
            solver.peak_bound_count
        );
        if part == "calls" {
            // §6.10: the local function must be indirect and monomorphic.
            let module = p
                .modules
                .iter()
                .find(|m| Some(m.package) == graph.root)
                .unwrap();
            let ExprKind::Fn(f) = &module.bindings[0].binding.value.kind else {
                panic!("calls function")
            };
            let Stmt::Bind(b) = &f.body.stmts[0] else {
                panic!("local binding")
            };
            assert!(!b.direct);
            let PatternKind::Var(v) = &b.pattern.kind else {
                panic!("f binder")
            };
            let Ty::Fn(f) = &locals.types[&v.var] else {
                panic!("shared function shape")
            };
            let Some([Ty::Var(v)]) = f.parts() else {
                panic!("one shared parameter")
            };
            let bound = solver.vars[v.0 as usize]
                .lower
                .iter()
                .find(|t| {
                    t.head()
                        == Some(Head::Named(crate::hir::TypeRef::Builtin(
                            crate::hir::BuiltinType::String,
                        )))
                })
                .unwrap();
            assert_eq!(
                bound.values().items().count(),
                n,
                "all calls reach the same parameter"
            );
        }
        eprintln!(
            "{part} n={n}: {elapsed:?}, peak bounds/head/side={}",
            solver.peak_bound_count
        );
    }

    #[test]
    #[ignore = "literal-types §9 performance gate"]
    fn t1_stress_match_2000() {
        stress("pick", 2000);
    }

    #[test]
    #[ignore = "literal-types §9 performance gate"]
    fn t1_stress_calls_5000() {
        stress("calls", 2500);
        stress("calls", 5000);
    }

    #[test]
    #[ignore = "literal-types §9 performance gate"]
    fn t1_stress_alias_1000() {
        stress("alias", 1000);
    }

    #[test]
    #[ignore = "literal-types §9 performance gate"]
    fn t1_stress_nested_50_by_50() {
        stress("nested", 50);
    }
}

#[cfg(test)]
mod handler_proof_tests {
    use super::*;

    #[test]
    fn exact_bound_proofs_stop_at_constructor_cycles() {
        let mut s = Solver::new(StdHashMap::new(), Vec::new());
        let a = s.fresh();
        let b = s.fresh();
        let list = |t| Ty::named(TypeRef::Builtin(BuiltinType::List), vec![t]);
        for t in [&a, &b] {
            let recursive = list(list(t.clone()));
            s.constrain(t, &recursive);
            s.constrain(&recursive, t);
        }
        assert!(!s.same_form(&a, &list(b)));
        let Ty::Var(v) = a else { unreachable!() };
        assert!(s.occurs(vec![v]));
    }
}

#[cfg(test)]
mod reach_marks_tests {
    use super::*;

    #[test]
    fn each_walk_starts_unmarked_also_after_the_stamp_wraps() {
        let marks = ReachMarks::default();
        {
            let mut w = marks.start(4);
            assert!(w.insert(V(1)));
            assert!(!w.insert(V(1)));
            assert!(w.insert(V(9)));
            assert!(w.insert(V(2)));
        }
        {
            let mut w = marks.start(4);
            assert!(w.insert(V(1)));
            assert!(w.insert(V(9)));
        }
        marks.0.borrow_mut().0 = u32::MAX;
        let mut w = marks.start(4);
        assert!(w.insert(V(1)));
        assert!(!w.insert(V(1)));
        assert!(w.insert(V(2)));
    }

    #[test]
    fn reach_marks_are_not_solver_state() {
        let mut a = Solver::new(Default::default(), Vec::new());
        let v = a.fresh_at(0);
        let b = a.clone();
        let _ = a.reach(vec![v], true);
        assert_eq!(a, b);
        let c = a.clone();
        assert!(c.reach_marks.0.borrow().1.is_empty());
        assert_eq!(c.reach(vec![v], true), a.reach(vec![v], true));
        assert_eq!(a, c);
    }

    #[test]
    fn reach_preserves_order_observations_and_changed_bounds() {
        let mut s = Solver::new(Default::default(), Vec::new());
        let [a, b, c, d] = std::array::from_fn(|_| s.fresh_at(0));
        let int = Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new());
        let string = Ty::named(TypeRef::Builtin(BuiltinType::String), Vec::new());
        s.var_mut(a).lower = vec![Ty::Var(b), int.clone(), int.clone()];
        s.var_mut(a).from = vec![c];
        s.var_mut(b).lower = vec![Ty::Var(a), string.clone()];
        s.var_mut(c).lower = vec![Ty::Var(b), int.clone()];
        s.var_mut(a).upper = vec![Ty::Var(d), string.clone()];
        s.var_mut(a).into = vec![b];
        s.var_mut(b).upper = vec![Ty::Var(a), int.clone()];
        s.var_mut(d).upper = vec![Ty::Var(a), string.clone()];
        s.observation = Some((s.vars.len(), HashSet::from_iter([c]), Default::default()));

        assert_eq!(
            s.reach(vec![b, a, a], true),
            (vec![a, c, b], vec![int.clone(), string.clone()])
        );
        assert_eq!(
            s.reach(vec![a], false),
            (vec![a, b, d], vec![string.clone(), int.clone()])
        );
        assert_eq!(
            *s.observation.as_ref().unwrap().2.borrow(),
            HashSet::from_iter([a, b, d])
        );

        s.var_mut(a).lower = vec![int.clone()];
        s.var_mut(a).from.clear();
        assert_eq!(s.reach(vec![a], true), (vec![a], vec![int.clone()]));
        let e = s.fresh_at(0);
        s.var_mut(a).from.push(e);
        s.var_mut(e).lower = vec![string.clone()];
        assert_eq!(s.reach(vec![a], true), (vec![a, e], vec![int, string]));
    }
}
