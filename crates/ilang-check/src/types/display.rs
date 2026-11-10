//! Showing a type (§4.7): coalescing the bounds of the type variables into
//! unions and intersections (Simple-sub), simplifying the result, naming the
//! variables, and writing it in the syntax of §3.
//!
//! Simplification removes a variable that is only in output places (it is
//! `Never`) or only in input places (it has no effect), and puts together a
//! variable with a type that it always comes with, both as an output and as
//! an input. A variable written in an annotation is
//! not removed for being on one side only. Shown names are chosen independently
//! of written names.

use super::solve::{Mode, Solver};
use super::{FnTy, Head, ShownFn, ShownType, Ty, V, Variance};
// Fast hashing for the small keys of coalescing (docs/performance.md).
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

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
pub(crate) fn scheme(s: &Solver, t: &Ty, opts: Opts) -> Result<ShownType, ()> {
    let t = simplify_with(s, t, None, opts)?;
    Ok(Namer::new(s).show(&t))
}

/// The shown type of `t`, simplified together with the types in `context`:
/// a variable that `t` shares with them is kept, as it is in the type
/// scheme that they are part of, and only `t` is shown.
///
/// `context` is the type of a top-level binding (or the types of the names
/// of its pattern), and a variable of `t` that is a variable of its shown
/// type scheme has the name it has there. A variable on one side only of
/// the scheme has no effect, and the scheme shows it as a name of its own:
/// `t` shows it with that name too. The other variables of `t` have names
/// that the scheme does not use.
pub(crate) fn scheme_in(s: &Solver, t: &Ty, context: &[Ty], opts: Opts) -> Result<String, ()> {
    let mut namer = Namer::new(s);
    let scheme = match context {
        [] => None,
        [b] => Some(scheme_names(s, b)?),
        _ => Some(scheme_names(s, &Ty::tuple(context.to_vec()))?),
    };
    let mut parts = context.to_vec();
    parts.push(t.clone());
    let traced = trace(s, &Ty::tuple(parts), opts)?;
    let (whole, origin) = traced.prepare();
    let Ty::Con(_, _, parts) = whole else {
        return Err(());
    };
    let t = parts.last().ok_or(())?;
    if let Some(scheme) = scheme {
        namer.taken = scheme.taken;
        // The variables of `t` that stand for a variable of the scheme take
        // its name, each name once.
        let mut order = Vec::new();
        roles(t, Role::Value, &mut order);
        let mut used: HashMap<String, V> = HashMap::default();
        for (x, _) in order {
            let Some(key) = origin.get(&x).copied().or((x.0 < MARK).then_some(x)) else {
                continue;
            };
            let Some((_, name)) = traced
                .class(key)
                .iter()
                .filter_map(|m| scheme.names.get(m))
                .min()
            else {
                continue;
            };
            if used.get(name).is_some_and(|k| *k != key) {
                continue;
            }
            used.insert(name.clone(), key);
            namer.names.insert(x, name.clone());
        }
    }
    namer.assign(t);
    Ok(namer.tree(t).text_limited(&s.type_names, 12))
}

/// The names that the shown type scheme of `t` (the type of a top-level
/// binding, named as [`scheme`] names it) gives to the type variables.
struct SchemeNames {
    /// For each variable that a name of the scheme stands for: the place of
    /// the name in the scheme, and the name.
    names: HashMap<V, (usize, String)>,
    /// Names used by the scheme and by unqualified types.
    taken: HashSet<String>,
}

fn scheme_names(s: &Solver, t: &Ty) -> Result<SchemeNames, ()> {
    let opts = Opts {
        mono: true,
        written: false,
    };
    let traced = trace(s, t, opts)?;
    let (shown, origin) = traced.prepare();
    let mut namer = Namer::new(s);
    namer.assign(&shown);
    let mut order = Vec::new();
    roles(&shown, Role::Value, &mut order);
    let mut names = HashMap::default();
    for (i, (x, _)) in order.iter().enumerate() {
        let Some(key) = origin.get(x).copied().or((x.0 < MARK).then_some(*x)) else {
            continue;
        };
        let Some(name) = namer.names.get(x) else {
            continue;
        };
        for m in traced.class(key) {
            names.entry(m).or_insert_with(|| (i, name.clone()));
        }
    }
    Ok(SchemeNames {
        names,
        taken: namer.taken,
    })
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
    simplify_steps(s, t, keep, opts, None)
}

/// The steps of a simplification, to do them again.
struct Steps {
    /// The coalesced type, before the steps.
    start: Ty,
    /// Each variable replaced, and what it is replaced by.
    steps: Vec<(V, Ty)>,
    /// The variables of each cycle, by the variable that stands for them.
    members: HashMap<V, Vec<V>>,
}

