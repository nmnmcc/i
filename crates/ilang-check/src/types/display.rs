//! Showing a type (§4.7): coalescing the bounds of the type variables into
//! unions and intersections (Simple-sub), simplifying the result, naming the
//! variables, and writing it in the syntax of §3.
//!
//! Simplification removes a variable that is only in output places (it is
//! `Never`) or only in input places (it has no effect), and puts together a
//! variable with a type that it always comes with, both as an output and as
//! an input. A variable written in an annotation keeps its name, and is
//! not removed for being on one side only.

use super::solve::Solver;
use super::{FnTy, Head, Ty, V, Variance};
use crate::hir::TypeRef;
use std::collections::{HashMap, HashSet};

/// What a shown type keeps.
#[derive(Clone, Copy, Default)]
pub(crate) struct Opts {
    /// Variables at level 0 are not generalized (the bindings of the top
    /// level whose values are not direct values): each is shown as the
    /// union of the types that come to it.
    pub mono: bool,
    /// Written variables are kept, also on one side only: a declared type
    /// is shown as it is written (§10.1).
    pub written: bool,
}

/// The shown type scheme of `t`. `Err` when the type has itself inside it
/// through a type variable (§4.7).
pub(crate) fn scheme(s: &Solver, t: &Ty, opts: Opts) -> Result<String, ()> {
    let t = simplify_with(s, t, None, opts)?;
    Ok(Namer::new(s).show(&t))
}

/// The shown type, as a type, for checks of what it uses.
pub(crate) fn shown(s: &Solver, t: &Ty, opts: Opts) -> Result<Ty, ()> {
    simplify_with(s, t, None, opts)
}

/// `t` with the bounds of its variables coalesced, simplified. The
/// variables at `keep` or below are not generalized: they are kept as they
/// are. `Err` when the type has itself inside it through a type variable.
pub(crate) fn simplify(s: &Solver, t: &Ty, keep: Option<u32>) -> Result<Ty, ()> {
    simplify_with(s, t, keep, Opts::default())
}

fn simplify_with(s: &Solver, t: &Ty, keep: Option<u32>, opts: Opts) -> Result<Ty, ()> {
    let mut c = Coalescer {
        s,
        keep,
        mono: opts.mono,
        stack: Vec::new(),
        occurs: false,
        rep: cycles(s, keep),
    };
    let t = c.go(t, true, 0, false);
    if c.occurs {
        return Err(());
    }
    let n = Norm {
        s,
        keep,
        written: opts.written,
    };
    let mut t = n.norm(t);
    for _ in 0..64 {
        match n.step(&t) {
            Some((v, by)) => t = n.norm(replace(&t, v, &by)),
            None => break,
        }
    }
    Ok(t)
}

fn rigid(s: &Solver, keep: Option<u32>, v: V) -> bool {
    keep.is_some_and(|k| s.vars.get(v.0 as usize).is_some_and(|x| x.level <= k))
}

/// A type as it is in the solver, for messages: variables are `_`.
pub(crate) fn show(s: &Solver, t: &Ty) -> String {
    render(&both(t), 0, &mut |_| "_".to_string(), &s.type_names)
}

/// The variables that are equal because they are in a cycle of bounds that
/// are variables (`a <: b <: a`), each with the one that stands for them
/// all: a variable kept as it is, else a written one, else the first.
fn cycles(s: &Solver, keep: Option<u32>) -> HashMap<V, V> {
    let n = s.vars.len();
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, st) in s.vars.iter().enumerate() {
        for u in &st.upper {
            if let Ty::Var(u) = u {
                edges[i].push(u.0 as usize);
            }
        }
        for l in &st.lower {
            if let Ty::Var(l) = l {
                edges[l.0 as usize].push(i);
            }
        }
    }
    let comp = scc(&edges);
    let mut best: HashMap<usize, V> = HashMap::new();
    let rank = |v: V| {
        let st = &s.vars[v.0 as usize];
        (!rigid(s, keep, v), st.name.is_none(), v.0)
    };
    for (i, c) in comp.iter().enumerate() {
        let v = V(i as u32);
        match best.get(c) {
            Some(b) if rank(*b) <= rank(v) => {}
            _ => {
                best.insert(*c, v);
            }
        }
    }
    let mut size: HashMap<usize, usize> = HashMap::new();
    for c in &comp {
        *size.entry(*c).or_default() += 1;
    }
    comp.iter()
        .enumerate()
        .filter(|(_, c)| size[c] > 1)
        .map(|(i, c)| (V(i as u32), best[c]))
        .collect()
}

