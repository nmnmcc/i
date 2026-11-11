//! The constraint solver: type variables with levels and bounds, as in
//! Simple-sub, extended with the unions, intersections, and differences of
//! §4.4 and with the variance of type parameters (§4.5).
//!
//! Every variable has a level, and lower and upper bounds. `constrain(l, r)`
//! keeps `l <: r` true for every bound that is added, so the bounds of a
//! variable are always consistent. A type at a deeper level is copied to
//! the level of the variable first (extrusion), so that generalization only
//! takes the variables of the binding.

use super::TyParts;
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
    /// The variables of the cycles that the occurs check has reported.
    reported: HashSet<V>,
    /// The pairs of bounds whose parts `add_bound` has put together.
    compared: HashSet<(Ty, Ty, bool, Mode)>,
    pub variance: StdHashMap<TypeId, Vec<Variance>>,
    has_handlers: bool,
    pub row_params: StdHashMap<TypeId, Vec<Mode>>,
    pub failures: Vec<Failure>,
    /// The names of the custom types, by `TypeId`, to show types.
    pub type_names: Vec<String>,
    /// A failed coalescence is diagnostic recovery, never an unknown type
    /// that may silently make a program pass. The checker drains this at
    /// binding and artifact boundaries, including known-type conversions.
    pub coalescing_failed: std::cell::Cell<bool>,
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
    pub fn new(variance: StdHashMap<TypeId, Vec<Variance>>, type_names: Vec<String>) -> Solver {
        Solver {
            coalescing_failed: std::cell::Cell::new(false),
            observed_shape: false,
            observation: None,
            vars: Vec::new(),
            #[cfg(test)]
            peak_bound_count: 0,
            level: 0,
            cache: HashSet::default(),
            compared: HashSet::default(),
            touched: Vec::new(),
            reported: HashSet::default(),
            variance,
            row_params: StdHashMap::new(),
            has_handlers: false,
            failures: Vec::new(),
            type_names,
        }
    }

    /// A scratch solver for closed types keeps constructor row modes.
    pub fn for_closed_types(&self) -> Solver {
        let mut solver = Solver::new(self.variance.clone(), self.type_names.clone());
        solver.row_params = self.row_params.clone();
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
        if self.observation.is_some() {
            super::tree::visit(t, |t| {
                if let Ty::Var(v) = t {
                    self.observe(*v);
                }
            });
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
            row: Mode::Value,
            difference: false,
            handled: Vec::new(),
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
        #[cfg(test)]
        self.observe_bounds(v, false);
    }

    /// Sets the lower bounds of a new variable.
    fn set_lower(&mut self, v: V, lower: Vec<Ty>) {
        for l in &lower {
            if let Ty::Var(w) = l {
                self.var_mut(*w).into.push(v);
            }
        }
        self.var_mut(v).lower = lower;
        #[cfg(test)]
        self.observe_bounds(v, true);
    }

    fn var(&self, v: V) -> &VarState {
        &self.vars[v.0 as usize]
    }

    fn var_mut(&mut self, v: V) -> &mut VarState {
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
            Head::Tuple(_) | Head::Fn => ParamModes::repeated(Variance::Co, n),
        }
    }

    fn fail(&mut self, kind: FailKind, lhs: &Ty, rhs: &Ty) {
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
        let mut pending = vec![t];
        let mut level = 0;
        while let Some(t) = pending.pop() {
            match t {
                Ty::Var(v) => level = level.max(self.var(*v).level),
                Ty::Con(_, _, args) | Ty::Union(args) | Ty::Inter(args) => pending.extend(args),
                Ty::Fn(f) => {
                    pending.extend(f.params.iter().map(|(_, t)| t));
                    pending.extend([&f.ret, &f.err, &f.env]);
                }
                Ty::Diff(a, h) => {
                    pending.push(a);
                    pending.extend(h);
                }
                Ty::Both(a, b) => {
                    pending.push(a);
                    pending.push(b);
                }
                Ty::Never | Ty::Top => {}
            }
        }
        level
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
    /// member of `M` with the same type constructor and equal arguments,
    /// whatever the variance of the parameters.
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
            match t.into_parts() {
                TyParts::Var(v) if self.var(v).row < mode => {
                    self.observe(v);
                    self.var_mut(v).row = mode;
                    added.push(v);
                    let st = self.var(v);
                    todo.extend(st.lower.iter().chain(&st.upper).cloned());
                    todo.extend(st.from.iter().chain(&st.into).copied().map(Ty::Var));
                }
                TyParts::Union(ms) | TyParts::Inter(ms) => todo.extend(ms),
                TyParts::Diff(a, _) => todo.push(*a),
                TyParts::Both(a, b) => {
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
        self.mode(t) != Mode::Value
    }

    fn mode(&self, t: &Ty) -> Mode {
        let mut pending = vec![t];
        let mut mode = Mode::Value;
        while let Some(t) = pending.pop() {
            match t {
                Ty::Var(v) => mode = mode.max(self.var(*v).row),
                Ty::Union(ms) | Ty::Inter(ms) => pending.extend(ms),
                Ty::Diff(a, _) => pending.push(a),
                _ => {}
            }
        }
        mode
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
        self.has_handlers = true;
        for w in self.reach(vec![v], true).0 {
            if self.var(w).handled.contains(h) {
                continue;
            }
            self.observe(w);
            self.var_mut(w).difference = true;
            self.var_mut(w).handled.push(h.clone());
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
        let mut pending = vec![(a, b)];
        while let Some((a, b)) = pending.pop() {
            if a == b {
                continue;
            }
            match (a, b) {
                (Ty::Con(h, values, xs), Ty::Con(k, other, ys))
                    if h == k && values == other && xs.len() == ys.len() =>
                {
                    pending.extend(xs.iter().zip(ys).rev());
                }
                (Ty::Fn(f), Ty::Fn(g)) if f.params.len() == g.params.len() => {
                    pending.push((&f.env, &g.env));
                    pending.push((&f.err, &g.err));
                    pending.push((&f.ret, &g.ret));
                    pending.extend(
                        f.params
                            .iter()
                            .zip(&g.params)
                            .rev()
                            .map(|(p, q)| (&p.1, &q.1)),
                    );
                }
                (Ty::Both(a, b), Ty::Both(c, d)) => {
                    pending.push((b, d));
                    pending.push((a, c));
                }
                (Ty::Union(xs), Ty::Union(ys)) | (Ty::Inter(xs), Ty::Inter(ys))
                    if xs.len() == 1 && ys.len() == 1 =>
                {
                    pending.push((&xs[0], &ys[0]));
                }
                (Ty::Diff(a, xs), Ty::Diff(b, ys)) if xs.is_empty() && ys.is_empty() => {
                    pending.push((a, b));
                }
                _ if !self.same_form_recursive(a, b, expanding) => return false,
                _ => {}
            }
        }
        true
    }

    fn same_form_recursive(&self, a: &Ty, b: &Ty, expanding: &mut HashSet<V>) -> bool {
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
            (Ty::Union(xs), Ty::Union(ys)) | (Ty::Inter(xs), Ty::Inter(ys)) => {
                sets(xs, ys, expanding)
            }
            (Ty::Diff(a, hs), Ty::Diff(b, ks)) => {
                self.same_form_in(a, b, expanding) && sets(hs, ks, expanding)
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
        let mut all = Vec::new();
        super::tree::visit(t, |part| all.push(part));
        let mut hidden = Vec::new();
        for v in all
            .iter()
            .copied()
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
                    let witnessed = all.iter().copied().any(|part| match part {
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
        enum Work<'a> {
            Check(&'a Ty, &'a Ty, Mode),
            Equal(&'a Ty, &'a Ty, FailKind),
            FinishEqual(&'a Ty, &'a Ty, FailKind, usize),
        }
        let mut work = vec![Work::Check(l, r, env)];
        while let Some(next) = work.pop() {
            let (l, r, env) = match next {
                Work::Check(l, r, env) => (l, r, env),
                Work::Equal(l, r, kind) => {
                    work.push(Work::FinishEqual(l, r, kind, self.failures.len()));
                    work.push(Work::Check(r, l, Mode::Value));
                    work.push(Work::Check(l, r, Mode::Value));
                    continue;
                }
                Work::FinishEqual(l, r, kind, before) => {
                    if self.failures.len() > before {
                        self.failures.truncate(before);
                        self.fail(kind, l, r);
                    }
                    continue;
                }
            };
            let env = env.max(self.mode(l)).max(self.mode(r));
            if env != Mode::Value {
                self.mark_mode(l, env);
                self.mark_mode(r, env);
            }
            if self.observed_shape
                && (matches!(r, Ty::Union(ts) if ts.contains(l))
                    || matches!(l,Ty::Inter(ts) if ts.contains(r)))
            {
                continue;
            }
            if l != r {
                self.observe_type(l);
                self.observe_type(r);
            }
            if l == r || matches!(l, Ty::Never) || matches!(r, Ty::Top) {
                continue;
            }
            // §4.5 rule 3: subtracting members never adds values. The
            // difference's separate same-form obligation remains in the scheme.
            if let Ty::Diff(a, _) = l
                && (a.as_ref() == r
                    || matches!((a.as_ref(), r), (Ty::Var(u), Ty::Var(v))
                if self.reach(vec![*u], false).0.contains(v)))
            {
                continue;
            }
            // §4.5 rule 2: these are inclusions, not recursive bounds such as
            // `n <: n | S(n)`. Recording them would invent an occurs cycle.
            if matches!(l, Ty::Var(_)) && matches!(r, Ty::Union(ms) if ms.contains(l)) {
                continue;
            }
            if (matches!(l, Ty::Var(_)) || matches!(r, Ty::Var(_)))
                && !self.cache.insert((l.clone(), r.clone(), env))
            {
                continue;
            }
            match (l, r) {
                (Ty::Union(ms), _) => {
                    for m in ms.iter().rev() {
                        work.push(Work::Check(m, r, env));
                    }
                }
                (_, Ty::Inter(ms)) => {
                    for m in ms.iter().rev() {
                        work.push(Work::Check(l, m, env));
                    }
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
                        continue;
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
                    let (conc, vars): (Vec<Ty>, Vec<Ty>) = ms
                        .iter()
                        .cloned()
                        .partition(|m| matches!(m, Ty::Con(..) | Ty::Fn(_)));
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
                (Ty::Both(a, _), _) => work.push(Work::Check(a, r, env)),
                (_, Ty::Both(_, b)) => work.push(Work::Check(l, b, env)),
                (_, Ty::Union(ms)) => self.route(l, ms, env),
                (_, Ty::Diff(a, h)) => {
                    if h.iter()
                        .any(|m| m.head() == l.head() && !m.values().disjoint(l.values()))
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
                        work.push(Work::Check(l, a, env));
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
                        continue;
                    }
                    if env != Mode::Value {
                        // §4.5 rule 4: service arguments are equal.
                        for (x, y) in a1.iter().zip(a2).rev() {
                            work.push(Work::Equal(x, y, FailKind::EnvForms));
                        }
                        continue;
                    }
                    let vs = self.variances(*h1, a1.len());
                    let children: Vec<_> = a1
                        .iter()
                        .zip(a2)
                        .zip(vs)
                        .zip(self.rows(*h1, a1.len()))
                        .collect();
                    for (((x, y), v), row) in children.into_iter().rev() {
                        match v {
                            Variance::Co => work.push(Work::Check(x, y, row)),
                            Variance::Contra => work.push(Work::Check(y, x, row)),
                            Variance::Inv => {
                                work.push(Work::Check(y, x, row));
                                work.push(Work::Check(x, y, row));
                            }
                            Variance::Bi => {}
                        }
                    }
                }
                (Ty::Fn(f), Ty::Fn(g)) => {
                    if f.params.len() != g.params.len() {
                        self.fail(FailKind::Arity, l, r);
                        continue;
                    }
                    work.push(Work::Check(&f.env, &g.env, Mode::Env));
                    work.push(Work::Check(&f.err, &g.err, Mode::Row));
                    work.push(Work::Check(&f.ret, &g.ret, Mode::Value));
                    for (p, q) in f.params.iter().zip(&g.params).rev() {
                        work.push(Work::Check(&q.1, &p.1, Mode::Value));
                    }
                }
                _ => self.fail(FailKind::Mismatch, l, r),
            }
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
    fn add_bound(&mut self, v: V, t: Ty, lower: bool, env: Mode) -> bool {
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
            Ty::Con(..) | Ty::Fn(_) => vec![t.clone()],
            _ => Vec::new(),
        };
        // No constructor pairs need checking when the new bound has none.
        if !new.is_empty() {
            let level = self.var(v).level;
            // The variables whose bounds now have `t` too, and their bounds.
            let preds = self.reach(vec![v], !lower).0;
            let mut others = self.reach(preds, lower).1;
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
                    if a.head() == h.head() {
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
            if let Ty::Var(w) = &t {
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
        let mut visited: HashSet<V> = HashSet::default();
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
                    Ty::Con(..) | Ty::Fn(_) if found.insert(b) => out.push(b.clone()),
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
        let lower = lower && first;
        match (a, b) {
            (Ty::Fn(f), Ty::Fn(g)) => {
                if g.params.len() != f.params.len() {
                    // All function types are one head; their parameters
                    // must agree in number to be put together.
                    self.fail(FailKind::Arity, b, a);
                } else if lower {
                    self.join(&f.ret, &g.ret, level, Mode::Value);
                    self.join(&f.err, &g.err, level, Mode::Row);
                    self.join(&f.env, &g.env, level, Mode::Env);
                }
            }
            (Ty::Con(h, _, xs), Ty::Con(_, _, ys)) if xs.len() == ys.len() => {
                let vs = self.variances(*h, xs.len());
                for (((x, y), var), row) in xs.iter().zip(ys).zip(vs).zip(self.rows(*h, xs.len())) {
                    if env != Mode::Value {
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
        let h = l.head();
        let mut rest = None;
        if h.is_some()
            && let Some(m) = flat.iter().find(|m| m.head() == h)
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
            .filter(|m| m.is_varlike() || matches!(m, Ty::Inter(_)))
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
        enum Task {
            Visit(Ty, Pol, Mode),
            Handled(V, V, Pol, Mode, usize),
            Lower(V, V, Pol, Mode, usize, bool),
            Upper(V, usize, bool),
            Con(Head, super::lits::ValueSet, usize),
            Fn(Vec<Option<String>>),
            Union(usize, Mode),
            Inter(usize),
            Diff(usize),
            Both,
        }
        fn take(out: &mut Vec<Ty>, count: usize) -> Vec<Ty> {
            let start = out
                .len()
                .checked_sub(count)
                .expect("extrude result stack underflow");
            out.split_off(start)
        }
        fn schedule(tasks: &mut Vec<Task>, children: Vec<(Ty, Pol, Mode)>) {
            tasks.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|(t, p, m)| Task::Visit(t, p, m)),
            );
        }
        let mut tasks = vec![Task::Visit(t.clone(), pol, env)];
        let mut out = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Visit(t, pol, env) => {
                    let env = env.max(self.mode(&t));
                    self.observe_type(&t);
                    if self.level_of(&t) <= level {
                        out.push(t);
                        continue;
                    }
                    match t.into_parts() {
                        TyParts::Var(v) => {
                            if let Some(nv) = cache.get(&(v, pol, env)) {
                                out.push(Ty::Var(*nv));
                                continue;
                            }
                            let nv = self.fresh_at(level);
                            self.vars[nv.0 as usize].input = self.var(v).input;
                            self.vars[nv.0 as usize].row = self.var(v).row;
                            self.vars[nv.0 as usize].difference = self.var(v).difference;
                            cache.insert((v, pol, env), nv);
                            let handled = self.var(v).handled.clone();
                            tasks.push(Task::Handled(v, nv, pol, env, handled.len()));
                            schedule(
                                &mut tasks,
                                handled
                                    .into_iter()
                                    .map(|t| (t, Pol::Both, Mode::Value))
                                    .collect(),
                            );
                        }
                        TyParts::Con(h, values, args) => {
                            let n = args.len();
                            let variances = self.variances(h, n);
                            let rows = self.rows(h, n);
                            let children = args
                                .into_iter()
                                .zip(variances)
                                .zip(rows)
                                .map(|((t, v), row)| {
                                    (
                                        t,
                                        if env != Mode::Value {
                                            Pol::Both
                                        } else {
                                            pol.by(v)
                                        },
                                        row,
                                    )
                                })
                                .collect();
                            tasks.push(Task::Con(h, values, n));
                            schedule(&mut tasks, children);
                        }
                        TyParts::Fn(f) => {
                            let f = *f;
                            let mut labels = Vec::with_capacity(f.params.len());
                            let mut children = Vec::with_capacity(f.params.len() + 3);
                            for (label, t) in f.params {
                                labels.push(label);
                                children.push((t, pol.flip(), Mode::Value));
                            }
                            children.extend([
                                (f.ret, pol, Mode::Value),
                                (f.err, pol, Mode::Row),
                                (f.env, pol, Mode::Env),
                            ]);
                            tasks.push(Task::Fn(labels));
                            schedule(&mut tasks, children);
                        }
                        TyParts::Union(ms) => {
                            tasks.push(Task::Union(ms.len(), env));
                            schedule(&mut tasks, ms.into_iter().map(|t| (t, pol, env)).collect());
                        }
                        TyParts::Inter(ms) => {
                            tasks.push(Task::Inter(ms.len()));
                            schedule(&mut tasks, ms.into_iter().map(|t| (t, pol, env)).collect());
                        }
                        TyParts::Diff(a, hs) => {
                            tasks.push(Task::Diff(hs.len()));
                            schedule(
                                &mut tasks,
                                hs.into_iter().map(|t| (t, Pol::Both, env)).collect(),
                            );
                            tasks.push(Task::Visit(*a, pol, env));
                        }
                        TyParts::Both(a, b) => {
                            tasks.push(Task::Both);
                            tasks.push(Task::Visit(*b, Pol::Neg, env));
                            tasks.push(Task::Visit(*a, Pol::Pos, env));
                        }
                        TyParts::Never => out.push(Ty::Never),
                        TyParts::Top => out.push(Ty::Top),
                    }
                }
                Task::Handled(v, nv, pol, env, n) => {
                    self.var_mut(nv).handled = take(&mut out, n);
                    let lower = if pol != Pol::Neg {
                        self.var_mut(v).upper.push(Ty::Var(nv));
                        self.var_mut(nv).from.push(v);
                        self.var(v).lower.clone()
                    } else {
                        Vec::new()
                    };
                    tasks.push(Task::Lower(v, nv, pol, env, lower.len(), pol != Pol::Neg));
                    schedule(
                        &mut tasks,
                        lower.into_iter().map(|t| (t, Pol::Pos, env)).collect(),
                    );
                }
                Task::Lower(v, nv, pol, env, n, set) => {
                    let lower = take(&mut out, n);
                    if set {
                        self.set_lower(nv, lower);
                    }
                    let upper = if pol != Pol::Pos {
                        self.var_mut(v).lower.push(Ty::Var(nv));
                        self.var_mut(nv).into.push(v);
                        self.var(v)
                            .upper
                            .iter()
                            .filter(|b| **b != Ty::Var(nv))
                            .cloned()
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    };
                    tasks.push(Task::Upper(nv, upper.len(), pol != Pol::Pos));
                    schedule(
                        &mut tasks,
                        upper.into_iter().map(|t| (t, Pol::Neg, env)).collect(),
                    );
                }
                Task::Upper(nv, n, set) => {
                    let upper = take(&mut out, n);
                    if set {
                        self.set_upper(nv, upper);
                    }
                    out.push(Ty::Var(nv));
                }
                Task::Con(h, values, n) => {
                    let args = take(&mut out, n);
                    out.push(Ty::Con(h, values, args));
                }
                Task::Fn(labels) => {
                    let mut parts = take(&mut out, labels.len() + 3).into_iter();
                    let params = labels
                        .into_iter()
                        .map(|label| (label, parts.next().unwrap()))
                        .collect();
                    out.push(Ty::Fn(Box::new(FnTy {
                        params,
                        ret: parts.next().unwrap(),
                        err: parts.next().unwrap(),
                        env: parts.next().unwrap(),
                    })));
                }
                Task::Union(n, env) => {
                    let ms = take(&mut out, n);
                    out.push(if env == Mode::Value {
                        Ty::Union(ms)
                    } else {
                        self.union_mode(ms, env)
                    });
                }
                Task::Inter(n) => {
                    let ms = take(&mut out, n);
                    out.push(Ty::Inter(ms));
                }
                Task::Diff(n) => {
                    let mut parts = take(&mut out, n + 1).into_iter();
                    let a = parts.next().unwrap();
                    out.push(Ty::Diff(Box::new(a), parts.collect()));
                }
                Task::Both => {
                    let mut parts = take(&mut out, 2).into_iter();
                    out.push(Ty::Both(
                        Box::new(parts.next().unwrap()),
                        Box::new(parts.next().unwrap()),
                    ));
                }
            }
        }
        assert_eq!(out.len(), 1, "extrude must produce exactly one type");
        out.pop().unwrap()
    }

    /// Instantiates a type scheme: copies the variables deeper than `above`
    /// to new variables at the current level, with their bounds.
    pub fn freshen(&mut self, t: &Ty, above: u32, map: &mut StdHashMap<V, V>) -> Ty {
        // Solver-variable bounds form a graph. Insert the fresh variable before
        // scheduling its obligations so shared and cyclic bounds keep one
        // identity, just as in Simple-sub's freshenAbove.
        enum Task {
            Visit(Ty),
            Handled(V, V, usize),
            Bounds(V, usize, usize),
            Con(Head, super::lits::ValueSet, usize),
            Fn(Vec<Option<String>>),
            Union(usize, Mode),
            Inter(usize),
            Diff(usize),
            Both,
        }
        fn take(out: &mut Vec<Ty>, count: usize) -> Vec<Ty> {
            let start = out
                .len()
                .checked_sub(count)
                .expect("freshen result stack underflow");
            out.split_off(start)
        }
        fn schedule(tasks: &mut Vec<Task>, children: Vec<Ty>) {
            tasks.extend(children.into_iter().rev().map(Task::Visit));
        }
        if self.level_of(t) <= above {
            return t.clone();
        }
        let mut tasks = vec![Task::Visit(t.clone())];
        let mut out = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Visit(t) => {
                    if self.level_of(&t) <= above {
                        out.push(t);
                        continue;
                    }
                    let row = if matches!(t, Ty::Union(_)) {
                        self.mode(&t)
                    } else {
                        Mode::Value
                    };
                    match t.into_parts() {
                        TyParts::Var(v) if self.var(v).level > above => {
                            if let Some(nv) = map.get(&v) {
                                out.push(Ty::Var(*nv));
                                continue;
                            }
                            let nv = self.fresh_at(self.level);
                            map.insert(v, nv);
                            self.var_mut(nv).row = self.var(v).row;
                            self.var_mut(nv).difference = self.var(v).difference;
                            let handled = self.var(v).handled.clone();
                            tasks.push(Task::Handled(v, nv, handled.len()));
                            schedule(&mut tasks, handled);
                        }
                        TyParts::Var(v) => out.push(Ty::Var(v)),
                        TyParts::Never => out.push(Ty::Never),
                        TyParts::Top => out.push(Ty::Top),
                        TyParts::Con(h, values, args) => {
                            tasks.push(Task::Con(h, values, args.len()));
                            schedule(&mut tasks, args);
                        }
                        TyParts::Fn(f) => {
                            let f = *f;
                            let (labels, mut params): (Vec<_>, Vec<_>) =
                                f.params.into_iter().unzip();
                            params.extend([f.ret, f.err, f.env]);
                            tasks.push(Task::Fn(labels));
                            schedule(&mut tasks, params);
                        }
                        TyParts::Union(ms) => {
                            tasks.push(Task::Union(ms.len(), row));
                            schedule(&mut tasks, ms);
                        }
                        TyParts::Inter(ms) => {
                            tasks.push(Task::Inter(ms.len()));
                            schedule(&mut tasks, ms);
                        }
                        TyParts::Diff(a, ms) => {
                            tasks.push(Task::Diff(ms.len()));
                            schedule(&mut tasks, ms);
                            tasks.push(Task::Visit(*a));
                        }
                        TyParts::Both(a, b) => {
                            tasks.push(Task::Both);
                            tasks.push(Task::Visit(*b));
                            tasks.push(Task::Visit(*a));
                        }
                    }
                }
                Task::Handled(v, nv, n) => {
                    self.var_mut(nv).handled = take(&mut out, n);
                    let (lower, upper) = (self.var(v).lower.clone(), self.var(v).upper.clone());
                    tasks.push(Task::Bounds(nv, lower.len(), upper.len()));
                    schedule(&mut tasks, upper);
                    schedule(&mut tasks, lower);
                }
                Task::Bounds(nv, nl, nu) => {
                    let mut bounds = take(&mut out, nl + nu);
                    let upper = bounds.split_off(nl);
                    self.set_lower(nv, bounds);
                    self.set_upper(nv, upper);
                    out.push(Ty::Var(nv));
                }
                Task::Con(h, values, n) => {
                    let args = take(&mut out, n);
                    out.push(Ty::Con(h, values, args));
                }
                Task::Fn(labels) => {
                    let mut parts = take(&mut out, labels.len() + 3).into_iter();
                    let params = labels
                        .into_iter()
                        .map(|label| (label, parts.next().unwrap()))
                        .collect();
                    out.push(Ty::Fn(Box::new(FnTy {
                        params,
                        ret: parts.next().unwrap(),
                        err: parts.next().unwrap(),
                        env: parts.next().unwrap(),
                    })));
                }
                Task::Union(n, mode) => {
                    let ms = take(&mut out, n);
                    out.push(if mode == Mode::Value {
                        Ty::Union(ms)
                    } else {
                        self.union_mode(ms, mode)
                    });
                }
                Task::Inter(n) => {
                    let ms = take(&mut out, n);
                    out.push(Ty::Inter(ms));
                }
                Task::Diff(n) => {
                    let mut parts = take(&mut out, n + 1).into_iter();
                    let a = parts.next().unwrap();
                    out.push(Ty::Diff(Box::new(a), parts.collect()));
                }
                Task::Both => {
                    let mut parts = take(&mut out, 2).into_iter();
                    out.push(Ty::Both(
                        Box::new(parts.next().unwrap()),
                        Box::new(parts.next().unwrap()),
                    ));
                }
            }
        }
        assert_eq!(out.len(), 1, "freshen must produce exactly one type");
        out.pop().unwrap()
    }

    /// `t` with each type directly inside it changed by `f`, which is given
    /// the polarity of the place.
    #[cfg(test)]
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
        self.union_mode(ms, Mode::Value)
    }

    /// The union of environments (§9.2): two forms of one type constructor
    /// must have equal arguments, whatever their variance.
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
        let t = self.union_in(ms, mode != Mode::Value);
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
            let row = self.fresh();
            for m in ms {
                self.constrain_in(m, &row, mode);
            }
        }
        t
    }

    fn union_in(&mut self, ms: Vec<Ty>, env: bool) -> Ty {
        let mut flat = Vec::new();
        flatten_union(&ms, &mut flat);
        if !env {
            flat = join_literals(flat);
        }
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
                    if out[i].values() != m.values() {
                        out.push(m);
                        continue;
                    }
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
        self.normalize_mode(Ty::Inter(ms), Mode::Value)
    }

    fn inter_in(&mut self, ms: Vec<Ty>, env: bool) -> Ty {
        let mut flat = Vec::new();
        for m in ms {
            match m.into_parts() {
                TyParts::Inter(inner) => flat.extend(inner),
                m => flat.push(m.into()),
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
            let TyParts::Union(xs) = flat.remove(i).into_parts() else {
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
    /// and by intersection when not. In an environment (`env`), the
    /// arguments must be equal (§9.2).
    fn merge(&mut self, a: Ty, b: Ty, union: bool, env: bool) -> Ty {
        match (a.into_parts(), b.into_parts()) {
            (TyParts::Con(h, values, xs), TyParts::Con(_, other, ys)) if xs.len() == ys.len() => {
                if env {
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
            (TyParts::Fn(f), TyParts::Fn(g)) if f.params.len() == g.params.len() => {
                let f = *f;
                let g = *g;
                let params = f
                    .params
                    .into_iter()
                    .zip(g.params)
                    .map(|((l1, p), (l2, q))| {
                        let label = if l1 == l2 { l1 } else { None };
                        (
                            label,
                            self.merge_arg(p, q, Variance::Contra, union, Mode::Value),
                        )
                    })
                    .collect();
                // The environments of two function types are put together
                // as environments (§4.4, §9.2).
                let env = if union {
                    self.union_mode(vec![f.env, g.env], Mode::Env)
                } else {
                    self.normalize_mode(Ty::Inter(vec![f.env, g.env]), Mode::Env)
                };
                Ty::Fn(Box::new(FnTy {
                    params,
                    ret: self.merge_arg(f.ret, g.ret, Variance::Co, union, Mode::Value),
                    err: self.merge_arg(f.err, g.err, Variance::Co, union, Mode::Row),
                    env,
                }))
            }
            (a, b) => {
                let (a, b) = (Ty::from(a), Ty::from(b));
                self.fail(FailKind::Arity, &a, &b);
                a
            }
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

    /// `M - T` for an environment `M` (§4.8 rule 9): the member of `M` that
    /// is taken out must be equal to its member of `T` (§9.2).
    pub fn env_diff(&mut self, a: Ty, h: Vec<Ty>) -> Ty {
        self.diff_mode(a, h, Mode::Env)
    }

    fn diff_mode(&mut self, a: Ty, h: Vec<Ty>, mode: Mode) -> Ty {
        let mode = mode.max(self.mode(&a));
        let env = mode != Mode::Value;
        let mut hs = Vec::new();
        flatten_union(&h, &mut hs);
        hs.retain(|m| !matches!(m, Ty::Never));
        if hs.is_empty() {
            return a;
        }
        if matches!(a, Ty::Con(..)) && a.scalar().is_some() && mode != Mode::Env {
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
        } else if matches!(a, Ty::Con(..) | Ty::Fn(_)) {
            match hs.iter().find(|m| m.head() == a.head()) {
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
            }
        } else {
            match a.into_parts() {
                TyParts::Never => Ty::Never,
                TyParts::Union(ms) => {
                    let parts = ms
                        .into_iter()
                        .map(|m| self.diff_mode(m, hs.clone(), mode))
                        .collect();
                    self.union_in(parts, env)
                }
                TyParts::Diff(b, mut h0) => {
                    self.diff_bound_mode(&b, &hs, mode);
                    for m in hs {
                        if !h0.contains(&m) {
                            h0.push(m);
                        }
                    }
                    Ty::Diff(b, h0)
                }
                a => {
                    let a = Ty::from(a);
                    self.diff_bound_mode(&a, &hs, mode);
                    Ty::Diff(Box::new(a), hs)
                }
            }
        }
    }

    /// Normalize closed or open type algebra without losing the row
    /// mode of constructor arguments or function effects.
    pub fn normalize_mode(&mut self, t: Ty, mode: Mode) -> Ty {
        enum Task {
            Visit(Ty, Mode),
            Con(Head, super::lits::ValueSet, usize),
            Fn(Vec<Option<String>>),
            Union(usize, Mode),
            Inter(usize, Mode),
            Diff(usize, Mode),
            Both,
        }
        let mut tasks = vec![Task::Visit(t, mode)];
        let mut results = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Visit(t, mode) => {
                    let mode = mode.max(self.mode(&t));
                    match t.into_parts() {
                        TyParts::Con(h, values, args) => {
                            let modes = self.rows(h, args.len());
                            tasks.push(Task::Con(h, values, args.len()));
                            let children: Vec<_> = args.into_iter().zip(modes).collect();
                            tasks
                                .extend(children.into_iter().rev().map(|(t, m)| Task::Visit(t, m)));
                        }
                        TyParts::Fn(f) => {
                            let FnTy {
                                params,
                                ret,
                                err,
                                env,
                            } = *f;
                            let mut names = Vec::with_capacity(params.len());
                            let mut children = Vec::with_capacity(params.len() + 3);
                            for (n, t) in params {
                                names.push(n);
                                children.push((t, Mode::Value));
                            }
                            children.extend([
                                (ret, Mode::Value),
                                (err, Mode::Row),
                                (env, Mode::Env),
                            ]);
                            tasks.push(Task::Fn(names));
                            tasks
                                .extend(children.into_iter().rev().map(|(t, m)| Task::Visit(t, m)));
                        }
                        TyParts::Union(ms) => {
                            tasks.push(Task::Union(ms.len(), mode));
                            tasks.extend(ms.into_iter().rev().map(|t| Task::Visit(t, mode)));
                        }
                        TyParts::Inter(ms) => {
                            tasks.push(Task::Inter(ms.len(), mode));
                            tasks.extend(ms.into_iter().rev().map(|t| Task::Visit(t, mode)));
                        }
                        TyParts::Diff(t, hs) => {
                            tasks.push(Task::Diff(hs.len(), mode));
                            tasks.extend(hs.into_iter().rev().map(|t| Task::Visit(t, mode)));
                            tasks.push(Task::Visit(*t, mode));
                        }
                        TyParts::Both(a, b) => {
                            tasks.push(Task::Both);
                            tasks.push(Task::Visit(*b, mode));
                            tasks.push(Task::Visit(*a, mode));
                        }
                        t => results.push(t.into()),
                    }
                }
                Task::Con(h, values, n) => {
                    let args = results.split_off(results.len() - n);
                    results.push(Ty::Con(h, values, args));
                }
                Task::Fn(names) => {
                    let mut children = results
                        .split_off(results.len() - names.len() - 3)
                        .into_iter();
                    let params = names
                        .into_iter()
                        .map(|n| (n, children.next().expect("normalized parameter")))
                        .collect();
                    results.push(Ty::Fn(Box::new(FnTy {
                        params,
                        ret: children.next().expect("normalized return"),
                        err: children.next().expect("normalized error"),
                        env: children.next().expect("normalized environment"),
                    })));
                }
                Task::Union(n, mode) => {
                    let ms = results.split_off(results.len() - n);
                    results.push(self.union_mode(ms, mode));
                }
                Task::Inter(n, mode) => {
                    let ms = results.split_off(results.len() - n);
                    let scalar = ms.iter().any(|t| t.scalar().is_some());
                    results.push(
                        self.inter_in(ms, mode != Mode::Value && !(mode == Mode::Row && scalar)),
                    );
                }
                Task::Diff(n, mode) => {
                    let mut values = results.split_off(results.len() - n - 1).into_iter();
                    let t = values.next().expect("normalized difference subject");
                    results.push(self.diff_mode(t, values.collect(), mode));
                }
                Task::Both => {
                    let b = results.pop().expect("normalized right side");
                    let a = results.pop().expect("normalized left side");
                    results.push(Ty::Both(Box::new(a), Box::new(b)));
                }
            }
        }
        assert_eq!(results.len(), 1, "normalization result stack");
        results.pop().unwrap()
    }

    /// `t` with each `Top` in it changed to a new variable at `level`.
    pub fn no_top(&mut self, t: &Ty, level: u32) -> Ty {
        let mut result = t.clone();
        let mut pending = vec![&mut result];
        while let Some(t) = pending.pop() {
            match t {
                Ty::Top => *t = Ty::Var(self.fresh_at(level)),
                Ty::Con(_, _, args) | Ty::Union(args) | Ty::Inter(args) => {
                    pending.extend(args.iter_mut().rev());
                }
                Ty::Fn(f) => {
                    pending.push(&mut f.env);
                    pending.push(&mut f.err);
                    pending.push(&mut f.ret);
                    pending.extend(f.params.iter_mut().rev().map(|(_, p)| p));
                }
                Ty::Diff(t, hs) => {
                    pending.extend(hs.iter_mut().rev());
                    pending.push(t);
                }
                Ty::Both(a, b) => {
                    pending.push(b);
                    pending.push(a);
                }
                Ty::Var(_) | Ty::Never => {}
            }
        }
        result
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
            if !done.insert(v) {
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
                let before = self.var(v).upper.len();
                self.var_mut(v).upper.retain(|b| {
                    !matches!(b, Ty::Union(ms) if ms.iter().any(|m| matches!(m, Ty::Var(u) if vars.contains(u))))
                });
                changed |= self.var(v).upper.len() != before;
            }
        }
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
    collect_vars(s, t, pos, con, false, out);
}

/// `bound_vars` for an environment: the arguments of its members are on
/// both sides.
fn env_vars(s: &Solver, t: &Ty, pos: bool, con: bool, out: &mut Vec<(V, bool, bool)>) {
    collect_vars(s, t, pos, con, true, out);
}

fn collect_vars(
    s: &Solver,
    t: &Ty,
    pos: bool,
    con: bool,
    env: bool,
    out: &mut Vec<(V, bool, bool)>,
) {
    let mut pending = vec![(t, pos, con, env)];
    while let Some((t, pos, con, env)) = pending.pop() {
        if env {
            match t {
                Ty::Con(_, _, args) => {
                    for a in args.iter().rev() {
                        pending.push((a, false, true, false));
                        pending.push((a, true, true, false));
                    }
                }
                Ty::Union(ms) | Ty::Inter(ms) => {
                    pending.extend(ms.iter().rev().map(|a| (a, pos, con, true)))
                }
                Ty::Diff(a, hs) => {
                    pending.extend(hs.iter().rev().map(|h| (h, pos, true, false)));
                    pending.push((a, pos, con, true));
                }
                _ => pending.push((t, pos, con, false)),
            }
            continue;
        }
        match t {
            Ty::Var(v) => out.push((*v, pos, con)),
            Ty::Con(h, _, args) => {
                let children: Vec<_> = args
                    .iter()
                    .zip(s.variances(*h, args.len()))
                    .zip(s.rows(*h, args.len()))
                    .collect();
                for ((a, variance), row) in children.into_iter().rev() {
                    if row != Mode::Value {
                        pending.push((a, pos, true, true));
                    } else {
                        match variance {
                            Variance::Co | Variance::Bi => pending.push((a, pos, true, false)),
                            Variance::Contra => pending.push((a, !pos, true, false)),
                            Variance::Inv => {
                                pending.push((a, !pos, true, false));
                                pending.push((a, pos, true, false));
                            }
                        }
                    }
                }
            }
            Ty::Fn(f) => {
                pending.push((&f.env, pos, true, true));
                pending.push((&f.err, pos, true, true));
                pending.push((&f.ret, pos, true, false));
                pending.extend(f.params.iter().rev().map(|(_, p)| (p, !pos, true, false)));
            }
            Ty::Union(ms) | Ty::Inter(ms) => {
                pending.extend(ms.iter().rev().map(|a| (a, pos, con, false)))
            }
            Ty::Diff(a, hs) => {
                pending.extend(hs.iter().rev().map(|h| (h, pos, con, false)));
                pending.push((a, pos, con, false));
            }
            Ty::Both(a, b) => {
                pending.push((b, false, con, false));
                pending.push((a, true, con, false));
            }
            Ty::Never | Ty::Top => {}
        }
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
    let mut pending: Vec<_> = ms.iter().rev().collect();
    while let Some(m) = pending.pop() {
        match m {
            Ty::Union(inner) => pending.extend(inner.iter().rev()),
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
        BuiltinType::List => &[Co],
        BuiltinType::Boundary => &[Inv, Inv, Inv],
        BuiltinType::Continuation => &[Contra, Co, Co, Co],
        _ => &[],
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
    fn service_equality_does_not_widen_literal_arguments() {
        // §5.1 rule 1 and §10: service forms are equal, not covariant.
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
            let Ty::Var(v) = &f.params[0].1 else {
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
mod deep_tree_tests {
    use super::*;

    fn recursive_vars(
        s: &Solver,
        t: &Ty,
        pos: bool,
        con: bool,
        env: bool,
        out: &mut Vec<(V, bool, bool)>,
    ) {
        if env {
            match t {
                Ty::Con(_, _, args) => {
                    for a in args {
                        recursive_vars(s, a, true, true, false, out);
                        recursive_vars(s, a, false, true, false, out);
                    }
                }
                Ty::Union(ms) | Ty::Inter(ms) => {
                    for a in ms {
                        recursive_vars(s, a, pos, con, true, out);
                    }
                }
                Ty::Diff(a, hs) => {
                    recursive_vars(s, a, pos, con, true, out);
                    for h in hs {
                        recursive_vars(s, h, pos, true, false, out);
                    }
                }
                _ => recursive_vars(s, t, pos, con, false, out),
            }
            return;
        }
        match t {
            Ty::Var(v) => out.push((*v, pos, con)),
            Ty::Con(h, _, args) => {
                for ((a, variance), row) in args
                    .iter()
                    .zip(s.variances(*h, args.len()))
                    .zip(s.rows(*h, args.len()))
                {
                    if row != Mode::Value {
                        recursive_vars(s, a, pos, true, true, out);
                        continue;
                    }
                    match variance {
                        Variance::Co | Variance::Bi => recursive_vars(s, a, pos, true, false, out),
                        Variance::Contra => recursive_vars(s, a, !pos, true, false, out),
                        Variance::Inv => {
                            recursive_vars(s, a, pos, true, false, out);
                            recursive_vars(s, a, !pos, true, false, out);
                        }
                    }
                }
            }
            Ty::Fn(f) => {
                for (_, p) in &f.params {
                    recursive_vars(s, p, !pos, true, false, out);
                }
                recursive_vars(s, &f.ret, pos, true, false, out);
                recursive_vars(s, &f.err, pos, true, true, out);
                recursive_vars(s, &f.env, pos, true, true, out);
            }
            Ty::Union(ms) | Ty::Inter(ms) => {
                for a in ms {
                    recursive_vars(s, a, pos, con, false, out);
                }
            }
            Ty::Diff(a, hs) => {
                recursive_vars(s, a, pos, con, false, out);
                for h in hs {
                    recursive_vars(s, h, pos, con, false, out);
                }
            }
            Ty::Both(a, b) => {
                recursive_vars(s, a, true, con, false, out);
                recursive_vars(s, b, false, con, false, out);
            }
            Ty::Never | Ty::Top => {}
        }
    }

    #[test]
    fn iterative_bound_walk_preserves_recursive_order_and_polarity() {
        let s = Solver::new(StdHashMap::new(), Vec::new());
        let boundary = Ty::named(
            TypeRef::Builtin(BuiltinType::Boundary),
            vec![Ty::Var(V(0)), Ty::Var(V(1)), Ty::Var(V(2))],
        );
        let t = Ty::Fn(Box::new(FnTy {
            params: vec![
                (None, boundary.clone()),
                (
                    None,
                    Ty::Both(Box::new(Ty::Var(V(3))), Box::new(Ty::Var(V(4)))),
                ),
            ],
            ret: Ty::Diff(Box::new(Ty::Var(V(5))), vec![Ty::Var(V(6))]),
            err: Ty::Union(vec![boundary.clone(), Ty::Var(V(7))]),
            env: Ty::Diff(Box::new(boundary), vec![Ty::Var(V(8))]),
        }));
        for pos in [false, true] {
            for con in [false, true] {
                for env in [false, true] {
                    let (mut actual, mut expected) = (Vec::new(), Vec::new());
                    collect_vars(&s, &t, pos, con, env, &mut actual);
                    recursive_vars(&s, &t, pos, con, env, &mut expected);
                    assert_eq!(actual, expected);
                }
            }
        }
    }

    fn recursive_extrude(
        s: &mut Solver,
        t: &Ty,
        pol: Pol,
        level: u32,
        env: Mode,
        cache: &mut HashMap<(V, Pol, Mode), V>,
    ) -> Ty {
        let env = env.max(s.mode(t));
        s.observe_type(t);
        if s.level_of(t) <= level {
            return t.clone();
        }
        match t {
            Ty::Var(v) => {
                if let Some(nv) = cache.get(&(*v, pol, env)) {
                    return Ty::Var(*nv);
                }
                let nv = s.fresh_at(level);
                s.vars[nv.0 as usize].input = s.var(*v).input;
                s.vars[nv.0 as usize].row = s.var(*v).row;
                s.vars[nv.0 as usize].difference = s.var(*v).difference;
                cache.insert((*v, pol, env), nv);
                let handled = s
                    .var(*v)
                    .handled
                    .clone()
                    .iter()
                    .map(|h| recursive_extrude(s, h, Pol::Both, level, Mode::Value, cache))
                    .collect();
                s.var_mut(nv).handled = handled;
                if pol != Pol::Neg {
                    s.var_mut(*v).upper.push(Ty::Var(nv));
                    s.var_mut(nv).from.push(*v);
                    let lower = s
                        .var(*v)
                        .lower
                        .clone()
                        .iter()
                        .map(|b| recursive_extrude(s, b, Pol::Pos, level, env, cache))
                        .collect();
                    s.set_lower(nv, lower);
                }
                if pol != Pol::Pos {
                    s.var_mut(*v).lower.push(Ty::Var(nv));
                    s.var_mut(nv).into.push(*v);
                    let upper = s
                        .var(*v)
                        .upper
                        .clone()
                        .iter()
                        .filter(|b| **b != Ty::Var(nv))
                        .map(|b| recursive_extrude(s, b, Pol::Neg, level, env, cache))
                        .collect();
                    s.set_upper(nv, upper);
                }
                Ty::Var(nv)
            }
            Ty::Con(h, values, args) => {
                let variance = s.variances(*h, args.len());
                let rows = s.rows(*h, args.len());
                Ty::Con(
                    *h,
                    values.clone(),
                    args.iter()
                        .zip(variance)
                        .zip(rows)
                        .map(|((a, v), row)| {
                            recursive_extrude(
                                s,
                                a,
                                if env != Mode::Value {
                                    Pol::Both
                                } else {
                                    pol.by(v)
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
                params: f
                    .params
                    .iter()
                    .map(|(label, p)| {
                        (
                            label.clone(),
                            recursive_extrude(s, p, pol.flip(), level, Mode::Value, cache),
                        )
                    })
                    .collect(),
                ret: recursive_extrude(s, &f.ret, pol, level, Mode::Value, cache),
                err: recursive_extrude(s, &f.err, pol, level, Mode::Row, cache),
                env: recursive_extrude(s, &f.env, pol, level, Mode::Env, cache),
            })),
            Ty::Union(ms) if env != Mode::Value || s.is_row(t) => {
                let mode = env.max(s.mode(t));
                let ms = ms
                    .iter()
                    .map(|m| recursive_extrude(s, m, pol, level, mode, cache))
                    .collect();
                s.union_mode(ms, mode)
            }
            _ => s.map_children(t, pol, &mut |s, x, p| {
                recursive_extrude(s, x, p, level, env, cache)
            }),
        }
    }

    #[test]
    fn iterative_extrusion_matches_recursive_rule_with_shared_rows() {
        let mut base = Solver::new(StdHashMap::new(), Vec::new());
        let a = base.fresh_at(3);
        let b = base.fresh_at(3);
        let weak = base.fresh_at(0);
        base.var_mut(a).row = Mode::Env;
        base.var_mut(a).difference = true;
        base.var_mut(a).handled.push(Ty::Var(b));
        base.var_mut(b).handled.push(Ty::Var(a));
        base.var_mut(a)
            .lower
            .push(Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new()));
        base.var_mut(b).upper.push(Ty::Var(weak));
        let t = Ty::Fn(Box::new(FnTy {
            params: vec![(Some("x".into()), Ty::Var(a)), (None, Ty::Var(weak))],
            ret: Ty::tuple(vec![Ty::Var(a), Ty::Var(b)]),
            err: Ty::Never,
            env: Ty::Union(vec![Ty::Var(a), Ty::Var(b)]),
        }));
        for pol in [Pol::Pos, Pol::Neg, Pol::Both] {
            let (mut actual, mut reference) = (base.clone(), base.clone());
            let (mut am, mut rm) = (HashMap::default(), HashMap::default());
            let a = actual.extrude(&t, pol, 0, Mode::Value, &mut am);
            let b = recursive_extrude(&mut reference, &t, pol, 0, Mode::Value, &mut rm);
            assert_eq!(a, b);
            assert_eq!(am, rm);
            assert_eq!(actual, reference);
        }
    }

    // The previous recursive rule is an independent oracle on bounded trees;
    // compare solver state as well as the returned tree and variable map.
    fn recursive_freshen(s: &mut Solver, t: &Ty, above: u32, map: &mut StdHashMap<V, V>) -> Ty {
        if s.level_of(t) <= above {
            return t.clone();
        }
        match t {
            Ty::Var(v) => {
                if let Some(nv) = map.get(v) {
                    return Ty::Var(*nv);
                }
                let nv = s.fresh_at(s.level);
                map.insert(*v, nv);
                s.var_mut(nv).row = s.var(*v).row;
                s.var_mut(nv).difference = s.var(*v).difference;
                let handled = s
                    .var(*v)
                    .handled
                    .clone()
                    .iter()
                    .map(|h| recursive_freshen(s, h, above, map))
                    .collect();
                s.var_mut(nv).handled = handled;
                let (lower, upper) = (s.var(*v).lower.clone(), s.var(*v).upper.clone());
                let lower = lower
                    .iter()
                    .map(|b| recursive_freshen(s, b, above, map))
                    .collect();
                let upper = upper
                    .iter()
                    .map(|b| recursive_freshen(s, b, above, map))
                    .collect();
                s.set_lower(nv, lower);
                s.set_upper(nv, upper);
                Ty::Var(nv)
            }
            Ty::Union(ms) if s.is_row(t) => {
                let mode = s.mode(t);
                let ms = ms
                    .iter()
                    .map(|m| recursive_freshen(s, m, above, map))
                    .collect();
                s.union_mode(ms, mode)
            }
            _ => s.map_children(t, Pol::Pos, &mut |s, x, _| {
                recursive_freshen(s, x, above, map)
            }),
        }
    }

    #[test]
    fn iterative_freshening_matches_recursive_rule_on_shared_row_obligations() {
        let mut base = Solver::new(StdHashMap::new(), Vec::new());
        let a = base.fresh_at(3);
        let b = base.fresh_at(3);
        let weak = base.fresh_at(0);
        base.var_mut(a).row = Mode::Env;
        base.var_mut(a).difference = true;
        base.var_mut(a).handled.push(Ty::Var(b));
        base.var_mut(b).handled.push(Ty::Var(a));
        let int = Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new());
        base.var_mut(a).lower.push(int);
        base.var_mut(b).upper.push(Ty::Var(weak));
        base.level = 2;
        let t = Ty::Fn(Box::new(FnTy {
            params: vec![(Some("x".into()), Ty::Var(a)), (None, Ty::Var(weak))],
            ret: Ty::tuple(vec![Ty::Var(a), Ty::Var(b)]),
            err: Ty::Never,
            env: Ty::Union(vec![Ty::Var(a), Ty::Var(b)]),
        }));
        let (mut actual, mut reference) = (base.clone(), base);
        let (mut am, mut rm) = (StdHashMap::new(), StdHashMap::new());
        let a = actual.freshen(&t, 0, &mut am);
        let b = recursive_freshen(&mut reference, &t, 0, &mut rm);
        assert_eq!(a, b);
        assert_eq!(am, rm);
        assert_eq!(actual, reference);
    }

    #[test]
    fn extrusion_of_deep_shared_type_preserves_bounds() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut s = Solver::new(StdHashMap::new(), Vec::new());
                let v = s.fresh_at(4);
                let int = Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new());
                s.constrain(&int, &Ty::Var(v));
                let mut t = Ty::tuple(vec![Ty::Var(v), Ty::Var(v)]);
                for _ in 0..10_000 {
                    t = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![t]);
                }
                let mut cache = HashMap::default();
                let result = s.extrude(&t, Pol::Pos, 0, Mode::Value, &mut cache);
                let new = cache[&(v, Pol::Pos, Mode::Value)];
                assert_ne!(v, new);
                assert!(s.var(new).lower.contains(&int));
                assert!(s.var(new).from.contains(&v));
                let mut end = &result;
                for _ in 0..10_000 {
                    let Ty::Con(_, _, args) = end else {
                        panic!("expected list")
                    };
                    end = &args[0];
                }
                let Ty::Con(Head::Tuple(2), _, args) = end else {
                    panic!("expected tuple")
                };
                assert!(args == &[Ty::Var(new), Ty::Var(new)]);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn nested_constructor_function_and_row_constraints_use_no_call_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                fn deep(mut t: Ty, function: bool) -> Ty {
                    for _ in 0..10_000 {
                        t = if function {
                            Ty::Fn(Box::new(FnTy {
                                params: Vec::new(),
                                ret: t,
                                err: Ty::Never,
                                env: Ty::Never,
                            }))
                        } else {
                            Ty::named(TypeRef::Builtin(BuiltinType::List), vec![t])
                        };
                    }
                    t
                }
                let int = || Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new());
                let string = || Ty::named(TypeRef::Builtin(BuiltinType::String), Vec::new());
                for function in [false, true] {
                    let mut s = Solver::new(StdHashMap::new(), Vec::new());
                    let a = deep(int(), function);
                    let b = deep(string(), function);
                    s.constrain(&a, &b);
                    assert_eq!(s.failures.len(), 1);
                    assert_eq!(s.failures[0].kind, FailKind::Mismatch);
                    drop((a, b, s));
                }
                let mut s = Solver::new(StdHashMap::new(), Vec::new());
                let a = deep(int(), false);
                let b = deep(string(), false);
                s.constrain_env(&a, &b);
                assert_eq!(s.failures.len(), 1);
                assert_eq!(s.failures[0].kind, FailKind::EnvForms);
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn deep_bound_and_union_walks_are_iterative() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut s = Solver::new(StdHashMap::new(), Vec::new());
                let v = s.fresh_at(0);
                let mut t = Ty::Var(v);
                for _ in 0..10_000 {
                    t = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![t]);
                }
                let mut vars = Vec::new();
                bound_vars(&s, &t, true, false, &mut vars);
                assert_eq!(vars, vec![(v, true, true)]);
                let mut u = Ty::Var(v);
                for _ in 0..10_000 {
                    u = Ty::Union(vec![u]);
                }
                let mut flat = Vec::new();
                flatten_union(&[u], &mut flat);
                assert!(flat == vec![Ty::Var(v)]);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn deep_normalization_preserves_type() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut s = Solver::new(StdHashMap::new(), Vec::new());
                let mut t = Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new());
                for i in 0..10_000 {
                    t = if i % 2 == 0 {
                        Ty::named(TypeRef::Builtin(BuiltinType::List), vec![t])
                    } else {
                        Ty::Fn(Box::new(FnTy {
                            params: Vec::new(),
                            ret: t,
                            err: Ty::Never,
                            env: Ty::Never,
                        }))
                    };
                }
                let expected = t.clone();
                assert_eq!(s.normalize_mode(t, Mode::Value), expected);
                let mut top = Ty::Top;
                for _ in 0..10_000 {
                    top = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![top]);
                }
                let replaced = s.no_top(&top, 0);
                let mut end = &replaced;
                for _ in 0..10_000 {
                    let Ty::Con(_, _, args) = end else {
                        panic!("expected list")
                    };
                    end = &args[0];
                }
                assert!(matches!(end, Ty::Var(_)));
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn same_form_handles_deep_mismatch_on_small_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let s = Solver::new(StdHashMap::new(), Vec::new());
                let mut a = Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new());
                let mut b = Ty::named(TypeRef::Builtin(BuiltinType::String), Vec::new());
                for _ in 0..10_000 {
                    a = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![a]);
                    b = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![b]);
                }
                assert!(!s.same_form(&a, &b));
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