fn simplify_steps(
    s: &Solver,
    t: &Ty,
    keep: Option<u32>,
    opts: Opts,
    mut record: Option<&mut Steps>,
) -> Result<Ty, ()> {
    let rep = cycles(s, t, keep);
    let mut members: HashMap<V, Vec<V>> = HashMap::default();
    for (m, r) in &rep {
        members.entry(*r).or_default().push(*m);
    }
    let mut c = Coalescer {
        s,
        keep,
        mono: opts.mono,
        stack: HashMap::default(),
        memo: HashMap::default(),
        occurs: false,
        rep,
        members,
    };
    let input = t;
    let t = c.go(input, true, 0, false);
    if c.occurs {
        // Alias classes can expose a redundant bound that the raw occurs
        // graph did not reach. Use exactly its fixed-point reductions.
        let result = match s.normalized(input) {
            Some(view) => simplify_steps(&view, input, keep, opts, record),
            None => Err(()),
        };
        if result.is_err() {
            s.coalescing_failed.set(true);
        }
        return result;
    }
    let n = Norm {
        s,
        keep,
        written: opts.written,
    };
    let mut t = n.norm(t);
    if let Some(r) = record.as_deref_mut() {
        r.start = t.clone();
        r.members = std::mem::take(&mut c.members);
    }
    for _ in 0..64 {
        match n.step(&t) {
            Some((v, by)) => {
                t = n.norm(replace(&t, v, &by));
                if let Some(r) = record.as_deref_mut() {
                    r.steps.push((v, by));
                }
            }
            None => break,
        }
    }
    Ok(t)
}

/// A variable that is a mark: the first is `MARK`. A variable on one side
/// only is replaced by `Top`; a simplification done again with the same
/// steps replaces it by its mark instead, which counts as `Top`, so that
/// each `Top` of the result is known to stand for that variable.
const MARK: u32 = 1 << 31;

fn mark(v: V) -> V {
    V(MARK + v.0)
}

/// `Top`, or a mark (which is `Top`).
fn is_top(t: &Ty) -> bool {
    match t {
        Ty::Top => true,
        Ty::Var(v) => v.0 >= MARK,
        _ => false,
    }
}

/// `a` and `b` are equal when each mark is `Top`.
fn same_unmarked(a: &Ty, b: &Ty) -> bool {
    let all = |xs: &[Ty], ys: &[Ty]| {
        xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| same_unmarked(x, y))
    };
    match (a, b) {
        _ if is_top(a) && is_top(b) => true,
        (Ty::Con(h, a, xs), Ty::Con(g, b, ys)) => h == g && a == b && all(xs, ys),
        (Ty::Union(xs), Ty::Union(ys)) | (Ty::Inter(xs), Ty::Inter(ys)) => all(xs, ys),
        (Ty::Fn(f), Ty::Fn(g)) => {
            f.params.len() == g.params.len()
                && f.params
                    .iter()
                    .zip(&g.params)
                    .all(|(p, q)| p.0 == q.0 && same_unmarked(&p.1, &q.1))
                && same_unmarked(&f.ret, &g.ret)
                && same_unmarked(&f.err, &g.err)
                && same_unmarked(&f.env, &g.env)
        }
        (Ty::Diff(x, h), Ty::Diff(y, g)) => same_unmarked(x, y) && all(h, g),
        (Ty::Both(x, y), Ty::Both(z, w)) => same_unmarked(x, z) && same_unmarked(y, w),
        _ => a == b,
    }
}

/// A simplified type, with what its variables and its `Top`s stand for.
struct Traced {
    /// The simplified type, as `simplify_with` gives it.
    shown: Ty,
    /// `shown` with the `Top` that each variable on one side only became
    /// written as its mark, when the steps done again give `shown`.
    marked: Option<Ty>,
    /// For each variable of `shown` or mark, the variables that it stands
    /// for besides itself: those of its cycle, and those replaced by it.
    classes: HashMap<V, Vec<V>>,
}

impl Traced {
    /// The variables that `v` stands for.
    fn class(&self, v: V) -> Vec<V> {
        let mut out = vec![v];
        out.extend(self.classes.get(&v).into_iter().flatten().copied());
        out
    }

    /// The shown type as the names are given to it (`Namer::assign`), with
    /// the variable of each `Top` that stands for a variable on one side
    /// only.
    fn prepare(&self) -> (Ty, HashMap<V, V>) {
        let shown = prepare(&self.shown, &mut HashMap::default());
        let mut origin = HashMap::default();
        if let Some(m) = &self.marked
            && prepare(m, &mut origin) == shown
        {
            return (shown, origin);
        }
        (shown, HashMap::default())
    }
}

fn trace(s: &Solver, t: &Ty, opts: Opts) -> Result<Traced, ()> {
    let mut r = Steps {
        start: Ty::Never,
        steps: Vec::new(),
        members: HashMap::default(),
    };
    let shown = simplify_steps(s, t, None, opts, Some(&mut r))?;
    let n = Norm {
        s,
        keep: None,
        written: opts.written,
    };
    let mut classes = r.members;
    for (rep, ms) in classes.iter_mut() {
        ms.retain(|m| m != rep);
    }
    let mut u = r.start;
    for (v, by) in r.steps {
        let by = match by {
            Ty::Top => Ty::Var(mark(v)),
            Ty::Var(w) => {
                let mut moved = classes.remove(&v).unwrap_or_default();
                moved.push(v);
                classes.entry(w).or_default().extend(moved);
                Ty::Var(w)
            }
            by => {
                classes.remove(&v);
                by
            }
        };
        u = n.norm(replace(&u, v, &by));
    }
    let marked = same_unmarked(&u, &shown).then_some(u);
    Ok(Traced {
        shown,
        marked,
        classes,
    })
}