/// The strongly connected components of a graph, as a number for each node
/// (Tarjan, without recursion).
fn scc(edges: &[Vec<usize>]) -> Vec<usize> {
    let n = edges.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0; n];
    let mut on = vec![false; n];
    let mut stack = Vec::new();
    let mut comp = vec![usize::MAX; n];
    let (mut next, mut ncomp) = (0, 0);
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut work = vec![(root, 0usize)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on[root] = true;
        while let Some(&mut (v, ref mut ei)) = work.last_mut() {
            if *ei < edges[v].len() {
                let w = edges[v][*ei];
                *ei += 1;
                if index[w] == usize::MAX {
                    index[w] = next;
                    low[w] = next;
                    next += 1;
                    stack.push(w);
                    on[w] = true;
                    work.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                work.pop();
                if let Some(&(u, _)) = work.last() {
                    low[u] = low[u].min(low[v]);
                }
                if low[v] == index[v] {
                    while let Some(w) = stack.pop() {
                        on[w] = false;
                        comp[w] = ncomp;
                        if w == v {
                            break;
                        }
                    }
                    ncomp += 1;
                }
            }
        }
    }
    comp
}

// ---- Coalescing ----

struct Coalescer<'s> {
    s: &'s Solver,
    keep: Option<u32>,
    mono: bool,
    /// The variables being expanded, with the number of type constructors
    /// above them.
    stack: Vec<(V, u32)>,
    occurs: bool,
    /// The variables that stand for a cycle of variables.
    rep: HashMap<V, V>,
}

impl Coalescer<'_> {
    /// The variables that `v` stands for.
    fn members(&self, v: V) -> Vec<V> {
        if !self.rep.values().any(|r| *r == v) {
            return vec![v];
        }
        self.rep
            .iter()
            .filter(|(_, r)| **r == v)
            .map(|(m, _)| *m)
            .collect()
    }

    /// `env`: the type is an environment, whose members have arguments that
    /// are compared as equal (§4.5 rule 4), so they are both outputs and
    /// inputs.
    fn go(&mut self, t: &Ty, pos: bool, depth: u32, env: bool) -> Ty {
        match t {
            Ty::Var(v) => {
                let v = self.rep.get(v).copied().unwrap_or(*v);
                if rigid(self.s, self.keep, v) {
                    return Ty::Var(v);
                }
                let mono = self.mono && self.s.vars[v.0 as usize].level == 0;
                if let Some(&(_, d)) = self.stack.iter().find(|x| x.0 == v) {
                    if depth > d {
                        self.occurs = true;
                    }
                    return if mono { Ty::Never } else { Ty::Var(v) };
                }
                self.stack.push((v, depth));
                let mut parts = Vec::new();
                if mono {
                    // Not generalized: the types that come to it (§4.7).
                    for m in self.members(v) {
                        let st = &self.s.vars[m.0 as usize];
                        let mut bounds = st.lower.clone();
                        bounds.extend(st.from.iter().map(|w| Ty::Var(*w)));
                        for b in bounds {
                            parts.push(self.go(&b, true, depth, env));
                        }
                    }
                } else {
                    parts.push(Ty::Var(v));
                    for m in self.members(v) {
                        let st = &self.s.vars[m.0 as usize];
                        let bounds = if pos { &st.lower } else { &st.upper };
                        for b in bounds.clone() {
                            parts.push(self.go(&b, pos, depth, env));
                        }
                    }
                }
                self.stack.pop();
                if pos || mono {
                    Ty::Union(parts)
                } else {
                    Ty::Inter(parts)
                }
            }
            Ty::Con(h, args) => {
                let vs = self.s.variances(*h, args.len());
                let args = args
                    .iter()
                    .zip(vs)
                    .map(|(a, v)| match v {
                        _ if env => Ty::Both(
                            Box::new(self.go(a, true, depth + 1, false)),
                            Box::new(self.go(a, false, depth + 1, false)),
                        ),
                        Variance::Co | Variance::Bi => self.go(a, pos, depth + 1, false),
                        Variance::Contra => self.go(a, !pos, depth + 1, false),
                        Variance::Inv => Ty::Both(
                            Box::new(self.go(a, true, depth + 1, false)),
                            Box::new(self.go(a, false, depth + 1, false)),
                        ),
                    })
                    .collect();
                Ty::Con(*h, args)
            }
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f
                    .params
                    .iter()
                    .map(|(l, p)| (l.clone(), self.go(p, !pos, depth + 1, false)))
                    .collect(),
                ret: self.go(&f.ret, pos, depth + 1, false),
                err: self.go(&f.err, pos, depth + 1, false),
                env: self.go(&f.env, pos, depth + 1, true),
            })),
            Ty::Union(ms) => Ty::Union(ms.iter().map(|m| self.go(m, pos, depth, env)).collect()),
            Ty::Inter(ms) => Ty::Inter(ms.iter().map(|m| self.go(m, pos, depth, env)).collect()),
            Ty::Diff(a, h) => Ty::Diff(
                Box::new(self.go(a, pos, depth, env)),
                h.iter().map(|m| self.go(m, pos, depth, false)).collect(),
            ),
            Ty::Both(a, b) => Ty::Both(
                Box::new(self.go(a, true, depth, env)),
                Box::new(self.go(b, false, depth, env)),
            ),
            Ty::Never | Ty::Top => t.clone(),
        }
    }
}

