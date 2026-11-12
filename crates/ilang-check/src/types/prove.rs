//! Read-only subtype proofs for interface comparison. A type variable is
//! rigid: it is only what its retained bounds in the solver say. No bound is
//! added to the solver. A question met again while it is being answered is
//! assumed to hold (recursive types are coinductive).

use super::{
    Head, Ty, V, Variance,
    solve::{Mode, Solver},
};
use rustc_hash::FxHashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Proven,
    Refuted,
    /// The proof needs something the bounds do not say.
    Unknown,
    ResourceLimit,
}

/// A budget of proof steps shared by the questions of one comparison.
pub(crate) struct Demand {
    left: usize,
}

impl Demand {
    pub const LIMIT: usize = 65_536;

    pub fn new(limit: usize) -> Self {
        Demand { left: limit }
    }

    pub fn charge(&mut self) -> Result<(), ()> {
        if self.left == 0 {
            return Err(());
        }
        self.left -= 1;
        Ok(())
    }
}

pub(crate) fn prove_mode(
    solver: &Solver,
    l: &Ty,
    r: &Ty,
    mode: Mode,
    demand: &mut Demand,
) -> Verdict {
    let mut p = Prover {
        s: solver,
        active: FxHashSet::default(),
        demand,
        vars: false,
    };
    match p.sub(l, r, mode) {
        Ok(true) => Verdict::Proven,
        Ok(false) if p.vars => Verdict::Unknown,
        Ok(false) => Verdict::Refuted,
        Err(()) => Verdict::ResourceLimit,
    }
}

pub(crate) fn prove(solver: &Solver, l: &Ty, r: &Ty, demand: &mut Demand) -> Verdict {
    prove_mode(solver, l, r, Mode::Value, demand)
}

struct Prover<'a> {
    s: &'a Solver,
    active: FxHashSet<(Ty, Ty, Mode)>,
    demand: &'a mut Demand,
    /// A failed step involved a type variable.
    vars: bool,
}

fn flatten(t: &Ty, union: bool, out: &mut Vec<Ty>) {
    match t {
        Ty::Union(ms) if union => ms.iter().for_each(|m| flatten(m, union, out)),
        Ty::Inter(ms) if !union => ms.iter().for_each(|m| flatten(m, union, out)),
        t => out.push(t.clone()),
    }
}