fn rigid(s: &Solver, keep: Option<u32>, v: V) -> bool {
    keep.is_some_and(|k| s.vars.get(v.0 as usize).is_some_and(|x| x.level <= k))
}

/// A type as it is in the solver, for messages: variables are `_`.
pub(crate) fn show(s: &Solver, t: &Ty) -> String {
    render(&both(t), &mut |_| "_".to_string(), &s.type_names)
}

/// The variables that are equal because they are in a cycle of bounds that
/// are variables (`a <: b <: a`), each with the one that stands for them
/// all: a variable kept as it is, else a written one, else the first.
///
/// Only the variables that `t` reaches through bounds, in either direction,
/// are looked at: coalescing goes to no other, and a cycle of one of them
/// has only such variables. So the cost is the size of the type, not of
/// the program.
fn cycles(s: &Solver, t: &Ty, keep: Option<u32>) -> HashMap<V, V> {
    // The variables reached, numbered in the order they are found.
    let mut number: HashMap<V, usize> = HashMap::default();
    let mut found: Vec<V> = Vec::new();
    let mut todo: Vec<V> = Vec::new();
    let add = |v: V, number: &mut HashMap<V, usize>, found: &mut Vec<V>, todo: &mut Vec<V>| {
        if let std::collections::hash_map::Entry::Vacant(e) = number.entry(v) {
            e.insert(found.len());
            found.push(v);
            todo.push(v);
        }
    };
    let mut vars = Vec::new();
    ty_vars(t, &mut vars);
    for v in vars.drain(..) {
        add(v, &mut number, &mut found, &mut todo);
    }
    while let Some(v) = todo.pop() {
        let st = &s.vars[v.0 as usize];
        for b in st.lower.iter().chain(&st.upper) {
            ty_vars(b, &mut vars);
        }
        vars.extend(st.from.iter().chain(&st.into).copied());
        for u in vars.drain(..) {
            add(u, &mut number, &mut found, &mut todo);
        }
    }
    let n = found.len();
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, v) in found.iter().enumerate() {
        let st = &s.vars[v.0 as usize];
        for u in &st.upper {
            if let Ty::Var(u) = u {
                edges[i].push(number[u]);
            }
        }
        for l in &st.lower {
            if let Ty::Var(l) = l {
                edges[number[l]].push(i);
            }
        }
    }
    let comp = scc(&edges);
    let mut best: HashMap<usize, V> = HashMap::default();
    let rank = |v: V| {
        let st = &s.vars[v.0 as usize];
        (!rigid(s, keep, v), st.name.is_none(), v.0)
    };
    for (i, c) in comp.iter().enumerate() {
        let v = found[i];
        match best.get(c) {
            Some(b) if rank(*b) <= rank(v) => {}
            _ => {
                best.insert(*c, v);
            }
        }
    }
    let mut size: HashMap<usize, usize> = HashMap::default();
    for c in &comp {
        *size.entry(*c).or_default() += 1;
    }
    comp.iter()
        .enumerate()
        .filter(|(_, c)| size[c] > 1)
        .map(|(i, c)| (found[i], best[c]))
        .collect()
}

/// The variables of a type, with repeats.
fn ty_vars(t: &Ty, out: &mut Vec<V>) {
    match t {
        Ty::Var(v) => out.push(*v),
        Ty::Con(_, _, xs) | Ty::Union(xs) | Ty::Inter(xs) => {
            xs.iter().for_each(|x| ty_vars(x, out))
        }
        Ty::Fn(f) => {
            f.params.iter().for_each(|p| ty_vars(&p.1, out));
            [&f.ret, &f.err, &f.env]
                .into_iter()
                .for_each(|x| ty_vars(x, out));
        }
        Ty::Diff(a, h) => {
            ty_vars(a, out);
            h.iter().for_each(|x| ty_vars(x, out));
        }
        Ty::Both(a, b) => {
            ty_vars(a, out);
            ty_vars(b, out);
        }
        Ty::Never | Ty::Top => {}
    }
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
    stack: HashMap<V, u32>,
    /// Completed expansions have no dependence on the path that reaches
    /// them. Alias SCCs already have one representative; recursive
    /// constructor paths are diagnosed before a result can be reused.
    memo: HashMap<(V, bool, bool), Ty>,
    occurs: bool,
    /// The variables that stand for a cycle of variables.
    rep: HashMap<V, V>,
    /// The variables of the cycle of each variable of `rep`.
    members: HashMap<V, Vec<V>>,
}