/// An argument that does not change, with its bounds as an output and as
/// an input, is shown as one type: the input side, which every value that
/// comes there is in, and which is enough for every use there; when the
/// input side has nothing, the output side (§4.5 rules 6 and 7).
pub(crate) fn both(t: &Ty) -> Ty {
    match t {
        Ty::Both(a, b) => {
            if **b == Ty::Top {
                both(a)
            } else {
                both(b)
            }
        }
        Ty::Con(h, xs) => Ty::Con(*h, xs.iter().map(both).collect()),
        Ty::Union(xs) => Ty::Union(xs.iter().map(both).collect()),
        Ty::Inter(xs) => Ty::Inter(xs.iter().map(both).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f.params.iter().map(|(l, p)| (l.clone(), both(p))).collect(),
            ret: both(&f.ret),
            err: both(&f.err),
            env: both(&f.env),
        })),
        Ty::Diff(a, h) => Ty::Diff(Box::new(both(a)), h.iter().map(both).collect()),
        t => t.clone(),
    }
}

// ---- Simplification ----

struct Norm<'s> {
    s: &'s Solver,
    keep: Option<u32>,
    written: bool,
}

/// The occurrences of a variable: how many on each side, and the members
/// it always comes with on each side.
#[derive(Default)]
struct Occ {
    pos: usize,
    neg: usize,
    pos_with: Option<Vec<Ty>>,
    neg_with: Option<Vec<Ty>>,
}