impl Prover<'_> {
    fn sub(&mut self, l: &Ty, r: &Ty, mode: Mode) -> Result<bool, ()> {
        if l == r || matches!(l, Ty::Never) || matches!(r, Ty::Top) {
            return Ok(true);
        }
        let key = (l.clone(), r.clone(), mode);
        if self.active.contains(&key) {
            return Ok(true);
        }
        self.demand.charge()?;
        self.active.insert(key.clone());
        let result = self.step(l, r, mode);
        self.active.remove(&key);
        if matches!(result, Ok(false)) && (matches!(l, Ty::Var(_)) || matches!(r, Ty::Var(_))) {
            self.vars = true;
        }
        result
    }

    fn any(&mut self, ls: &[Ty], r: &Ty, mode: Mode) -> Result<bool, ()> {
        for l in ls {
            if self.sub(l, r, mode)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn step(&mut self, l: &Ty, r: &Ty, mode: Mode) -> Result<bool, ()> {
        match (l, r) {
            (Ty::Both(a, _), _) => self.sub(a, r, mode),
            (_, Ty::Both(_, b)) => self.sub(l, b, mode),
            (Ty::Union(_), _) => {
                let mut ms = Vec::new();
                flatten(l, true, &mut ms);
                for m in &ms {
                    if !self.sub(m, r, mode)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (_, Ty::Inter(_)) => {
                let mut ms = Vec::new();
                flatten(r, false, &mut ms);
                for m in &ms {
                    if !self.sub(l, m, mode)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (Ty::Var(u), Ty::Var(w)) if self.reaches(*u, *w) => Ok(true),
            (Ty::Var(v), _) => {
                // `v <: r` when an upper bound of `v` is in `r`, or when `v`
                // is a member of a union `r`.
                if let Ty::Union(_) = r {
                    let mut ms = Vec::new();
                    flatten(r, true, &mut ms);
                    if ms.contains(l) {
                        return Ok(true);
                    }
                }
                let upper = self.uppers(*v);
                if self.any(&upper, r, mode)? {
                    return Ok(true);
                }
                if let Ty::Var(w) = r {
                    let lower = self.lowers(*w);
                    for b in &lower {
                        if self.sub(l, b, mode)? {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            }
            (_, Ty::Var(w)) => {
                let lower = self.lowers(*w);
                for b in &lower {
                    if self.sub(l, b, mode)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            (Ty::Inter(_), _) => {
                let mut ms = Vec::new();
                flatten(l, false, &mut ms);
                self.any(&ms, r, mode)
            }
            // `a - H1 <: b - H2` when `a <: b` and every member of `H2` is
            // taken out by `H1`.
            (Ty::Diff(a, h1), Ty::Diff(b, h2))
                if h2.iter().all(|m| h1.iter().any(|n| n.head() == m.head())) =>
            {
                self.sub(a, b, mode)
            }
            (Ty::Diff(a, _), _) => self.sub(a, r, mode),
            (_, Ty::Diff(a, hs)) => {
                let h = l.head();
                let excluded = hs.iter().any(|m| {
                    let mut ms = Vec::new();
                    flatten(m, true, &mut ms);
                    ms.iter().any(|m| {
                        m.head().is_some()
                            && m.head() == h
                            && !(m.scalar().is_some() && m.values().disjoint(l.values()))
                    })
                });
                if excluded {
                    return Ok(false);
                }
                self.sub(l, a, mode)
            }
            (_, Ty::Union(_)) => {
                let mut ms = Vec::new();
                flatten(r, true, &mut ms);
                if l.scalar().is_some() {
                    let sets: Vec<_> = ms
                        .iter()
                        .filter(|m| m.head() == l.head())
                        .map(|m| m.values().clone())
                        .collect();
                    if !sets.is_empty()
                        && l.values().subset(&super::lits::ValueSet::join_all(&sets))
                    {
                        return Ok(true);
                    }
                }
                self.any_member(l, &ms, mode)
            }
            (Ty::Con(h1, s1, a1), Ty::Con(h2, s2, a2)) if h1 == h2 && a1.len() == a2.len() => {
                if (mode == Mode::Env && s1 != s2) || (mode != Mode::Env && !s1.subset(s2)) {
                    return Ok(false);
                }
                if mode != Mode::Value {
                    for (x, y) in a1.iter().zip(a2) {
                        if !(self.sub(x, y, Mode::Value)? && self.sub(y, x, Mode::Value)?) {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
                let vs = self.s.variances(*h1, a1.len());
                let rows = self.s.rows(*h1, a1.len());
                for (((x, y), v), row) in a1.iter().zip(a2).zip(vs).zip(rows) {
                    let ok = match v {
                        Variance::Co => self.sub(x, y, row)?,
                        Variance::Contra => self.sub(y, x, row)?,
                        Variance::Inv => self.sub(x, y, row)? && self.sub(y, x, row)?,
                        Variance::Bi => true,
                    };
                    if !ok {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (Ty::Fn(f), Ty::Fn(g)) => Ok(self.sub(&g.param, &f.param, Mode::Value)?
                && self.sub(&f.ret, &g.ret, Mode::Value)?
                && self.sub(&f.err, &g.err, Mode::Row)?
                && self.sub(&f.env, &g.env, Mode::Env)?),
            _ => Ok(false),
        }
    }

    /// `l <: m1 | m2 | …`: some member holds `l`.
    fn any_member(&mut self, l: &Ty, ms: &[Ty], mode: Mode) -> Result<bool, ()> {
        let h = l.head();
        if h.is_some() && h != Some(Head::Fn) {
            let same: Vec<Ty> = ms.iter().filter(|m| m.head() == h).cloned().collect();
            for m in &same {
                if self.sub(l, m, mode)? {
                    return Ok(true);
                }
            }
        }
        for m in ms {
            if m.head() != h && self.sub(l, m, mode)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn uppers(&self, v: V) -> Vec<Ty> {
        let st = &self.s.vars[v.0 as usize];
        let mut out = st.upper.clone();
        out.extend(st.into.iter().map(|w| Ty::Var(*w)));
        out
    }

    fn lowers(&self, v: V) -> Vec<Ty> {
        let st = &self.s.vars[v.0 as usize];
        let mut out = st.lower.clone();
        out.extend(st.from.iter().map(|w| Ty::Var(*w)));
        out
    }

    /// `u` flows to `w` through variable edges.
    fn reaches(&self, u: V, w: V) -> bool {
        let mut seen = FxHashSet::default();
        let mut todo = vec![u];
        while let Some(v) = todo.pop() {
            if v == w {
                return true;
            }
            if !seen.insert(v) {
                continue;
            }
            let st = &self.s.vars[v.0 as usize];
            todo.extend(st.into.iter().copied());
            todo.extend(st.upper.iter().filter_map(|t| match t {
                Ty::Var(x) => Some(*x),
                _ => None,
            }));
        }
        false
    }
}