impl Coalescer<'_> {
    /// The variables that `v` stands for.
    fn members(&self, v: V) -> Vec<V> {
        match self.members.get(&v) {
            Some(ms) => ms.clone(),
            None => vec![v],
        }
    }

    /// `env`: the type is an environment, whose members have arguments that
    /// are compared as equal (§4.5 rule 4), so they are both outputs and
    /// inputs.
    fn go(&mut self, t: &Ty, pos: bool, depth: u32, env: bool) -> Ty {
        let env = env || self.s.is_row(t);
        match t {
            Ty::Var(v) => {
                let v = self.rep.get(v).copied().unwrap_or(*v);
                if rigid(self.s, self.keep, v) {
                    return Ty::Var(v);
                }
                let mono = self.mono && self.s.vars[v.0 as usize].level == 0;
                if let Some(&d) = self.stack.get(&v) {
                    if depth > d {
                        self.occurs = true;
                    }
                    return if mono { Ty::Never } else { Ty::Var(v) };
                }
                let empty = |m: V| {
                    let st = &self.s.vars[m.0 as usize];
                    if mono {
                        st.lower.is_empty() && st.from.is_empty()
                    } else if pos {
                        st.lower.is_empty()
                    } else {
                        st.upper.is_empty()
                    }
                };
                if self
                    .members
                    .get(&v)
                    .map_or_else(|| empty(v), |ms| ms.iter().copied().all(empty))
                {
                    return if mono { Ty::Never } else { Ty::Var(v) };
                }
                if let Some(t) = self.memo.get(&(v, pos, env)) {
                    return t.clone();
                }
                self.stack.insert(v, depth);
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
                self.stack.remove(&v);
                let positive = pos || mono;
                if parts.len() <= 2
                    && !parts.iter().any(|t| {
                        matches!(t, Ty::Union(_) if positive)
                            || matches!(t, Ty::Inter(_) if !positive)
                    })
                {
                    if parts.len() == 2 && parts[0] == parts[1] {
                        parts.pop();
                    }
                    let t = if parts.len() == 1 {
                        parts.pop().unwrap()
                    } else if positive {
                        Ty::Union(parts)
                    } else {
                        Ty::Inter(parts)
                    };
                    if !self.occurs {
                        self.memo.insert((v, pos, env), t.clone());
                    }
                    return t;
                }
                let mut flat = Vec::new();
                let mut todo: Vec<_> = parts.into_iter().rev().collect();
                let mut seen = HashSet::default();
                while let Some(t) = todo.pop() {
                    match t {
                        Ty::Union(ms) if positive => todo.extend(ms.into_iter().rev()),
                        Ty::Inter(ms) if !positive => todo.extend(ms.into_iter().rev()),
                        t if seen.insert(t.clone()) => flat.push(t),
                        _ => {}
                    }
                }
                let t = if positive {
                    Ty::Union(flat)
                } else {
                    Ty::Inter(flat)
                };
                if !self.occurs {
                    self.memo.insert((v, pos, env), t.clone());
                }
                t
            }
            Ty::Con(h, values, args) => {
                let vs = self.s.variances(*h, args.len());
                let args = args
                    .iter()
                    .zip(vs)
                    .zip(self.s.rows(*h, args.len()))
                    .map(|((a, v), row)| match v {
                        _ if env => Ty::Both(
                            Box::new(self.go(a, true, depth + 1, row != Mode::Value)),
                            Box::new(self.go(a, false, depth + 1, row != Mode::Value)),
                        ),
                        Variance::Co | Variance::Bi => {
                            self.go(a, pos, depth + 1, row != Mode::Value)
                        }
                        Variance::Contra => self.go(a, !pos, depth + 1, row != Mode::Value),
                        Variance::Inv => Ty::Both(
                            Box::new(self.go(a, true, depth + 1, row != Mode::Value)),
                            Box::new(self.go(a, false, depth + 1, row != Mode::Value)),
                        ),
                    })
                    .collect();
                Ty::Con(*h, values.clone(), args)
            }
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f
                    .params
                    .iter()
                    .map(|(l, p)| (l.clone(), self.go(p, !pos, depth + 1, false)))
                    .collect(),
                ret: self.go(&f.ret, pos, depth + 1, false),
                err: self.go(&f.err, pos, depth + 1, true),
                env: self.go(&f.env, pos, depth + 1, true),
            })),
            Ty::Union(ms) => Ty::Union(ms.iter().map(|m| self.go(m, pos, depth, env)).collect()),
            Ty::Inter(ms) => Ty::Inter(ms.iter().map(|m| self.go(m, pos, depth, env)).collect()),
            Ty::Diff(a, h) => Ty::Diff(
                Box::new(self.go(a, pos, depth, env)),
                h.iter().map(|m| self.go(m, pos, depth, env)).collect(),
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
            if is_top(b) {
                both(a)
            } else {
                both(b)
            }
        }
        Ty::Con(h, values, xs) => Ty::Con(*h, values.clone(), xs.iter().map(both).collect()),
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
/// it always comes with on each side. The simplification pass only reads
/// these members; borrowing them avoids copying entire neighboring types.
#[derive(Default)]
struct Occ<'a> {
    pos: usize,
    neg: usize,
    pos_with: Option<Vec<&'a Ty>>,
    neg_with: Option<Vec<&'a Ty>>,
}