impl Norm<'_> {
    fn norm(&self, t: Ty) -> Ty {
        match t {
            Ty::Union(ms) => self.union(ms.into_iter().map(|m| self.norm(m)).collect()),
            Ty::Inter(ms) => self.inter(ms.into_iter().map(|m| self.norm(m)).collect()),
            Ty::Diff(a, h) => {
                self.diff(self.norm(*a), h.into_iter().map(|m| self.norm(m)).collect())
            }
            Ty::Con(h, args) => Ty::Con(h, args.into_iter().map(|a| self.norm(a)).collect()),
            Ty::Fn(f) => {
                let f = *f;
                Ty::Fn(Box::new(FnTy {
                    params: f
                        .params
                        .into_iter()
                        .map(|(l, p)| (l, self.norm(p)))
                        .collect(),
                    ret: self.norm(f.ret),
                    err: self.norm(f.err),
                    env: self.env(f.env),
                }))
            }
            Ty::Both(a, b) => {
                let (a, b) = (self.norm(*a), self.norm(*b));
                if a == b {
                    a
                } else {
                    Ty::Both(Box::new(a), Box::new(b))
                }
            }
            t => t,
        }
    }

    /// An environment: two forms of one type constructor are not put
    /// together, because their arguments must be equal (§9.2).
    fn env(&self, t: Ty) -> Ty {
        match t {
            Ty::Union(ms) => self.union_in(ms.into_iter().map(|m| self.norm(m)).collect(), true),
            t => self.norm(t),
        }
    }

    fn union(&self, ms: Vec<Ty>) -> Ty {
        self.union_in(ms, false)
    }

    fn union_in(&self, ms: Vec<Ty>, env: bool) -> Ty {
        let mut flat = Vec::new();
        super::solve::flatten_union(&ms, &mut flat);
        let mut out: Vec<Ty> = Vec::new();
        for m in flat {
            if m == Ty::Never || out.contains(&m) {
                continue;
            }
            if m == Ty::Top {
                return Ty::Top;
            }
            match m
                .head()
                .and_then(|h| out.iter().position(|o| o.head() == Some(h)))
            {
                Some(_) if env => out.push(m),
                Some(i) => {
                    let old = std::mem::replace(&mut out[i], Ty::Never);
                    out[i] = self.merge(old, m, true);
                }
                None => out.push(m),
            }
        }
        // `a | a - H` is `a`, and `a | a & T` is `a`.
        let snapshot = out.clone();
        out.retain(|m| match m {
            Ty::Diff(a, _) => !snapshot.contains(a),
            Ty::Inter(xs) => !xs.iter().any(|x| snapshot.contains(x)),
            _ => true,
        });
        // `T | r - T`, where `r` has no effect and `T` has every value of
        // its type constructor, is every value.
        let full: HashSet<Head> = out
            .iter()
            .filter(|m| self.full(m))
            .filter_map(Ty::head)
            .collect();
        if out.iter().any(|m| match m {
            Ty::Diff(a, h) => {
                **a == Ty::Top
                    && h.iter()
                        .all(|x| x.head().is_some_and(|y| full.contains(&y)))
            }
            _ => false,
        }) {
            return Ty::Top;
        }
        match out.len() {
            0 => Ty::Never,
            1 => out.pop().unwrap(),
            _ => Ty::Union(out),
        }
    }

    /// `t` has every value of its type constructor.
    fn full(&self, t: &Ty) -> bool {
        match t {
            Ty::Con(h, args) => {
                let vs = self.s.variances(*h, args.len());
                args.iter().zip(vs).all(|(a, v)| match v {
                    Variance::Co => *a == Ty::Top,
                    Variance::Contra => *a == Ty::Never,
                    Variance::Bi => true,
                    Variance::Inv => false,
                })
            }
            Ty::Fn(f) => {
                f.params.iter().all(|p| p.1 == Ty::Never)
                    && [&f.ret, &f.err, &f.env].iter().all(|x| **x == Ty::Top)
            }
            _ => false,
        }
    }

    fn inter(&self, ms: Vec<Ty>) -> Ty {
        let mut flat = Vec::new();
        for m in ms {
            match m {
                Ty::Inter(xs) => flat.extend(xs),
                m => flat.push(m),
            }
        }
        // `{ A | B } & C` is `A & C | B & C` (§4.4), for unions of types
        // with type constructors.
        let (unions, rest): (Vec<Ty>, Vec<Ty>) =
            flat.into_iter().partition(|m| matches!(m, Ty::Union(_)));
        let conc: Vec<Ty> = rest
            .iter()
            .filter(|m| m.head().is_some())
            .cloned()
            .collect();
        let headed = |u: &Ty| match u {
            Ty::Union(xs) => xs.iter().all(|x| x.head().is_some()),
            _ => false,
        };
        let combos: usize = unions
            .iter()
            .map(|u| match u {
                Ty::Union(xs) => xs.len(),
                _ => 1,
            })
            .product();
        if (unions.len() >= 2 || (unions.len() == 1 && !conc.is_empty()))
            && unions.iter().all(headed)
            && combos <= 64
        {
            let mut alts: Vec<Vec<Ty>> = vec![conc];
            for u in &unions {
                let Ty::Union(xs) = u else { continue };
                alts = alts
                    .into_iter()
                    .flat_map(|a| {
                        xs.iter().map(move |x| {
                            let mut a = a.clone();
                            a.push(x.clone());
                            a
                        })
                    })
                    .collect();
            }
            let joined = self.union(alts.into_iter().map(|a| self.inter(a)).collect());
            let mut ms: Vec<Ty> = rest.into_iter().filter(|m| m.head().is_none()).collect();
            if ms.is_empty() {
                return joined;
            }
            ms.push(joined);
            return self.inter(ms);
        }
        let flat: Vec<Ty> = rest.into_iter().chain(unions).collect();
        let mut out: Vec<Ty> = Vec::new();
        for m in flat {
            if m == Ty::Top || out.contains(&m) {
                continue;
            }
            if m == Ty::Never {
                return Ty::Never;
            }
            match m.head() {
                Some(h) => match out.iter().position(|o| o.head().is_some()) {
                    Some(i) if out[i].head() == Some(h) => {
                        let old = std::mem::replace(&mut out[i], Ty::Never);
                        out[i] = self.merge(old, m, false);
                    }
                    Some(_) => return Ty::Never,
                    None => out.push(m),
                },
                None => out.push(m),
            }
        }
        // `a & { a | T }` is `a`.
        let snapshot = out.clone();
        out.retain(|m| match m {
            Ty::Union(xs) => !xs.iter().any(|x| snapshot.contains(x)),
            _ => true,
        });
        match out.len() {
            0 => Ty::Top,
            1 => out.pop().unwrap(),
            _ => Ty::Inter(out),
        }
    }

    fn diff(&self, a: Ty, h: Vec<Ty>) -> Ty {
        let mut hs = Vec::new();
        super::solve::flatten_union(&h, &mut hs);
        hs.retain(|m| *m != Ty::Never);
        if hs.is_empty() {
            return a;
        }
        match a {
            Ty::Never => Ty::Never,
            Ty::Union(ms) => self.union(ms.into_iter().map(|m| self.diff(m, hs.clone())).collect()),
            Ty::Con(..) | Ty::Fn(_) => {
                if hs.iter().any(|m| m.head() == a.head()) {
                    Ty::Never
                } else {
                    a
                }
            }
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

    fn merge(&self, a: Ty, b: Ty, union: bool) -> Ty {
        let pick = |x: Ty, y: Ty, v: Variance| match (v, union) {
            (Variance::Co | Variance::Bi, true) | (Variance::Contra, false) => {
                self.union(vec![x, y])
            }
            (Variance::Co | Variance::Bi, false) | (Variance::Contra, true) => {
                self.inter(vec![x, y])
            }
            (Variance::Inv, _) => x,
        };
        match (a, b) {
            (Ty::Con(h, xs), Ty::Con(_, ys)) if xs.len() == ys.len() => {
                let vs = self.s.variances(h, xs.len());
                Ty::Con(
                    h,
                    xs.into_iter()
                        .zip(ys)
                        .zip(vs)
                        .map(|((x, y), v)| pick(x, y, v))
                        .collect(),
                )
            }
            (Ty::Fn(f), Ty::Fn(g)) if f.params.len() == g.params.len() => {
                let (f, g) = (*f, *g);
                Ty::Fn(Box::new(FnTy {
                    params: f
                        .params
                        .into_iter()
                        .zip(g.params)
                        .map(|((l1, p), (l2, q))| {
                            (
                                if l1 == l2 { l1 } else { None },
                                pick(p, q, Variance::Contra),
                            )
                        })
                        .collect(),
                    ret: pick(f.ret, g.ret, Variance::Co),
                    err: pick(f.err, g.err, Variance::Co),
                    env: pick(f.env, g.env, Variance::Co),
                }))
            }
            (a, _) => a,
        }
    }

    /// The next variable to replace, and what it is replaced by.
    fn step(&self, t: &Ty) -> Option<(V, Ty)> {
        let mut occ: HashMap<V, Occ> = HashMap::new();
        let mut order = Vec::new();
        self.walk(t, true, &mut occ, &mut order, false);
        for v in &order {
            if rigid(self.s, self.keep, *v) {
                continue;
            }
            let o = &occ[v];
            let named = self.named(*v);
            if !named && o.neg == 0 {
                return Some((*v, Ty::Never));
            }
            if !named && o.pos == 0 {
                return Some((*v, Ty::Top));
            }
            if o.pos == 0 || o.neg == 0 {
                // A written variable on one side only has no effect; when it
                // always comes with a variable that is not written, that
                // variable takes its name.
                let with = if o.pos == 0 { &o.neg_with } else { &o.pos_with };
                if let Some(Ty::Var(w)) = with
                    .iter()
                    .flatten()
                    .find(|w| matches!(w, Ty::Var(x) if !self.named(*x)))
                {
                    return Some((*w, Ty::Var(*v)));
                }
                // §4.7: a variable on one side only has no effect, also a
                // written one, but not in a declared type.
                if !self.written {
                    return Some((*v, if o.neg == 0 { Ty::Never } else { Ty::Top }));
                }
                continue;
            }
            let (pw, nw) = (
                o.pos_with.clone().unwrap_or_default(),
                o.neg_with.clone().unwrap_or_default(),
            );
            for w in pw.iter().filter(|w| nw.contains(w)) {
                match w {
                    Ty::Var(x) if x == v => {}
                    Ty::Var(x) if named && !self.named(*x) => return Some((*x, Ty::Var(*v))),
                    Ty::Var(_) => return Some((*v, w.clone())),
                    _ if !mentions(w, *v) => return Some((*v, w.clone())),
                    _ => {}
                }
            }
        }
        None
    }

    fn named(&self, v: V) -> bool {
        rigid(self.s, self.keep, v)
            || self
                .s
                .vars
                .get(v.0 as usize)
                .is_some_and(|s| s.name.is_some())
    }

    /// `env`: the type is an environment, whose members have arguments that
    /// are compared as equal (§4.5 rule 4): they count on both sides.
    fn walk(&self, t: &Ty, pos: bool, occ: &mut HashMap<V, Occ>, order: &mut Vec<V>, env: bool) {
        // Only a union in an output place and an intersection in an input
        // place say that a variable comes with the other members.
        let (group, together): (Vec<Ty>, bool) = match t {
            Ty::Union(ms) => (ms.clone(), pos),
            Ty::Inter(ms) => (ms.clone(), !pos),
            _ => (vec![t.clone()], true),
        };
        for (i, m) in group.iter().enumerate() {
            match m {
                Ty::Var(v) => {
                    let with: Vec<Ty> = if together {
                        group
                            .iter()
                            .enumerate()
                            .filter(|(j, _)| *j != i)
                            .map(|(_, x)| x.clone())
                            .collect()
                    } else {
                        Vec::new()
                    };
                    record(occ, order, *v, pos, with);
                }
                Ty::Union(_) | Ty::Inter(_) => self.walk(m, pos, occ, order, env),
                // Only the type constructors of `H` matter in `A - H`
                // (§4.4), so the variables in `H` are not counted.
                Ty::Diff(a, _) => match &**a {
                    Ty::Var(v) => record(occ, order, *v, pos, Vec::new()),
                    a => self.walk(a, pos, occ, order, env),
                },
                Ty::Con(h, args) => {
                    let vs = self.s.variances(*h, args.len());
                    for (a, v) in args.iter().zip(vs) {
                        match v {
                            _ if env => {
                                self.walk(a, true, occ, order, false);
                                self.walk(a, false, occ, order, false);
                            }
                            Variance::Contra => self.walk(a, !pos, occ, order, false),
                            _ => self.walk(a, pos, occ, order, false),
                        }
                    }
                }
                Ty::Fn(f) => {
                    for (_, p) in &f.params {
                        self.walk(p, !pos, occ, order, false);
                    }
                    self.walk(&f.ret, pos, occ, order, false);
                    self.walk(&f.err, pos, occ, order, false);
                    self.walk(&f.env, pos, occ, order, true);
                }
                Ty::Both(a, b) => {
                    self.walk(a, true, occ, order, false);
                    self.walk(b, false, occ, order, false);
                }
                Ty::Never | Ty::Top => {}
            }
        }
    }
}

fn record(occ: &mut HashMap<V, Occ>, order: &mut Vec<V>, v: V, pos: bool, with: Vec<Ty>) {
    let o = occ.entry(v).or_insert_with(|| {
        order.push(v);
        Occ::default()
    });
    let (n, set) = if pos {
        (&mut o.pos, &mut o.pos_with)
    } else {
        (&mut o.neg, &mut o.neg_with)
    };
    *n += 1;
    *set = Some(match set.take() {
        None => with,
        Some(old) => old.into_iter().filter(|x| with.contains(x)).collect(),
    });
}

fn mentions(t: &Ty, v: V) -> bool {
    let mut found = false;
    visit(t, &mut |x| found |= *x == Ty::Var(v));
    found
}

fn visit(t: &Ty, f: &mut dyn FnMut(&Ty)) {
    f(t);
    match t {
        Ty::Con(_, xs) | Ty::Union(xs) | Ty::Inter(xs) => xs.iter().for_each(|x| visit(x, f)),
        Ty::Fn(g) => {
            g.params.iter().for_each(|p| visit(&p.1, f));
            [&g.ret, &g.err, &g.env]
                .into_iter()
                .for_each(|x| visit(x, f));
        }
        Ty::Diff(a, h) => {
            visit(a, f);
            h.iter().for_each(|x| visit(x, f));
        }
        Ty::Both(a, b) => {
            visit(a, f);
            visit(b, f);
        }
        _ => {}
    }
}

/// `t` with the variable `v` replaced by `by`.
fn replace(t: &Ty, v: V, by: &Ty) -> Ty {
    let r = |x: &Ty| replace(x, v, by);
    match t {
        Ty::Var(x) if *x == v => by.clone(),
        Ty::Con(h, xs) => Ty::Con(*h, xs.iter().map(r).collect()),
        Ty::Union(xs) => Ty::Union(xs.iter().map(r).collect()),
        Ty::Inter(xs) => Ty::Inter(xs.iter().map(r).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f.params.iter().map(|(l, p)| (l.clone(), r(p))).collect(),
            ret: r(&f.ret),
            err: r(&f.err),
            env: r(&f.env),
        })),
        Ty::Diff(a, h) => Ty::Diff(Box::new(r(a)), h.iter().map(r).collect()),
        Ty::Both(a, b) => Ty::Both(Box::new(r(a)), Box::new(r(b))),
        t => t.clone(),
    }
}

// ---- Names and text ----

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Value,
    Err,
    Env,
}

struct Namer<'s> {
    s: &'s Solver,
    names: HashMap<V, String>,
    taken: HashSet<String>,
}

impl<'s> Namer<'s> {
    fn new(s: &'s Solver) -> Namer<'s> {
        Namer {
            s,
            names: HashMap::new(),
            taken: HashSet::new(),
        }
    }

    fn show(mut self, t: &Ty) -> String {
        // `Top` is a variable that has no effect: each gets its own name.
        let mut next = u32::MAX;
        let t = tops(&holes(&both(t)), &mut next);
        let mut order = Vec::new();
        roles(&t, Role::Value, &mut order);
        // Written names first, then the others.
        for (v, _) in &order {
            if let Some(n) = self.s.vars.get(v.0 as usize).and_then(|s| s.name.clone())
                && !self.names.contains_key(v)
                && self.taken.insert(n.clone())
            {
                self.names.insert(*v, n);
            }
        }
        for (v, role) in &order {
            if self.names.contains_key(v) {
                continue;
            }
            let n = self.fresh(*role);
            self.names.insert(*v, n);
        }
        let names = self.names;
        render(
            &t,
            0,
            &mut |v| names.get(&v).cloned().unwrap_or_else(|| "_".into()),
            &self.s.type_names,
        )
    }

    fn fresh(&mut self, role: Role) -> String {
        let candidates: Box<dyn Iterator<Item = String>> = match role {
            Role::Err => {
                Box::new(std::iter::once("e".to_string()).chain((2..).map(|i| format!("e{i}"))))
            }
            Role::Env => {
                Box::new(std::iter::once("n".to_string()).chain((2..).map(|i| format!("n{i}"))))
            }
            Role::Value => Box::new(
                ('a'..='z')
                    .filter(|c| *c != 'e' && *c != 'n')
                    .map(String::from)
                    .chain((2..).flat_map(|i| {
                        ('a'..='z')
                            .filter(|c| *c != 'e' && *c != 'n')
                            .map(move |c| format!("{c}{i}"))
                    })),
            ),
        };
        for c in candidates {
            if self.taken.insert(c.clone()) {
                return c;
            }
        }
        unreachable!()
    }
}

fn tops(t: &Ty, next: &mut u32) -> Ty {
    match t {
        Ty::Top => {
            *next -= 1;
            Ty::Var(V(*next))
        }
        Ty::Con(h, xs) => Ty::Con(*h, xs.iter().map(|x| tops(x, next)).collect()),
        Ty::Union(xs) => Ty::Union(xs.iter().map(|x| tops(x, next)).collect()),
        Ty::Inter(xs) => Ty::Inter(xs.iter().map(|x| tops(x, next)).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f
                .params
                .iter()
                .map(|(l, p)| (l.clone(), tops(p, next)))
                .collect(),
            ret: tops(&f.ret, next),
            err: tops(&f.err, next),
            env: tops(&f.env, next),
        })),
        // A part of `H` with no effect is written `_`.
        Ty::Diff(a, h) => Ty::Diff(Box::new(tops(a, next)), h.clone()),
        Ty::Both(a, b) => Ty::Both(Box::new(tops(a, next)), Box::new(tops(b, next))),
        t => t.clone(),
    }
}