impl Norm<'_> {
    fn norm(&self, t: Ty) -> Ty {
        match t {
            Ty::Union(ms) => self.union(ms.into_iter().map(|m| self.norm(m)).collect()),
            Ty::Inter(ms) => self.inter(ms.into_iter().map(|m| self.norm(m)).collect()),
            Ty::Diff(a, h) => {
                self.diff(self.norm(*a), h.into_iter().map(|m| self.norm(m)).collect())
            }
            Ty::Con(h, values, args) => {
                let rows = self.s.rows(h, args.len());
                Ty::Con(
                    h,
                    values,
                    args.into_iter()
                        .zip(rows)
                        .map(|(a, row)| {
                            if row != Mode::Value {
                                self.env(a)
                            } else {
                                self.norm(a)
                            }
                        })
                        .collect(),
                )
            }
            Ty::Fn(f) => {
                let f = *f;
                Ty::Fn(Box::new(FnTy {
                    params: f
                        .params
                        .into_iter()
                        .map(|(l, p)| (l, self.norm(p)))
                        .collect(),
                    ret: self.norm(f.ret),
                    err: self.env(f.err),
                    env: self.env(f.env),
                }))
            }
            Ty::Both(a, b) => {
                let (a, b) = (self.norm(*a), self.norm(*b));
                if a == b {
                    a
                } else if same_unmarked(&a, &b) {
                    // Equal but for marks: the side with the marks.
                    if b == Ty::Top { a } else { b }
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
            Ty::Both(a, b) => {
                let a = self.env(*a);
                let b = self.env(*b);
                self.norm(Ty::Both(Box::new(a), Box::new(b)))
            }
            Ty::Diff(a, hs) => {
                self.diff(self.env(*a), hs.into_iter().map(|h| self.norm(h)).collect())
            }
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
        if !env {
            flat = super::solve::join_literals(flat);
        }
        let mut out: Vec<Ty> = Vec::new();
        for m in flat {
            if m == Ty::Never || out.contains(&m) {
                continue;
            }
            if is_top(&m) {
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
        if out.iter().any(|m| match m {
            Ty::Diff(a, h) => {
                is_top(a)
                    && h.iter().all(|x| {
                        out.iter().any(|u| {
                            u.head() == x.head() && self.full(u) && x.values().subset(u.values())
                        })
                    })
            }
            _ => false,
        }) {
            return Ty::Top;
        }
        if env {
            out.sort_by_cached_key(|t| match t.head() {
                Some(Head::Named(crate::hir::TypeRef::Custom(id))) => (
                    1,
                    self.s
                        .type_names
                        .get(id.0 as usize)
                        .cloned()
                        .unwrap_or_default(),
                ),
                Some(Head::Named(crate::hir::TypeRef::Builtin(b))) => (1, b.name().to_string()),
                Some(_) => (2, String::new()),
                None => (0, String::new()),
            });
            let mut ranks = Vec::new();
            for member in out.iter().filter(|m| m.head().is_some()) {
                visit(member, &mut |t| {
                    if let Ty::Var(v) = t
                        && !ranks.contains(v)
                    {
                        ranks.push(*v);
                    }
                });
            }
            out.sort_by_key(|t| match t {
                Ty::Var(v) => (0, ranks.iter().position(|w| w == v).unwrap_or(usize::MAX)),
                t if t.head().is_none() => (0, usize::MAX),
                _ => (1, 0),
            });
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
            Ty::Con(h, _, args) => {
                let vs = self.s.variances(*h, args.len());
                args.iter().zip(vs).all(|(a, v)| match v {
                    Variance::Co => is_top(a),
                    Variance::Contra => *a == Ty::Never,
                    Variance::Bi => true,
                    Variance::Inv => false,
                })
            }
            Ty::Fn(f) => {
                f.params.iter().all(|p| p.1 == Ty::Never)
                    && [&f.ret, &f.err, &f.env].iter().all(|x| is_top(x))
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
            if is_top(&m) || out.contains(&m) {
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
                        if out[i] == Ty::Never {
                            return Ty::Never;
                        }
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
            Ty::Con(..) if a.scalar().is_some() => {
                let sets: Vec<_> = hs
                    .iter()
                    .filter(|m| m.head() == a.head())
                    .map(|m| m.values().clone())
                    .collect();
                if sets.is_empty() {
                    a
                } else {
                    a.with_values(a.values().minus(&super::lits::ValueSet::join_all(&sets)))
                }
            }
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
            (Ty::Con(h, values, xs), Ty::Con(_, other, ys)) if xs.len() == ys.len() => {
                let values = if union {
                    values.union(&other)
                } else {
                    let Some(values) = values.meet(&other) else {
                        return Ty::Never;
                    };
                    values
                };
                let vs = self.s.variances(h, xs.len());
                Ty::Con(
                    h,
                    values,
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
        let mut occ: HashMap<V, Occ> = HashMap::default();
        let mut order = Vec::new();
        self.walk(t, true, &mut occ, &mut order, false);
        // A handler's arguments constrain input variables even when a
        // result annotation hides that relation. Do not turn an otherwise
        // output-only row into an input merely because it excludes itself.
        let mut handler_inputs = Vec::new();
        visit(t, &mut |part| {
            if let Ty::Diff(a, hs) = part
                && self.s.is_row(a)
            {
                for h in hs {
                    visit(h, &mut |x| {
                        if let Ty::Var(v) = x
                            && occ.get(v).is_some_and(|o| o.neg > 0 && o.pos == 0)
                        {
                            handler_inputs.push(*v);
                        }
                    });
                }
            }
        });
        for v in handler_inputs {
            record(&mut occ, &mut order, v, true, Vec::new());
        }
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
                o.pos_with.as_deref().unwrap_or_default(),
                o.neg_with.as_deref().unwrap_or_default(),
            );
            for w in pw.iter().filter(|w| nw.contains(w)) {
                match w {
                    Ty::Var(x) if x == v => {}
                    Ty::Var(x) if named && !self.named(*x) => return Some((*x, Ty::Var(*v))),
                    Ty::Var(_) => return Some((*v, (**w).clone())),
                    _ if !mentions(w, *v) => return Some((*v, (**w).clone())),
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

    /// Invariant arguments count on both sides (§4.5 rules 6 and 7), even
    /// when normalization has collapsed their equal `Both` forms. In an
    /// environment, every argument is invariant (§4.5 rule 4).
    fn walk<'a>(
        &self,
        t: &'a Ty,
        pos: bool,
        occ: &mut HashMap<V, Occ<'a>>,
        order: &mut Vec<V>,
        env: bool,
    ) {
        // Only a union in an output place and an intersection in an input
        // place say that a variable comes with the other members.
        // This walk only reads the group. Cloning the enclosing tree at
        // each recursive step costs quadratic work on nested types.
        let (group, together): (&[Ty], bool) = match t {
            Ty::Union(ms) => (ms, pos),
            Ty::Inter(ms) => (ms, !pos),
            _ => (std::slice::from_ref(t), true),
        };
        for (i, m) in group.iter().enumerate() {
            match m {
                Ty::Var(v) => {
                    let with: Vec<&Ty> = if together {
                        group
                            .iter()
                            .enumerate()
                            .filter(|(j, _)| *j != i)
                            .map(|(_, x)| x)
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
                Ty::Con(h, _, args) => {
                    let vs = self.s.variances(*h, args.len());
                    for ((a, v), row) in args.iter().zip(vs).zip(self.s.rows(*h, args.len())) {
                        match v {
                            _ if env || v == Variance::Inv => {
                                self.walk(a, true, occ, order, row != Mode::Value);
                                self.walk(a, false, occ, order, row != Mode::Value);
                            }
                            Variance::Contra => self.walk(a, !pos, occ, order, row != Mode::Value),
                            _ => self.walk(a, pos, occ, order, row != Mode::Value),
                        }
                    }
                }
                Ty::Fn(f) => {
                    for (_, p) in &f.params {
                        self.walk(p, !pos, occ, order, false);
                    }
                    self.walk(&f.ret, pos, occ, order, false);
                    self.walk(&f.err, pos, occ, order, true);
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

fn record<'a>(
    occ: &mut HashMap<V, Occ<'a>>,
    order: &mut Vec<V>,
    v: V,
    pos: bool,
    with: Vec<&'a Ty>,
) {
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
        Ty::Con(_, _, xs) | Ty::Union(xs) | Ty::Inter(xs) => xs.iter().for_each(|x| visit(x, f)),
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
        Ty::Con(h, values, xs) => Ty::Con(*h, values.clone(), xs.iter().map(r).collect()),
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

struct Namer {
    names: HashMap<V, String>,
    taken: HashSet<String>,
}

impl Namer {
    fn new(s: &Solver) -> Namer {
        let taken = s
            .type_names
            .iter()
            .filter(|n| !n.contains('.'))
            .cloned()
            .chain(
                crate::hir::BuiltinType::ALL
                    .into_iter()
                    .map(|b| b.name().to_string()),
            )
            .collect();
        Namer {
            names: HashMap::default(),
            taken,
        }
    }

    fn show(mut self, t: &Ty) -> ShownType {
        let t = prepare(t, &mut HashMap::default());
        self.assign(&t);
        self.tree(&t)
    }

    /// A name for each variable of `t` (made by [`prepare`]) that has none.
    fn assign(&mut self, t: &Ty) {
        let mut order = Vec::new();
        roles(t, Role::Value, &mut order);
        for (v, role) in &order {
            if self.names.contains_key(v) {
                continue;
            }
            let n = self.fresh(*role);
            self.names.insert(*v, n);
        }
    }

    fn tree(&self, t: &Ty) -> ShownType {
        tree(t, &mut |v| {
            self.names.get(&v).cloned().unwrap_or_else(|| "_".into())
        })
    }

    fn fresh(&mut self, role: Role) -> String {
        let candidates: Box<dyn Iterator<Item = String>> = match role {
            Role::Err => {
                Box::new(std::iter::once("E".to_string()).chain((2..).map(|i| format!("E{i}"))))
            }
            Role::Env => {
                Box::new(std::iter::once("N".to_string()).chain((2..).map(|i| format!("N{i}"))))
            }
            Role::Value => Box::new(
                ('A'..='Z')
                    .filter(|c| *c != 'E' && *c != 'N')
                    .map(String::from)
                    .chain((2..).flat_map(|i| {
                        ('A'..='Z')
                            .filter(|c| *c != 'E' && *c != 'N')
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

/// `t` as its variables are named: `Top` is a variable that has no effect,
/// and each gets its own variable, so its own name. The variable of a mark
/// (which is `Top`) is in `origin` with the variable that the mark is of.
fn prepare(t: &Ty, origin: &mut HashMap<V, V>) -> Ty {
    let mut next = u32::MAX;
    tops(&holes(&both(t)), &mut next, origin)
}

fn tops(t: &Ty, next: &mut u32, origin: &mut HashMap<V, V>) -> Ty {
    let mut go = |x: &Ty| tops(x, next, origin);
    match t {
        Ty::Top => {
            *next -= 1;
            Ty::Var(V(*next))
        }
        Ty::Var(m) if m.0 >= MARK => {
            *next -= 1;
            origin.insert(V(*next), V(m.0 - MARK));
            Ty::Var(V(*next))
        }
        Ty::Con(h, values, xs) => Ty::Con(*h, values.clone(), xs.iter().map(go).collect()),
        Ty::Union(xs) => Ty::Union(xs.iter().map(go).collect()),
        Ty::Inter(xs) => Ty::Inter(xs.iter().map(go).collect()),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f.params.iter().map(|(l, p)| (l.clone(), go(p))).collect(),
            ret: go(&f.ret),
            err: go(&f.err),
            env: go(&f.env),
        })),
        // A part of `H` with no effect is written `_`.
        Ty::Diff(a, h) => Ty::Diff(Box::new(go(a)), h.clone()),
        Ty::Both(a, b) => {
            let a = go(a);
            Ty::Both(Box::new(a), Box::new(go(b)))
        }
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
        Ty::Con(_, _, xs) => xs.iter().for_each(|x| roles(x, Role::Value, out)),
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

/// The tree of a shown type: `Both` is its side that is shown, `Top` is
/// `_`, and each variable has the name that `name` gives it.
pub(crate) fn tree(t: &Ty, name: &mut dyn FnMut(V) -> String) -> ShownType {
    tree_with(t, &mut |v| ShownType::Var(name(v)))
}

pub(crate) fn tree_with(t: &Ty, name: &mut dyn FnMut(V) -> ShownType) -> ShownType {
    let all = |xs: &[Ty], name: &mut dyn FnMut(V) -> ShownType| -> Vec<ShownType> {
        xs.iter().map(|x| tree_with(x, name)).collect()
    };
    match t {
        Ty::Var(v) => name(*v),
        Ty::Never => ShownType::Never,
        Ty::Top => ShownType::Hole,
        Ty::Con(Head::Named(r), values, _) if !values.is_all() => {
            let mut items: Vec<_> = values.items().map(|v| v.to_string()).collect();
            if *r == crate::hir::TypeRef::Builtin(crate::hir::BuiltinType::Int) {
                items.sort_by_cached_key(|v| {
                    v.parse::<num_bigint::BigInt>().expect("canonical integer")
                });
            }
            let items: Vec<_> = items
                .into_iter()
                .map(|v| {
                    ShownType::Lit(
                        if *r == crate::hir::TypeRef::Builtin(crate::hir::BuiltinType::String) {
                            ilang_syntax::print::string_literal(&v)
                        } else {
                            v
                        },
                    )
                })
                .collect();
            if values.is_finite() {
                if items.len() == 1 {
                    items.into_iter().next().unwrap()
                } else {
                    ShownType::Union(items)
                }
            } else {
                ShownType::Diff(Box::new(ShownType::Named(*r, Vec::new())), items)
            }
        }
        Ty::Con(Head::Named(r), _, args) => ShownType::Named(*r, all(args, name)),
        Ty::Con(_, _, parts) => ShownType::Tuple(all(parts, name)),
        Ty::Fn(f) => {
            let params = f
                .params
                .iter()
                .map(|(l, p)| (l.clone(), tree_with(p, name)))
                .collect();
            let ret = tree_with(&f.ret, name);
            let err = (f.err != Ty::Never).then(|| tree_with(&f.err, name));
            let env = (f.env != Ty::Never).then(|| tree_with(&f.env, name));
            ShownType::Fn(Box::new(ShownFn {
                params,
                ret,
                err,
                env,
            }))
        }
        Ty::Union(ms) => ShownType::Union(flat_shown(all(ms, name))),
        Ty::Inter(ms) => ShownType::Inter(all(ms, name)),
        Ty::Diff(a, h) => ShownType::Diff(Box::new(tree_with(a, name)), flat_shown(all(h, name))),
        Ty::Both(a, b) => {
            if **a == Ty::Never {
                tree_with(b, name)
            } else {
                tree_with(a, name)
            }
        }
    }
}

/// The text of a type (§3), with the names of the custom types `tn`.
fn render(t: &Ty, name: &mut dyn FnMut(V) -> String, tn: &[String]) -> String {
    tree(t, name).text_limited(tn, 12)
}

/// `t` with the variables of `H` in each difference that are nowhere else
/// changed to `Top`, shown `_`: only the type constructors of `H` matter
/// there (§4.4), and such a variable has no other effect.
fn holes(t: &Ty) -> Ty {
    let mut outside = HashSet::default();
    outer_vars(t, &mut outside);
    hide(t, &outside, false)
}

fn outer_vars(t: &Ty, out: &mut HashSet<V>) {
    match t {
        Ty::Var(v) => {
            out.insert(*v);
        }
        Ty::Con(_, _, xs) | Ty::Union(xs) | Ty::Inter(xs) => {
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
        Ty::Var(v) if in_h && (v.0 >= MARK || !outside.contains(v)) => Ty::Top,
        Ty::Con(h, values, xs) => Ty::Con(*h, values.clone(), xs.iter().map(go).collect()),
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

/// `known`, but with the variables that are not general kept, for the
/// matching that gives them types (§4.8).
pub(crate) fn known_keep(s: &Solver, t: &Ty, level: u32) -> Ty {
    match simplify(s, t, Some(level)) {
        Ok(t) => known_norm(s, &both(&t)),
        Err(()) => Ty::Top,
    }
}

/// The known type of a name of another module (§4.8): its type scheme in
/// the simpler form of §4.7, where a variable that is not generalized is
/// the union of the types that come to it.
pub(crate) fn known_shown(s: &Solver, t: &Ty) -> Ty {
    let opts = Opts {
        mono: true,
        written: false,
    };
    match shown(s, t, opts) {
        Ok(t) => known_norm(s, &both(&t)),
        Err(()) => Ty::Top,
    }
}

fn not_general(s: &Solver, t: &Ty, level: u32) -> Ty {
    let go = |x: &Ty| not_general(s, x, level);
    match t {
        Ty::Var(v) if s.vars.get(v.0 as usize).is_none_or(|x| x.level <= level) => Ty::Top,
        Ty::Con(h, values, xs) => Ty::Con(*h, values.clone(), xs.iter().map(go).collect()),
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
    let mut names: HashMap<V, String> = HashMap::default();
    let mut next = 0u8;
    render(
        t,
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

fn flat_shown(ts: Vec<ShownType>) -> Vec<ShownType> {
    ts.into_iter()
        .flat_map(|t| match t {
            ShownType::Union(ms) => ms,
            t => vec![t],
        })
        .collect()
}

#[cfg(test)]
mod literal_display_tests {
    use super::super::lits::ValueSet;
    use super::*;
    use crate::hir::{BuiltinType, TypeRef};
    use std::sync::Arc;
    fn lit(b: BuiltinType, xs: &[&str]) -> Ty {
        Ty::lit(b, ValueSet::only(xs.iter().map(|s| Arc::from(*s))).unwrap())
    }
    #[test]
    fn set_normalization_and_full_text() {
        // literal-types §5.7: subset-based Top, exact differences and number ordering.
        let s = Solver::new(Default::default(), Vec::new());
        let a = lit(BuiltinType::String, &["a"]);
        assert_eq!(
            known_union(
                &s,
                vec![a.clone(), Ty::Diff(Box::new(Ty::Top), vec![a.clone()])]
            ),
            Ty::Top
        );
        let string = Ty::named(TypeRef::Builtin(BuiltinType::String), Vec::new());
        assert_ne!(
            known_union(
                &s,
                vec![a.clone(), Ty::Diff(Box::new(Ty::Top), vec![string.clone()])]
            ),
            Ty::Top
        );
        let t = known_diff(&s, string, vec![lit(BuiltinType::String, &["b", "a"])]);
        assert_eq!(
            scheme(&s, &t, Opts::default()).unwrap().text(&[]),
            "String - \"a\" - \"b\""
        );
        let t = lit(BuiltinType::Int, &["10", "-2", "1"]);
        assert_eq!(
            scheme(&s, &t, Opts::default()).unwrap().text(&[]),
            "-2 | 1 | 10"
        );
        assert_eq!(
            known_inter(&s, vec![a, lit(BuiltinType::String, &["b"])]),
            Ty::Never
        );
        let xs: Vec<_> = (0..40).map(|i| Arc::from(format!("v{i:02}"))).collect();
        let t = Ty::lit(BuiltinType::String, ValueSet::only(xs).unwrap());
        let tree = scheme(&s, &t, Opts::default()).unwrap();
        assert_eq!(tree.text(&[]).matches('|').count(), 39);
        assert!(tree.text_limited(&[], 12).ends_with("… (30 more)"));
        assert!(!tree.text(&[]).contains('…'));
    }
}