/// The variables in the order they are written, each with the part of a
/// function type it is first a member of.
fn roles(t: &Ty, role: Role, out: &mut Vec<(V, Role)>) {
    match t {
        Ty::Var(v) => {
            if !out.iter().any(|x| x.0 == *v) {
                out.push((*v, role));
            }
        }
        Ty::Union(xs) | Ty::Inter(xs) => xs.iter().for_each(|x| roles(x, role, out)),
        Ty::Diff(a, h) => {
            roles(a, role, out);
            h.iter().for_each(|x| roles(x, Role::Value, out));
        }
        Ty::Con(_, xs) => xs.iter().for_each(|x| roles(x, Role::Value, out)),
        Ty::Fn(f) => {
            f.params.iter().for_each(|p| roles(&p.1, Role::Value, out));
            roles(&f.ret, Role::Value, out);
            roles(&f.err, Role::Err, out);
            roles(&f.env, Role::Env, out);
        }
        Ty::Both(a, b) => {
            roles(a, role, out);
            roles(b, role, out);
        }
        Ty::Never | Ty::Top => {}
    }
}

/// The text of a type (§3). `prec` is what the place allows: 0 any type, 1
/// no union, 2 no intersection, 3 no difference and no function type.
fn render(t: &Ty, prec: u8, name: &mut dyn FnMut(V) -> String, tn: &[String]) -> String {
    let wrap = |s: String, need: u8| if prec > need { format!("{{ {s} }}") } else { s };
    match t {
        Ty::Var(v) => name(*v),
        Ty::Never => "Never".into(),
        Ty::Top => "_".into(),
        Ty::Con(Head::Named(r), args) => {
            let base = type_name(*r, tn);
            if args.is_empty() {
                base
            } else {
                let args: Vec<String> = args.iter().map(|a| render(a, 0, name, tn)).collect();
                format!("{base}({})", args.join(", "))
            }
        }
        Ty::Con(_, parts) => {
            let parts: Vec<String> = parts.iter().map(|a| render(a, 0, name, tn)).collect();
            format!("({})", parts.join(", "))
        }
        Ty::Fn(f) => {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|(l, p)| match l {
                    Some(l) => format!("{l}: {}", render(p, 0, name, tn)),
                    None => render(p, 0, name, tn),
                })
                .collect();
            let has_err = f.err != Ty::Never;
            let has_env = f.env != Ty::Never;
            let part = |x: &Ty, followed: bool, name: &mut dyn FnMut(V) -> String| {
                let s = render(x, 0, name, tn);
                if followed && matches!(x, Ty::Fn(_)) {
                    format!("{{ {s} }}")
                } else {
                    s
                }
            };
            let mut s = format!(
                "({}) -> {}",
                params.join(", "),
                part(&f.ret, has_err || has_env, name)
            );
            if has_err {
                s.push_str(&format!(" ? {}", part(&f.err, has_env, name)));
            }
            if has_env {
                s.push_str(&format!(" ! {}", part(&f.env, false, name)));
            }
            wrap(s, 0)
        }
        Ty::Union(ms) => {
            let ms: Vec<String> = ms.iter().map(|m| render(m, 1, name, tn)).collect();
            wrap(ms.join(" | "), 0)
        }
        Ty::Inter(ms) => {
            let ms: Vec<String> = ms.iter().map(|m| render(m, 2, name, tn)).collect();
            wrap(ms.join(" & "), 1)
        }
        Ty::Diff(a, h) => {
            // Only the type constructors of `H` matter (§4.4).
            let mut s = render(a, 2, name, tn);
            for m in h {
                s.push_str(" - ");
                s.push_str(&render(m, 3, name, tn));
            }
            wrap(s, 2)
        }
        Ty::Both(a, b) => {
            if **a == Ty::Never {
                render(b, prec, name, tn)
            } else {
                render(a, prec, name, tn)
            }
        }
    }
}

/// `t` with the variables of `H` in each difference that are nowhere else
/// changed to `Top`, shown `_`: only the type constructors of `H` matter
/// there (§4.4), and such a variable has no other effect.
fn holes(t: &Ty) -> Ty {
    let mut outside = HashSet::new();
    outer_vars(t, &mut outside);
    hide(t, &outside, false)
}

fn outer_vars(t: &Ty, out: &mut HashSet<V>) {
    match t {
        Ty::Var(v) => {
            out.insert(*v);
        }
        Ty::Con(_, xs) | Ty::Union(xs) | Ty::Inter(xs) => {
            xs.iter().for_each(|x| outer_vars(x, out))
        }
        Ty::Fn(f) => {
            f.params.iter().for_each(|p| outer_vars(&p.1, out));
            [&f.ret, &f.err, &f.env]
                .into_iter()
                .for_each(|x| outer_vars(x, out));
        }
        Ty::Diff(a, _) => outer_vars(a, out),
        Ty::Both(a, b) => {
            outer_vars(a, out);
            outer_vars(b, out);
        }
        Ty::Never | Ty::Top => {}
    }
}

fn hide(t: &Ty, outside: &HashSet<V>, in_h: bool) -> Ty {
    let go = |x: &Ty| hide(x, outside, in_h);
    match t {
        Ty::Var(v) if in_h && !outside.contains(v) => Ty::Top,
        Ty::Con(h, xs) => Ty::Con(*h, xs.iter().map(go).collect()),
        Ty::Union(xs) => Ty::Union(xs.iter().map(go).collect()),
        Ty::Inter(xs) => Ty::Inter(xs.iter().map(go).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f.params.iter().map(|(l, p)| (l.clone(), go(p))).collect(),
            ret: go(&f.ret),
            err: go(&f.err),
            env: go(&f.env),
        })),
        Ty::Diff(a, h) => Ty::Diff(
            Box::new(go(a)),
            h.iter().map(|x| hide(x, outside, true)).collect(),
        ),
        Ty::Both(a, b) => Ty::Both(Box::new(go(a)), Box::new(go(b))),
        t => t.clone(),
    }
}

fn type_name(r: TypeRef, types: &[String]) -> String {
    match r {
        TypeRef::Builtin(b) => b.name().to_string(),
        TypeRef::Custom(id) => types
            .get(id.0 as usize)
            .cloned()
            .unwrap_or_else(|| format!("T{}", id.0)),
    }
}

// ---- Known types (§4.8) ----

/// The known type of a name with the type scheme `t`, whose variables
/// deeper than `level` are general: the scheme in its simpler form (§4.7).
/// A variable that is not general is not known, so it is `_` (`Top`).
pub(crate) fn known(s: &Solver, t: &Ty, level: u32) -> Ty {
    match simplify(s, t, Some(level)) {
        Ok(t) => known_norm(s, &not_general(s, &both(&t), level)),
        Err(()) => Ty::Top,
    }
}

fn not_general(s: &Solver, t: &Ty, level: u32) -> Ty {
    let go = |x: &Ty| not_general(s, x, level);
    match t {
        Ty::Var(v) if s.vars.get(v.0 as usize).is_none_or(|x| x.level <= level) => Ty::Top,
        Ty::Con(h, xs) => Ty::Con(*h, xs.iter().map(go).collect()),
        Ty::Union(xs) => Ty::Union(xs.iter().map(go).collect()),
        Ty::Inter(xs) => Ty::Inter(xs.iter().map(go).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f.params.iter().map(|(l, p)| (l.clone(), go(p))).collect(),
            ret: go(&f.ret),
            err: go(&f.err),
            env: go(&f.env),
        })),
        Ty::Diff(a, h) => Ty::Diff(Box::new(go(a)), h.iter().map(go).collect()),
        Ty::Both(a, b) => Ty::Both(Box::new(go(a)), Box::new(go(b))),
        t => t.clone(),
    }
}

fn known_ops(s: &Solver) -> Norm<'_> {
    Norm {
        s,
        keep: None,
        written: false,
    }
}

/// The union of known types (§4.4), with no constraints: `_` in a union is
/// `_`, because a part that is not known makes the union not known.
pub(crate) fn known_union(s: &Solver, ms: Vec<Ty>) -> Ty {
    known_ops(s).union(ms)
}

pub(crate) fn known_inter(s: &Solver, ms: Vec<Ty>) -> Ty {
    known_ops(s).inter(ms)
}

pub(crate) fn known_diff(s: &Solver, a: Ty, h: Vec<Ty>) -> Ty {
    known_ops(s).diff(a, h)
}

/// A known type with its unions, intersections, and differences worked
/// out again, after type variables were put in.
pub(crate) fn known_norm(s: &Solver, t: &Ty) -> Ty {
    known_ops(s).norm(t.clone())
}

/// The text of a known type: a part that is not known is `_`, and the type
/// variables of a scheme are named.
pub(crate) fn show_known(s: &Solver, t: &Ty) -> String {
    let mut names: HashMap<V, String> = HashMap::new();
    let mut next = 0u8;
    render(
        t,
        0,
        &mut |v| {
            names
                .entry(v)
                .or_insert_with(|| {
                    let n = s
                        .vars
                        .get(v.0 as usize)
                        .and_then(|x| x.name.clone())
                        .unwrap_or_else(|| ((b'a' + next % 26) as char).to_string());
                    next += 1;
                    n
                })
                .clone()
        },
        &s.type_names,
    )
}
