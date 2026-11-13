//! What patterns match: "can not fail" (§7.3), "taken" (§4.8 rule 12),
//! covering every value and "never used" (§7.4), all with the usefulness
//! algorithm of Maranget ("Warnings for pattern matching", JFP 2007).
//!
//! A pattern vector `q` is **useful** for a matrix of rows when some values
//! are matched by `q` and by no row. The columns may have types: then only
//! the values of the type are looked at. A column with no type (`Col::Any`)
//! may have any value, of any type constructor.
//!
//! The signatures, by type constructor (`Head`):
//! - a custom type: its constructors, each with its fields (§7.1);
//! - a tuple of n parts: one constructor;
//! - `List`: the empty list, and a cons of an element and the rest;
//! - every other head (literals, string prefixes, bit arrays, functions,
//!   types with no constructors): no finite signature;
//! - a type test `x: T`: every value of the heads of `T`;
//! - a type with a type variable as a member has values of heads that no
//!   pattern names: the "other heads" of a column.

use super::infer::Checker;
use super::lits::ValueSet;
use super::solve::Solver;
use super::{Head, Ty, V};
use crate::hir::*;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

/// A literal pattern, a string prefix, or a bit array pattern. Each matches
/// some values of a head with no finite signature.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Lit {
    /// A refutable predicate on an opaque value. Its overlap with another
    /// predicate cannot be inferred by the coverage checker.
    Opaque(TypeId),
    /// An integer, as its decimal text with its sign; `-0` is `0` (§11.4).
    Int(String),
    /// The bits of a float; `-0.0` is `0.0` (§11.4).
    Float(u64),
    Str(String),
    /// `"lit" <> r`: the strings that start with `lit`.
    Prefix(String, String),
    /// A bit array pattern (§8.4). It is never known to match all the
    /// values of another pattern.
    Bits,
}

fn affix_matches(value: &str, prefix: &str, suffix: &str) -> bool {
    value.len() >= prefix.len() + suffix.len()
        && value.starts_with(prefix)
        && value.ends_with(suffix)
}

/// A constructor in the sense of Maranget: a part of a signature.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Ctor {
    Custom(CtorRef),
    Shape(super::ShapeId),
    Nil,
    Cons,
    Lit(Lit),
}

#[derive(Clone, Debug)]
pub(crate) enum Pat {
    /// A catch-all. The number names it for narrowing (rule 12).
    Wild(Option<u32>),
    /// A constructor, with its parts in the order of its fields.
    Ctor(Ctor, Vec<Pat>),
    /// Every value of one type constructor, with its number of arguments
    /// (parameters for a function type): a member of `T` in `x: T`.
    Head(Head, usize),
    /// Choices `p1 | p2`, and the members of a union in a type test.
    Or(Vec<Pat>),
    /// A cofinite scalar set (items are excluded values).
    Except(Head, ValueSet),
}

/// A row that names only finitely many top-level scalar values.
pub(crate) fn scalar_literal_row(p: &Pat) -> bool {
    match p {
        Pat::Ctor(Ctor::Lit(Lit::Int(_) | Lit::Str(_)), args) => args.is_empty(),
        Pat::Or(ps) => ps.iter().all(scalar_literal_row),
        _ => false,
    }
}

/// Index for a prefix containing only top-level Int/String literals.
/// All other rows retain the ordinary matrix algorithm. A literal prefix
/// removes exactly these finite sets from the subject, without constraints.
#[derive(Default)]
pub(crate) struct ScalarRows {
    ints: rustc_hash::FxHashSet<String>,
    strings: rustc_hash::FxHashSet<String>,
    closed: bool,
}

impl ScalarRows {
    fn choices(p: &Pat, out: &mut Vec<Lit>) -> bool {
        match p {
            Pat::Ctor(Ctor::Lit(lit @ (Lit::Int(_) | Lit::Str(_))), args) if args.is_empty() => {
                out.push(lit.clone());
                true
            }
            Pat::Or(alts) => alts.iter().all(|p| Self::choices(p, out)),
            _ => false,
        }
    }

    fn contains(&self, lit: &Lit) -> bool {
        match lit {
            Lit::Int(v) => self.ints.contains(v),
            Lit::Str(v) => self.strings.contains(v),
            _ => false,
        }
    }

    pub fn add(&mut self, p: &Pat) -> bool {
        fn wild(p: &Pat) -> bool {
            match p {
                Pat::Wild(_) => true,
                Pat::Or(ps) => ps.iter().any(wild),
                _ => false,
            }
        }
        if self.closed || wild(p) {
            self.closed = true;
            return true;
        }
        let mut lits = Vec::new();
        if !Self::choices(p, &mut lits) {
            return false;
        }
        for lit in lits {
            match lit {
                Lit::Int(v) => {
                    self.ints.insert(v);
                }
                Lit::Str(v) => {
                    self.strings.insert(v);
                }
                _ => unreachable!(),
            }
        }
        true
    }

    fn rest(&self, subject: &Ty) -> Ty {
        let mut excluded = Vec::new();
        for (b, items) in [
            (BuiltinType::Int, &self.ints),
            (BuiltinType::String, &self.strings),
        ] {
            if let Some(set) =
                ValueSet::only(items.iter().map(|v| std::sync::Arc::from(v.as_str())))
            {
                excluded.push(Ty::lit(b, set));
            }
        }
        if excluded.is_empty() {
            subject.clone()
        } else {
            Ty::Diff(Box::new(subject.clone()), excluded)
        }
    }
}

/// The values a column of the matrix may have.
#[derive(Clone, Debug)]
pub(crate) enum Col {
    /// Any value.
    Any,
    /// The values of a type, found from its lower bounds (`Pos`) or from its
    /// upper bounds (`Up`); see `values`.
    Ty(Ty, Mode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Mode {
    Pos,
    Up,
}

impl Col {
    /// The values of the type of an expression.
    pub fn of(t: Ty) -> Col {
        Col::Ty(t, Mode::Pos)
    }
}

/// The values of a type, by type constructor: the heads, each with its
/// arguments and how their values are found, and whether the type has
/// members whose heads are not known.
#[derive(Clone, Debug, Default)]
struct Vals {
    heads: Vec<(Head, ValueSet, Vec<Ty>, Mode)>,
    other: bool,
}

impl Vals {
    fn any() -> Vals {
        Vals {
            heads: Vec::new(),
            other: true,
        }
    }

    fn has(&self, h: Head) -> bool {
        self.other || self.heads.iter().any(|x| x.0 == h)
    }

    fn is_empty(&self) -> bool {
        !self.other && self.heads.is_empty()
    }

    fn union(mut self, o: Vals) -> Vals {
        for (h, set, args, m) in o.heads {
            match self.heads.iter_mut().find(|x| x.0 == h) {
                Some(x) if x.2.len() == args.len() => {
                    x.1 = x.1.union(&set);
                    for (a, b) in x.2.iter_mut().zip(args) {
                        *a = Ty::Union(vec![a.clone(), b]);
                    }
                    if m != x.3 {
                        x.3 = Mode::Up;
                    }
                }
                Some(_) => {}
                None => self.heads.push((h, set, args, m)),
            }
        }
        self.other |= o.other;
        self
    }

    fn inter(self, o: Vals) -> Vals {
        if self.other {
            return o;
        }
        if o.other {
            return self;
        }
        let mut heads = Vec::new();
        for (h, set, args, m) in &self.heads {
            // An unknown alternative can contain all values of this head.
            let other = o.heads.iter().find(|x| x.0 == *h);
            if o.other {
                heads.push((*h, set.clone(), args.clone(), *m));
            } else if let Some(x) = other
                && let Some(set) = set.meet(&x.1)
            {
                let args = if args.len() == x.2.len() {
                    args.iter()
                        .zip(&x.2)
                        .map(|(a, b)| Ty::Inter(vec![a.clone(), b.clone()]))
                        .collect()
                } else {
                    args.clone()
                };
                heads.push((*h, set, args, if *m == x.3 { *m } else { Mode::Up }));
            }
        }
        if self.other {
            for x in &o.heads {
                if !self.heads.iter().any(|y| y.0 == x.0) {
                    heads.push(x.clone());
                } else if let Some(y) = heads.iter_mut().find(|y| y.0 == x.0) {
                    y.1 = x.1.clone();
                }
            }
        }
        Vals {
            heads,
            other: self.other && o.other,
        }
    }
}

/// Values of variables, each with the variables that its search reached.
type Found<R> = HashMap<(V, Mode), (Vals, R)>;

/// What the searches of `values` found that holds as long as the bounds of
/// the variables stay as they are. One match makes many searches over the
/// same variables.
#[derive(Default)]
pub(crate) struct Searches {
    /// The bounds the results were found with: the solver's variables, their
    /// number, and the numbers of bounds added and of rewrites.
    stamp: (usize, usize, usize, u64),
    /// For each variable and mode looked at, the variables that its search
    /// reaches, or `None` when the search is not closed.
    regions: HashMap<(V, Mode), Option<HashSet<V>>>,
    /// The values of the variables whose search is closed, with the
    /// variables that it reached.
    memo: Found<Vec<(V, Mode)>>,
    /// The values of the variables whose search did not depend on its path,
    /// with the variables it reached: they hold wherever none of those is on
    /// the path.
    done: Found<HashSet<(V, Mode)>>,
}

/// One search of `values`: the variables being looked at.
struct Walk<'s> {
    s: &'s Solver,
    found: &'s mut Searches,
    seen: Vec<(V, Mode)>,
    /// The number of times the search depended on its path: it met a
    /// variable of the path again.
    cuts: u64,
    /// The variables reached, in order.
    trail: Vec<(V, Mode)>,
}

/// The values of a type; see [`Walk::values`].
fn values(s: &Solver, t: &Ty, mode: Mode, found: &mut Searches) -> Vals {
    let stamp = (
        s.vars.as_ptr() as usize,
        s.vars.len(),
        s.touched.len(),
        s.rewrites,
    );
    if found.stamp != stamp {
        *found = Searches {
            stamp,
            ..Searches::default()
        };
    }
    Walk {
        s,
        found,
        seen: Vec::new(),
        cuts: 0,
        trail: Vec::new(),
    }
    .values(t, mode)
}

/// The variables whose values the values of `t` come from.
fn searched_vars(t: &Ty, out: &mut Vec<V>) {
    match t {
        Ty::Var(v) => out.push(*v),
        Ty::Union(ms) | Ty::Inter(ms) => ms.iter().for_each(|m| searched_vars(m, out)),
        Ty::Diff(a, _) => searched_vars(a, out),
        Ty::Con(..) | Ty::Fn(_) | Ty::Never | Ty::Top | Ty::Both(..) => {}
    }
}

impl Walk<'_> {
    /// The values of `t`.
    ///
    /// In `Pos` mode, the values of a type variable are those of its lower
    /// bounds, and of the variables that have it as an upper bound: the values
    /// that come to it in the constraints. A variable whose
    /// values may also come from outside (an input: a parameter, a written type
    /// variable, the result of a function that is not known) also has every
    /// value that its upper bounds allow (`Up` mode): every value that comes to
    /// it is a subtype of them, and with no bound with a head, it may be any
    /// value (§7.3).
    ///
    /// A variable met again on the path of the search adds nothing: no values
    /// in `Pos` mode, and no restriction in `Up` mode.
    fn values(&mut self, t: &Ty, mode: Mode) -> Vals {
        match t {
            Ty::Var(v) => {
                if self.seen.contains(&(*v, mode)) {
                    self.cuts += 1;
                    return match mode {
                        Mode::Pos => Vals::default(),
                        Mode::Up => Vals::any(),
                    };
                }
                if let Some((vals, reached)) = self.found.done.get(&(*v, mode))
                    && !self.seen.iter().any(|x| reached.contains(x))
                {
                    let vals = vals.clone();
                    self.trail.extend(reached.iter().copied());
                    return vals;
                }
                if self.closed(*v, mode) {
                    return self.closed_values(*v, mode);
                }
                let (cuts, start) = (self.cuts, self.trail.len());
                self.trail.push((*v, mode));
                self.seen.push((*v, mode));
                let out = self.bounds(*v, mode, &mut |w, t, mode| w.values(t, mode));
                self.seen.pop();
                if self.cuts == cuts {
                    // No variable of the path was met again: the values do
                    // not depend on the path, only on the variables reached
                    // not being on it.
                    let reached = self.trail[start..].iter().copied().collect();
                    self.found.done.insert((*v, mode), (out.clone(), reached));
                }
                out
            }
            Ty::Con(h, set, args) => Vals {
                heads: vec![(*h, set.clone(), args.clone(), mode)],
                other: false,
            },
            // The parameter type stands for the one argument of the head.
            Ty::Fn(f) => Vals {
                heads: vec![(Head::Fn, ValueSet::ALL, vec![f.param.clone()], mode)],
                other: false,
            },
            Ty::Union(ms) => ms
                .iter()
                .fold(Vals::default(), |acc, m| acc.union(self.values(m, mode))),
            Ty::Inter(ms) => ms
                .iter()
                .fold(Vals::any(), |acc, m| acc.inter(self.values(m, mode))),
            Ty::Diff(a, h) => {
                let mut v = self.values(a, mode);
                for excluded in h {
                    v.heads = v
                        .heads
                        .into_iter()
                        .filter_map(|(head, set, args, mode)| {
                            if excluded.head() != Some(head) {
                                Some((head, set, args, mode))
                            } else {
                                set.minus(excluded.values())
                                    .map(|set| (head, set, args, mode))
                            }
                        })
                        .collect();
                }
                v
            }
            Ty::Never => Vals::default(),
            Ty::Top | Ty::Both(..) => Vals::any(),
        }
    }

    /// The values of the variable `v` from its bounds, each bound's values
    /// found by `part`.
    fn bounds(
        &mut self,
        v: V,
        mode: Mode,
        part: &mut dyn FnMut(&mut Self, &Ty, Mode) -> Vals,
    ) -> Vals {
        let st = &self.s.vars[v.0 as usize];
        match mode {
            Mode::Pos => {
                let mut acc = Vals::default();
                for l in st.lower.clone() {
                    acc = acc.union(part(self, &l, Mode::Pos));
                }
                for w in st.from.clone() {
                    acc = acc.union(part(self, &Ty::Var(w), Mode::Pos));
                }
                if st.input {
                    // `Up` mode never comes back to `Pos` mode, so the
                    // variables on the path of the search do not matter.
                    acc = acc.union(self.values(&Ty::Var(v), Mode::Up));
                }
                acc
            }
            Mode::Up => {
                let mut acc = Vals::any();
                for u in st.upper.clone() {
                    acc = acc.inter(part(self, &u, Mode::Up));
                }
                acc
            }
        }
    }

    /// Whether the search from `v` is closed and meets no variable on the
    /// path of the search. A search is closed when, from variable to
    /// variable, it only goes through the operation of its mode (union in
    /// `Pos` mode, intersection in `Up` mode), and every other part of a
    /// bound (such as a union in `Up` mode) leads to a search that never
    /// comes back to those variables. Then a variable met again adds
    /// nothing that it did not add the first time: the values are those of
    /// every bound reached, put together in the order they are first
    /// reached, whatever the paths. Without this, a dense graph of variables
    /// (a recursive match over many constructors) would be searched along
    /// each of its paths, whose number is exponential.
    fn closed(&mut self, v: V, mode: Mode) -> bool {
        if !self.found.regions.contains_key(&(v, mode)) {
            let region = self.region(v, mode);
            self.found.regions.insert((v, mode), region);
        }
        match &self.found.regions[&(v, mode)] {
            Some(region) => {
                let free = !self
                    .seen
                    .iter()
                    .any(|(x, m)| *m == mode && region.contains(x));
                // Whether the search is closed here depends on the path.
                self.cuts += u64::from(!free);
                free
            }
            None => false,
        }
    }

    /// Every variable that the search from `v` reaches, when it is closed.
    fn region(&self, v: V, mode: Mode) -> Option<HashSet<V>> {
        let mut region = HashSet::default();
        let mut todo = vec![v];
        let mut parts = Vec::new();
        while let Some(x) = todo.pop() {
            if !region.insert(x) {
                continue;
            }
            let st = &self.s.vars[x.0 as usize];
            let bounds = match mode {
                Mode::Pos => &st.lower,
                Mode::Up => &st.upper,
            };
            for b in bounds {
                self.closed_part(b, mode, &mut todo, &mut parts);
            }
            if mode == Mode::Pos {
                todo.extend(st.from.iter().copied());
            }
        }
        // The searches of the other parts, which must not come back.
        let mut reached = HashSet::default();
        while let Some(x) = parts.pop() {
            if region.contains(&x) {
                return None;
            }
            if !reached.insert(x) {
                continue;
            }
            let st = &self.s.vars[x.0 as usize];
            let bounds = match mode {
                Mode::Pos => &st.lower,
                Mode::Up => &st.upper,
            };
            for b in bounds {
                searched_vars(b, &mut parts);
            }
            if mode == Mode::Pos {
                parts.extend(st.from.iter().copied());
            }
        }
        region.extend(reached);
        Some(region)
    }

    /// Adds the variables of a bound that the search goes to through the
    /// operation of its mode to `todo`, and the others to `parts`.
    fn closed_part(&self, t: &Ty, mode: Mode, todo: &mut Vec<V>, parts: &mut Vec<V>) {
        match t {
            Ty::Var(v) => todo.push(*v),
            Ty::Union(ms) if mode == Mode::Pos => ms
                .iter()
                .for_each(|m| self.closed_part(m, mode, todo, parts)),
            Ty::Inter(ms) if mode == Mode::Up => ms
                .iter()
                .for_each(|m| self.closed_part(m, mode, todo, parts)),
            t => searched_vars(t, parts),
        }
    }

    /// The values of `v` when its search is closed: each variable reached is
    /// looked at once.
    fn closed_values(&mut self, v: V, mode: Mode) -> Vals {
        if let Some((vals, reached)) = self.found.memo.get(&(v, mode)) {
            let vals = vals.clone();
            self.trail.extend(reached.iter().copied());
            return vals;
        }
        let start = self.trail.len();
        let region = self.found.regions[&(v, mode)]
            .as_ref()
            .expect("closed search");
        self.trail.extend(region.iter().map(|x| (*x, mode)));
        let mut visited = HashSet::default();
        let out = self.closed_var(v, mode, &mut visited);
        let reached = self.trail[start..].to_vec();
        self.found.memo.insert((v, mode), (out.clone(), reached));
        out
    }

    fn closed_var(&mut self, v: V, mode: Mode, visited: &mut HashSet<V>) -> Vals {
        if !visited.insert(v) {
            return match mode {
                Mode::Pos => Vals::default(),
                Mode::Up => Vals::any(),
            };
        }
        self.bounds(v, mode, &mut |w, t, mode| w.closed_ty(t, mode, visited))
    }

    fn closed_ty(&mut self, t: &Ty, mode: Mode, visited: &mut HashSet<V>) -> Vals {
        match t {
            Ty::Var(v) => self.closed_var(*v, mode, visited),
            Ty::Union(ms) if mode == Mode::Pos => ms.iter().fold(Vals::default(), |acc, m| {
                acc.union(self.closed_ty(m, mode, visited))
            }),
            Ty::Inter(ms) if mode == Mode::Up => ms.iter().fold(Vals::any(), |acc, m| {
                acc.inter(self.closed_ty(m, mode, visited))
            }),
            t => self.values(t, mode),
        }
    }
}

impl Ctor {
    pub fn head(&self) -> Head {
        match self {
            Ctor::Custom(c) => Head::Named(TypeRef::Custom(c.ty)),
            Ctor::Shape(n) => Head::Shape(*n),
            Ctor::Nil | Ctor::Cons => Head::Named(TypeRef::Builtin(BuiltinType::List)),
            Ctor::Lit(Lit::Opaque(ty)) => Head::Named(TypeRef::Custom(*ty)),
            Ctor::Lit(l) => Head::Named(TypeRef::Builtin(match l {
                Lit::Int(_) => BuiltinType::Int,
                Lit::Float(_) => BuiltinType::Float,
                Lit::Str(_) | Lit::Prefix(_, _) => BuiltinType::String,
                Lit::Bits => BuiltinType::BitArray,
                Lit::Opaque(_) => unreachable!(),
            })),
        }
    }

    fn arity(&self, p: &Program) -> usize {
        match self {
            Ctor::Custom(c) => match &p.ty(c.ty).body {
                TypeBody::Custom(cs) => cs.get(c.index as usize).map_or(0, |x| x.fields.len()),
                TypeBody::Alias(_) => 0,
            },
            Ctor::Shape(n) => n.len(),
            Ctor::Cons => 2,
            Ctor::Nil | Ctor::Lit(_) => 0,
        }
    }

    /// A row pattern `self` matches every value of `c`.
    fn covers(&self, c: &Ctor) -> bool {
        match (self, c) {
            (Ctor::Lit(Lit::Bits | Lit::Opaque(_)), _) => false,
            (Ctor::Lit(Lit::Prefix(p, z)), Ctor::Lit(Lit::Str(s))) => affix_matches(s, p, z),
            (Ctor::Lit(Lit::Prefix(p, z)), Ctor::Lit(Lit::Prefix(q, w))) => {
                q.starts_with(p.as_str()) && w.ends_with(z.as_str())
            }
            (a, b) => a == b,
        }
    }
}

/// The patterns of a row matched against every value of the head `h`: a
/// catch-all, a type test of `h`, or a constructor of `h`.
fn of_head(p: &Pat, h: Head) -> bool {
    match p {
        Pat::Wild(_) => true,
        Pat::Head(x, _) | Pat::Except(x, _) => *x == h,
        Pat::Ctor(c, _) => c.head() == h,
        Pat::Or(_) => false,
    }
}

/// The rows with the choices in their first column made into rows of their
/// own.
fn expand(rows: &[Vec<Pat>]) -> Vec<Vec<Pat>> {
    let mut out = Vec::new();
    for r in rows {
        match r.first() {
            Some(Pat::Or(alts)) => {
                let more: Vec<Vec<Pat>> = alts
                    .iter()
                    .map(|a| {
                        let mut x = vec![a.clone()];
                        x.extend_from_slice(&r[1..]);
                        x
                    })
                    .collect();
                out.extend(expand(&more));
            }
            _ => out.push(r.clone()),
        }
    }
    out
}

/// A copy of `p` with the catch-all named `id` changed to `by`.
pub(crate) fn replace(p: &Pat, id: u32, by: &Pat) -> Pat {
    match p {
        Pat::Wild(Some(x)) if *x == id => by.clone(),
        Pat::Ctor(c, args) => {
            Pat::Ctor(c.clone(), args.iter().map(|a| replace(a, id, by)).collect())
        }
        Pat::Or(alts) => Pat::Or(alts.iter().map(|a| replace(a, id, by)).collect()),
        p => p.clone(),
    }
}

impl Checker<'_> {
    /// Usefulness after a scalar prefix, with the same full-row semantics.
    pub(crate) fn scalar_row_useful(
        &mut self,
        rows: &ScalarRows,
        q: &Pat,
        subject: &Ty,
        matrix: &[Vec<Pat>],
    ) -> bool {
        if rows.closed {
            return false;
        }
        let mut lits = Vec::new();
        if ScalarRows::choices(q, &mut lits) {
            let col = Col::of(subject.clone());
            lits.into_iter()
                .any(|lit| !rows.contains(&lit) && self.sub_cols(&col, &Ctor::Lit(lit)).is_some())
        } else if matches!(q, Pat::Wild(_)) {
            self.useful(&[], std::slice::from_ref(q), &[Col::of(rows.rest(subject))])
                .is_some()
        } else {
            // Prefixes, complements and structural tests retain the full
            // matrix, including its conservative overlap decisions.
            self.useful(matrix, std::slice::from_ref(q), &[Col::of(subject.clone())])
                .is_some()
        }
    }

    pub(crate) fn scalar_row_missing(
        &mut self,
        rows: &ScalarRows,
        subject: &Ty,
    ) -> Option<Vec<Pat>> {
        if rows.closed {
            None
        } else {
            self.useful(&[], &[Pat::Wild(None)], &[Col::of(rows.rest(subject))])
        }
    }

    /// A witness of usefulness: a vector of patterns, one for each column,
    /// that matches values that no row matches. `None` when `q` is not
    /// useful.
    pub(crate) fn useful(
        &mut self,
        rows: &[Vec<Pat>],
        q: &[Pat],
        cols: &[Col],
    ) -> Option<Vec<Pat>> {
        if q.is_empty() {
            return rows.is_empty().then(Vec::new);
        }
        let rows = expand(rows);
        match &q[0] {
            Pat::Or(alts) => alts.iter().find_map(|a| {
                let mut q2 = vec![a.clone()];
                q2.extend_from_slice(&q[1..]);
                self.useful(&rows, &q2, cols)
            }),
            Pat::Ctor(c, args) => self.useful_ctor(&rows, c, args, &q[1..], cols),
            Pat::Except(h, set) => self.useful_scalar(&rows, *h, &q[1..], cols, Some(set)),
            Pat::Head(h, n) => {
                if !self.col_values(&cols[0]).has(*h) {
                    return None;
                }
                self.useful_head(&rows, *h, *n, &q[1..], cols)
            }
            Pat::Wild(_) => {
                let vals = self.col_values(&cols[0]);
                if vals.other {
                    // A value of a head that no row names: only the
                    // catch-alls match it.
                    let rest: Vec<Vec<Pat>> = rows
                        .iter()
                        .filter(|r| matches!(r[0], Pat::Wild(_)))
                        .map(|r| r[1..].to_vec())
                        .collect();
                    let mut w = self.useful(&rest, &q[1..], &cols[1..])?;
                    w.insert(0, Pat::Wild(None));
                    return Some(w);
                }
                let one = vals.heads.len() == 1;
                for (h, _, args, _) in vals.heads {
                    let n = args.len();
                    if let Some(mut w) = self.useful_head(&rows, h, n, &q[1..], cols) {
                        if one && matches!(w[0], Pat::Head(..)) {
                            w[0] = Pat::Wild(None);
                        }
                        return Some(w);
                    }
                }
                None
            }
        }
    }

    fn useful_ctor(
        &mut self,
        rows: &[Vec<Pat>],
        c: &Ctor,
        args: &[Pat],
        rest_q: &[Pat],
        cols: &[Col],
    ) -> Option<Vec<Pat>> {
        let sub = self.sub_cols(&cols[0], c)?;
        let k = c.arity(self.p);
        let h = c.head();
        let mut spec = Vec::new();
        for r in rows {
            let mut x = match &r[0] {
                Pat::Wild(_) => vec![Pat::Wild(None); k],
                Pat::Head(x, _) if *x == h => vec![Pat::Wild(None); k],
                Pat::Except(x, set)
                    if *x == h
                        && match c {
                            Ctor::Lit(Lit::Int(v) | Lit::Str(v)) => set.contains(v),
                            Ctor::Lit(Lit::Prefix(p, z)) => {
                                !set.items().any(|v| affix_matches(v, p, z))
                            }
                            _ => false,
                        } =>
                {
                    vec![Pat::Wild(None); k]
                }
                Pat::Ctor(c2, a2) if c2.covers(c) => {
                    if a2.len() == k {
                        a2.clone()
                    } else {
                        vec![Pat::Wild(None); k]
                    }
                }
                _ => continue,
            };
            x.extend_from_slice(&r[1..]);
            spec.push(x);
        }
        let mut q2 = args.to_vec();
        q2.resize(k, Pat::Wild(None));
        q2.extend_from_slice(rest_q);
        let mut cols2 = sub;
        cols2.extend_from_slice(&cols[1..]);
        let w = self.useful(&spec, &q2, &cols2)?;
        let mut out = vec![Pat::Ctor(c.clone(), w[..k].to_vec())];
        out.extend_from_slice(&w[k..]);
        Some(out)
    }

    /// `q` with every value of the head `h` in its first column.
    fn useful_head(
        &mut self,
        rows: &[Vec<Pat>],
        h: Head,
        n: usize,
        rest_q: &[Pat],
        cols: &[Col],
    ) -> Option<Vec<Pat>> {
        if scalar_head(h).is_some() {
            return self.useful_scalar(rows, h, rest_q, cols, None);
        }
        let rows: Vec<Vec<Pat>> = rows.iter().filter(|r| of_head(&r[0], h)).cloned().collect();
        let default = |all_strings: bool| -> Vec<Vec<Pat>> {
            rows.iter()
                .filter(|r| match &r[0] {
                    Pat::Wild(_) | Pat::Head(..) => true,
                    // `"" <> r` matches every string.
                    Pat::Ctor(Ctor::Lit(Lit::Prefix(p, z)), _) => {
                        all_strings && p.is_empty() && z.is_empty()
                    }
                    _ => false,
                })
                .map(|r| r[1..].to_vec())
                .collect()
        };
        let Some(sig) = self.signature(h, &cols[0]) else {
            let d = default(true);
            let mut w = self.useful(&d, rest_q, &cols[1..])?;
            w.insert(0, Pat::Head(h, n));
            return Some(w);
        };
        let present: Vec<&Ctor> = rows
            .iter()
            .filter_map(|r| match &r[0] {
                Pat::Ctor(c, _) => Some(c),
                _ => None,
            })
            .collect();
        let missing = sig
            .iter()
            .find(|c| !present.iter().any(|p| p.covers(c)))
            .cloned();
        match missing {
            None => sig
                .iter()
                .find_map(|c| self.useful_ctor(&rows, c, &[], rest_q, cols)),
            Some(m) => {
                let d = default(false);
                let mut w = self.useful(&d, rest_q, &cols[1..])?;
                let first = if present.is_empty() {
                    Pat::Head(h, n)
                } else {
                    let k = m.arity(self.p);
                    Pat::Ctor(m, vec![Pat::Wild(None); k])
                };
                w.insert(0, first);
                Some(w)
            }
        }
    }

    /// Listed values need their own specialization; cofinite rows only cover the default.
    fn useful_scalar(
        &mut self,
        rows: &[Vec<Pat>],
        h: Head,
        rest_q: &[Pat],
        cols: &[Col],
        query: Option<&ValueSet>,
    ) -> Option<Vec<Pat>> {
        let vals = self.col_values(&cols[0]);
        let set = if vals.other {
            ValueSet::ALL
        } else {
            vals.heads.iter().find(|x| x.0 == h)?.1.clone()
        };
        let set = match query {
            Some(q) => set.meet(q)?,
            None => set,
        };
        let rows: Vec<_> = rows.iter().filter(|r| of_head(&r[0], h)).cloned().collect();
        let mut listed = std::collections::BTreeSet::<String>::new();
        for r in &rows {
            match &r[0] {
                Pat::Ctor(Ctor::Lit(Lit::Int(v) | Lit::Str(v)), _) => {
                    listed.insert(v.clone());
                }
                Pat::Except(_, s) => listed.extend(s.items().map(|v| v.to_string())),
                _ => {}
            }
        }
        listed.extend(set.items().map(|v| v.to_string()));
        let is_int = scalar_head(h) == Some(BuiltinType::Int);
        // Int display/witness ordering is numeric, not canonical-text order.
        let mut items: Vec<_> = listed.iter().filter(|v| set.contains(v)).cloned().collect();
        if is_int {
            items.sort_by_key(|s| s.parse::<num_bigint::BigInt>().unwrap());
        }
        for v in items {
            let c = Ctor::Lit(if is_int { Lit::Int(v) } else { Lit::Str(v) });
            if let Some(w) = self.useful_ctor(&rows, &c, &[], rest_q, cols) {
                return Some(w);
            }
        }
        if set.is_finite() {
            return None;
        }
        let default: Vec<_> = rows
            .iter()
            .filter(|r| match &r[0] {
                Pat::Wild(_) | Pat::Head(..) | Pat::Except(..) => true,
                Pat::Ctor(Ctor::Lit(Lit::Prefix(p, z)), _) => p.is_empty() && z.is_empty(),
                _ => false,
            })
            .map(|r| r[1..].to_vec())
            .collect();
        let mut w = self.useful(&default, rest_q, &cols[1..])?;
        let v = if is_int {
            let mut n = 0usize;
            loop {
                let v = n.to_string();
                if set.contains(&v) && !listed.contains(&v) {
                    break v;
                }
                n += 1;
            }
        } else if set.contains("") && !listed.contains("") {
            String::new()
        } else {
            let prefixes: Vec<_> = rows
                .iter()
                .filter_map(|r| match &r[0] {
                    Pat::Ctor(Ctor::Lit(Lit::Prefix(p, z)), _)
                        if !p.is_empty() || !z.is_empty() =>
                    {
                        Some((p, z))
                    }
                    _ => None,
                })
                .collect();
            let c = ('a'..='z')
                .chain('0'..='9')
                .chain((123..=0x10ffff).filter_map(char::from_u32))
                .find(|c| {
                    !prefixes.iter().any(|(p, z)| {
                        if p.is_empty() {
                            z.ends_with(*c)
                        } else {
                            p.starts_with(*c)
                        }
                    })
                })
                .unwrap();
            let mut v = c.to_string();
            while !set.contains(&v) || listed.contains(&v) {
                v.push(c);
            }
            v
        };
        w.insert(
            0,
            Pat::Ctor(
                Ctor::Lit(if is_int { Lit::Int(v) } else { Lit::Str(v) }),
                Vec::new(),
            ),
        );
        Some(w)
    }

    /// Whether the values of `t` are known and some have a head other than
    /// `h`, and whether then none has `h`.
    pub(crate) fn made_by_others(&self, t: &Ty, h: Head) -> (bool, bool) {
        let vals = self.col_values(&Col::of(t.clone()));
        let others = !vals.other && vals.heads.iter().any(|x| x.0 != h);
        (others, others && !vals.has(h))
    }

    fn col_values(&self, col: &Col) -> Vals {
        match col {
            Col::Any => Vals::any(),
            Col::Ty(t, mode) => values(&self.s, t, *mode, &mut self.coverage.borrow_mut()),
        }
    }

    /// The columns of the parts of `c`, in a column `col`. `None` when the
    /// column has no values made by `c`.
    fn sub_cols(&mut self, col: &Col, c: &Ctor) -> Option<Vec<Col>> {
        let k = c.arity(self.p);
        if matches!(col, Col::Any) {
            return Some(vec![Col::Any; k]);
        }
        let vals = self.col_values(col);
        let Some((_, set, args, mode)) = vals.heads.iter().find(|x| x.0 == c.head()) else {
            return vals.other.then(|| vec![Col::Any; k]);
        };
        if !vals.other
            && let Ctor::Lit(lit) = c
        {
            let possible = match lit {
                Lit::Int(v) | Lit::Str(v) => set.contains(v),
                Lit::Prefix(p, z) => {
                    !set.is_finite() || set.items().any(|v| affix_matches(v, p, z))
                }
                _ => true,
            };
            if !possible {
                return None;
            }
        }
        let (args, mode) = (args.clone(), *mode);
        let parts: Vec<Ty> = match c {
            Ctor::Custom(r) => {
                if args.len() != self.p.ty(r.ty).params.len() {
                    return Some(vec![Col::Any; k]);
                }
                self.ctor_fields(*r, &args)
                    .into_iter()
                    .map(|f| f.1)
                    .collect()
            }
            Ctor::Shape(_) => args,
            Ctor::Cons => {
                let e = args.first().cloned().unwrap_or(Ty::Top);
                let list = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![e.clone()]);
                vec![e, list]
            }
            Ctor::Nil | Ctor::Lit(_) => Vec::new(),
        };
        if parts.len() != k {
            return Some(vec![Col::Any; k]);
        }
        Some(parts.into_iter().map(|t| Col::Ty(t, mode)).collect())
    }

    /// The constructors of a head with a finite signature, with values in
    /// the column: a constructor with a field of no values has none.
    fn signature(&mut self, h: Head, col: &Col) -> Option<Vec<Ctor>> {
        if let Head::Named(TypeRef::Builtin(b @ (BuiltinType::Int | BuiltinType::String))) = h {
            let vals = self.col_values(col);
            if !vals.other
                && let Some((_, set, _, _)) = vals.heads.iter().find(|x| x.0 == h)
                && set.is_finite()
            {
                return Some(
                    set.items()
                        .map(|v| {
                            Ctor::Lit(if b == BuiltinType::Int {
                                Lit::Int(v.to_string())
                            } else {
                                Lit::Str(v.to_string())
                            })
                        })
                        .collect(),
                );
            }
        }
        let all = match h {
            Head::Named(TypeRef::Custom(id)) => match &self.p.ty(id).body {
                TypeBody::Custom(cs) if !cs.is_empty() => (0..cs.len())
                    .map(|i| {
                        Ctor::Custom(CtorRef {
                            ty: id,
                            index: i as u32,
                        })
                    })
                    .collect(),
                _ => return None,
            },
            Head::Named(TypeRef::Builtin(BuiltinType::List)) => vec![Ctor::Nil, Ctor::Cons],
            Head::Shape(n) => vec![Ctor::Shape(n)],
            _ => return None,
        };
        if matches!(col, Col::Any) {
            return Some(all);
        }
        let mut out = Vec::new();
        for c in all {
            let inhabited = self
                .sub_cols(col, &c)
                .is_some_and(|cols| cols.iter().all(|x| !self.col_values(x).is_empty()));
            if inhabited {
                out.push(c);
            }
        }
        Some(out)
    }

    /// Explain whole scalar inference at a shared place, using the actual witness.
    pub(crate) fn shared_hint(&mut self, witness: &Pat, rows: &[Vec<Pat>]) -> Option<String> {
        fn literals(p: &Pat, b: BuiltinType, out: &mut Vec<String>) -> bool {
            match p {
                Pat::Wild(_) => true,
                Pat::Or(ps) => ps.iter().all(|p| literals(p, b, out)),
                Pat::Ctor(Ctor::Lit(Lit::Int(n)), _) if b == BuiltinType::Int => {
                    out.push(n.clone());
                    true
                }
                Pat::Ctor(Ctor::Lit(Lit::Str(s)), _) if b == BuiltinType::String => {
                    out.push(ilang_syntax::print::string_literal(s));
                    true
                }
                _ => false,
            }
        }
        fn children(p: &Pat, c: &Ctor, i: usize, out: &mut Vec<Pat>) {
            match p {
                Pat::Wild(_) => out.push(Pat::Wild(None)),
                Pat::Or(ps) => {
                    for p in ps {
                        children(p, c, i, out);
                    }
                }
                Pat::Ctor(d, parts) if c == d => {
                    if let Some(p) = parts.get(i) {
                        out.push(p.clone());
                    }
                }
                _ => {}
            }
        }
        fn go(
            ch: &mut Checker<'_>,
            w: &Pat,
            ps: &[Pat],
            shared: Option<(&str, &[Pat])>,
        ) -> Option<String> {
            if let Pat::Ctor(Ctor::Lit(l), _) = w {
                let b = match l {
                    Lit::Int(_) => BuiltinType::Int,
                    Lit::Str(_) => BuiltinType::String,
                    _ => return None,
                };
                if let Some((owner, candidates)) = shared {
                    let mut ls = Vec::new();
                    if candidates.iter().all(|p| literals(p, b, &mut ls)) && !ls.is_empty() {
                        ls.sort();
                        ls.dedup();
                        let whole = b.name();
                        let example = format!("{owner}({})", ls.join(" | "));
                        let start = if owner == "List" {
                            format!(
                                "the items of a list get the whole type `{whole}` from literal patterns (§4.8)"
                            )
                        } else {
                            format!(
                                "the fields of `{owner}` share one type, so they get the whole type `{whole}` from literal patterns"
                            )
                        };
                        return Some(format!(
                            "{start}; if only these values can come, write the type, such as `{example}`"
                        ));
                    }
                }
            }
            let Pat::Ctor(c, args) = w else { return None };
            let mut family = Vec::new();
            let owner = match c {
                Ctor::Cons => Some("List".to_string()),
                Ctor::Custom(r) => {
                    let TypeBody::Custom(cs) = &ch.p.ty(r.ty).body else {
                        return None;
                    };
                    for (ci, ctor) in cs.iter().enumerate() {
                        let cr = CtorRef {
                            ty: r.ty,
                            index: ci as u32,
                        };
                        for i in 0..ctor.fields.len() {
                            if ch.field_shared(cr, i) {
                                for p in ps {
                                    children(p, &Ctor::Custom(cr), i, &mut family);
                                }
                            }
                        }
                    }
                    Some(ch.p.ty(r.ty).name.clone())
                }
                _ => None,
            };
            for (i, w) in args.iter().enumerate() {
                let mut next = Vec::new();
                for p in ps {
                    children(p, c, i, &mut next);
                }
                let here = match c {
                    Ctor::Cons if i == 0 => Some(("List", next.as_slice())),
                    Ctor::Custom(r) if ch.field_shared(*r, i) => {
                        Some((owner.as_deref().unwrap(), family.as_slice()))
                    }
                    _ => shared,
                };
                if let Some(h) = go(ch, w, &next, here) {
                    return Some(h);
                }
            }
            None
        }
        let ps: Vec<_> = rows.iter().filter_map(|r| r.first().cloned()).collect();
        go(self, witness, &ps, None)
    }

    // ---- Text ----

    /// The text of a witness, in the syntax of patterns (§7.1).
    pub(crate) fn show_pat(&self, p: &Pat) -> String {
        match p {
            Pat::Wild(_) => "_".into(),
            Pat::Or(alts) => alts
                .iter()
                .map(|a| self.show_pat(a))
                .collect::<Vec<_>>()
                .join(" | "),
            Pat::Except(h, set) => {
                let t = Ty::lit(scalar_head(*h).unwrap(), set.clone());
                format!("_ : {}", super::display::show_known(&self.s, &t))
            }
            Pat::Head(h, n) => format!("_ : {}", self.show_head(*h, *n)),
            Pat::Ctor(Ctor::Shape(shape), args) => {
                let parts: Vec<String> = args.iter().map(|a| self.show_pat(a)).collect();
                shape.text(&parts)
            }
            Pat::Ctor(Ctor::Nil, _) => "[]".into(),
            Pat::Ctor(Ctor::Cons, _) => {
                let mut items = Vec::new();
                let mut cur = p;
                let end = loop {
                    match cur {
                        Pat::Ctor(Ctor::Cons, args) if args.len() == 2 => {
                            items.push(self.show_pat(&args[0]));
                            cur = &args[1];
                        }
                        Pat::Ctor(Ctor::Nil, _) => break None,
                        _ => break Some(".."),
                    }
                };
                items.extend(end.map(String::from));
                format!("[{}]", items.join(", "))
            }
            Pat::Ctor(Ctor::Lit(l), _) => match l {
                Lit::Int(s) => s.clone(),
                Lit::Float(b) => format!("{:?}", f64::from_bits(*b)),
                Lit::Str(s) => ilang_syntax::print::string_literal(s),
                Lit::Prefix(p, z) => {
                    let start = if p.is_empty() && !z.is_empty() {
                        "_".into()
                    } else {
                        format!("{} <> _", ilang_syntax::print::string_literal(p))
                    };
                    if z.is_empty() {
                        start
                    } else {
                        format!("{start} <> {}", ilang_syntax::print::string_literal(z))
                    }
                }
                Lit::Bits => "<<..>>".into(),
                Lit::Opaque(_) => "a syntax pattern".into(),
            },
            Pat::Ctor(Ctor::Custom(c), args) => {
                let TypeBody::Custom(cs) = &self.p.ty(c.ty).body else {
                    return "_".into();
                };
                let def = &cs[c.index as usize];
                if def.fields.is_empty() {
                    return def.name.clone();
                }
                let mut parts = Vec::new();
                let mut spread = false;
                for (f, a) in def.fields.iter().zip(args) {
                    match &f.label {
                        None => parts.push(self.show_pat(a)),
                        Some(_) if matches!(a, Pat::Wild(_)) => spread = true,
                        Some(l) => parts.push(format!("{} = {}", l.name, self.show_pat(a))),
                    }
                }
                if spread {
                    parts.push("..".into());
                }
                format!("{}({})", def.name, parts.join(", "))
            }
        }
    }

    fn show_head(&self, h: Head, n: usize) -> String {
        let holes = |n: usize| vec!["_"; n].join(", ");
        match h {
            Head::Named(TypeRef::Builtin(b)) if n == 0 => b.name().to_string(),
            Head::Named(TypeRef::Builtin(b)) => format!("{}({})", b.name(), holes(n)),
            Head::Named(TypeRef::Custom(id)) => {
                let name = &self.p.ty(id).name;
                if n == 0 {
                    name.clone()
                } else {
                    format!("{name}({})", holes(n))
                }
            }
            Head::Shape(k) => format!("{{ {} }}", k.text(&vec!["_".to_string(); k.len()])),
            Head::Fn => format!("{{ ({}) -> _ }}", holes(n)),
        }
    }
}

/// The parts of a constructor pattern in the order of the fields of `c`
/// (§7.1): by position to the fields without labels, then by label. Parts
/// that match no field were reported when the pattern was typed.
pub(crate) fn field_order<'p>(
    p: &Program,
    c: CtorRef,
    args: &'p [Pattern],
) -> Vec<Option<&'p Pattern>> {
    let TypeBody::Custom(cs) = &p.ty(c.ty).body else {
        return Vec::new();
    };
    let Some(def) = cs.get(c.index as usize) else {
        return Vec::new();
    };
    let mut out: Vec<Option<&Pattern>> = vec![None; def.fields.len()];
    let unlabeled: Vec<usize> = (0..def.fields.len())
        .filter(|i| def.fields[*i].label.is_none())
        .collect();
    for (j, a) in args.iter().enumerate() {
        let i = if j < unlabeled.len() {
            Some(unlabeled[j])
        } else {
            super::pattern::part_label(a).and_then(|l| {
                def.fields
                    .iter()
                    .position(|f| f.label.as_ref().is_some_and(|x| x.name == l))
            })
        };
        if let Some(i) = i
            && out[i].is_none()
        {
            out[i] = Some(a);
        }
    }
    out
}

/// The literal of a pattern, when it is one.
pub(crate) fn literal(q: &Pattern) -> Option<Lit> {
    Some(match &q.kind {
        PatternKind::Quote { syntax, .. } => Lit::Opaque(*syntax),
        PatternKind::Int { negative, value } => Lit::Int(super::lits::int_text(*negative, value)),
        PatternKind::Float { negative, value } => {
            let v = if *negative { -*value } else { *value };
            Lit::Float(if v == 0.0 { 0 } else { v.to_bits() })
        }
        PatternKind::String(s) => Lit::Str(s.clone()),
        PatternKind::Prefix { prefix, suffix, .. } => Lit::Prefix(prefix.clone(), suffix.clone()),
        PatternKind::BitArray(_) => Lit::Bits,
        _ => return None,
    })
}

/// A list pattern as constructors: `[p1, p2]` is `Cons(p1, Cons(p2, Nil))`,
/// and `[p1, ..r]` is `Cons(p1, _)`.
pub(crate) fn list(items: Vec<Pat>, rest: bool) -> Pat {
    let mut tail = if rest {
        Pat::Wild(None)
    } else {
        Pat::Ctor(Ctor::Nil, Vec::new())
    };
    for x in items.into_iter().rev() {
        tail = Pat::Ctor(Ctor::Cons, vec![x, tail]);
    }
    tail
}

/// A pattern of a binding or a parameter, for "can not fail" (§7.3). `x: T`
/// matches every value that comes to it there, because fitting makes the
/// value a `T` (§4.8), so it is a catch-all here.
pub(crate) fn fit_pat(p: &Program, q: &Pattern) -> Pat {
    if let Some(l) = literal(q) {
        return Pat::Ctor(Ctor::Lit(l), Vec::new());
    }
    match &q.kind {
        PatternKind::Named { pattern, .. } => fit_pat(p, pattern),
        PatternKind::Constructor { ctor, args, .. } => Pat::Ctor(
            Ctor::Custom(*ctor),
            field_order(p, *ctor, args)
                .into_iter()
                .map(|a| a.map_or(Pat::Wild(None), |a| fit_pat(p, a)))
                .collect(),
        ),
        PatternKind::Tuple(ps) => Pat::Ctor(
            Ctor::Shape(super::ShapeId::positional(ps.len() as u32)),
            ps.iter().map(|a| fit_pat(p, a)).collect(),
        ),
        PatternKind::List { items, rest } => list(
            items.iter().map(|a| fit_pat(p, a)).collect(),
            rest.is_some(),
        ),
        _ => Pat::Wild(None),
    }
}

fn scalar_head(h: Head) -> Option<BuiltinType> {
    match h {
        Head::Named(TypeRef::Builtin(b @ (BuiltinType::Int | BuiltinType::String))) => Some(b),
        _ => None,
    }
}

/// A runtime type-test pattern; used by narrowing and coverage together.
pub(crate) fn scalar_pat(t: &Ty) -> Pat {
    let h = t.head().unwrap();
    if t.values().is_all() {
        Pat::Head(h, 0)
    } else if t.values().is_finite() {
        let b = t.scalar().unwrap();
        Pat::Or(
            t.values()
                .items()
                .map(|v| {
                    Pat::Ctor(
                        Ctor::Lit(if b == BuiltinType::Int {
                            Lit::Int(v.to_string())
                        } else {
                            Lit::Str(v.to_string())
                        }),
                        Vec::new(),
                    )
                })
                .collect(),
        )
    } else {
        Pat::Except(h, t.values().clone())
    }
}

#[cfg(test)]
mod literal_coverage_tests {
    use super::*;
    #[test]
    fn complements_preserve_cross_columns_and_prefixes() {
        // §5.5: excluded values are specialized separately; prefixes cannot
        // be treated as literal strings when testing a cofinite row.
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("i.toml"), "name = \"app\"\n").unwrap();
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(root.path().join("src/main.i"), "x = 1\n").unwrap();
        let (graph, ds) = ilang_package::load(root.path());
        assert!(ds.is_empty());
        let checked = crate::check(&graph);
        let p = checked.program.unwrap();
        let mut ch = Checker::new(&p, &graph.sources);
        let h = Head::Named(TypeRef::Builtin(BuiltinType::String));
        let except = ValueSet::except([std::sync::Arc::from("ab")]);
        let row = vec![Pat::Except(h, except.clone()), Pat::Wild(None)];
        let prefix = Pat::Ctor(
            Ctor::Lit(Lit::Prefix("a".into(), String::new())),
            Vec::new(),
        );
        assert!(
            ch.useful(
                std::slice::from_ref(&row),
                &[prefix, Pat::Wild(None)],
                &[Col::Any, Col::Any]
            )
            .is_some()
        );
        let literal = Pat::Ctor(Ctor::Lit(Lit::Str("ab".into())), Vec::new());
        assert!(
            ch.useful(
                std::slice::from_ref(&row),
                &[literal.clone(), Pat::Wild(None)],
                &[Col::Any, Col::Any]
            )
            .is_some()
        );
        let col = [Col::of(Ty::named(
            TypeRef::Builtin(BuiltinType::String),
            Vec::new(),
        ))];
        assert!(
            ch.useful(
                &[vec![row[0].clone()], vec![literal]],
                &[Pat::Wild(None)],
                &col
            )
            .is_none()
        );
        let q = Pat::Except(h, except);
        assert!(ch.useful(&[vec![row[0].clone()]], &[q], &col).is_none());
    }

    #[test]
    fn scalar_index_agrees_with_the_full_matrix() {
        // §9: an optimization may avoid the matrix, but must make the
        // same usefulness/coverage decisions as the full-row algorithm.
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("i.toml"), "name = \"app\"\n").unwrap();
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(root.path().join("src/main.i"), "x = 1\n").unwrap();
        let (graph, ds) = ilang_package::load(root.path());
        assert!(ds.is_empty());
        let checked = crate::check(&graph);
        let p = checked.program.unwrap();
        let mut ch = Checker::new(&p, &graph.sources);
        let int = Head::Named(TypeRef::Builtin(BuiltinType::Int));
        let string = Head::Named(TypeRef::Builtin(BuiltinType::String));
        let lit = |l| Pat::Ctor(Ctor::Lit(l), Vec::new());
        let prefix = [
            lit(Lit::Int("0".into())),
            lit(Lit::Int("1".into())),
            lit(Lit::Str("a".into())),
            lit(Lit::Str("ab".into())),
        ];
        let mut queries = prefix.to_vec();
        queries.extend([
            lit(Lit::Str("".into())),
            lit(Lit::Prefix("a".into(), String::new())),
            Pat::Head(int, 0),
            Pat::Head(string, 0),
            Pat::Except(string, ValueSet::except([std::sync::Arc::from("a")])),
            Pat::Wild(None),
            Pat::Or(vec![prefix[0].clone(), prefix[2].clone()]),
            Pat::Or(vec![prefix[0].clone(), Pat::Wild(None)]),
            Pat::Ctor(
                Ctor::Shape(crate::types::ShapeId::positional(1)),
                vec![Pat::Wild(None)],
            ),
            lit(Lit::Float(0)),
        ]);
        let inputs = [
            Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new()),
            Ty::named(TypeRef::Builtin(BuiltinType::String), Vec::new()),
            Ty::Union(vec![
                Ty::Con(int, ValueSet::ALL, Vec::new()),
                Ty::Con(string, ValueSet::ALL, Vec::new()),
            ]),
            Ty::lit(
                BuiltinType::Int,
                ValueSet::only([std::sync::Arc::from("0"), std::sync::Arc::from("2")]).unwrap(),
            ),
            Ty::lit(
                BuiltinType::String,
                ValueSet::only([std::sync::Arc::from("a"), std::sync::Arc::from("x")]).unwrap(),
            ),
            Ty::lit(
                BuiltinType::Int,
                ValueSet::except([std::sync::Arc::from("0")]),
            ),
            Ty::lit(
                BuiltinType::String,
                ValueSet::except([std::sync::Arc::from("ab")]),
            ),
            Ty::Top,
            Ty::Never,
            ch.s.fresh_input(),
        ];
        for mask in 0..32 {
            let mut index = ScalarRows::default();
            let mut rows = Vec::new();
            for (i, p) in prefix
                .iter()
                .chain(std::iter::once(&Pat::Wild(None)))
                .enumerate()
            {
                if mask & (1 << i) != 0 {
                    assert!(index.add(p));
                    rows.push(vec![p.clone()]);
                }
            }
            if mask < 16 {
                // Narrowing uses Any columns. A finite scalar prefix
                // leaves values of each whole infinite scalar head.
                assert!(rows.iter().all(|r| scalar_literal_row(&r[0])));
                for h in [int, string] {
                    assert!(ch.useful(&rows, &[Pat::Head(h, 0)], &[Col::Any]).is_some());
                }
            }
            for subject in &inputs {
                let col = [Col::of(subject.clone())];
                for q in &queries {
                    let ordinary = ch.useful(&rows, std::slice::from_ref(q), &col).is_some();
                    assert_eq!(
                        ch.scalar_row_useful(&index, q, subject, &rows),
                        ordinary,
                        "{mask}: {subject:?}, {q:?}"
                    );
                }
                let ordinary = ch.useful(&rows, &[Pat::Wild(None)], &col);
                let fast = ch.scalar_row_missing(&index, subject);
                assert_eq!(
                    format!("{fast:?}"),
                    format!("{ordinary:?}"),
                    "{mask}: {subject:?}"
                );
            }
        }
    }

    #[test]
    fn unknown_alternatives_are_whole_even_beside_an_explicit_head() {
        // literal-types §5.5: a side with other counts as all values of every head.
        let head = Head::Named(TypeRef::Builtin(BuiltinType::Int));
        let set = |v: &str| ValueSet::only([std::sync::Arc::from(v)]).unwrap();
        let a = Vals {
            heads: vec![(head, set("0"), Vec::new(), Mode::Pos)],
            other: true,
        };
        let b = Vals {
            heads: vec![(head, set("1"), Vec::new(), Mode::Pos)],
            other: false,
        };
        assert_eq!(a.clone().inter(b.clone()).heads[0].1, set("1"));
        assert_eq!(b.clone().inter(a).heads[0].1, set("1"));
        let c = Vals {
            heads: vec![(head, set("0"), Vec::new(), Mode::Pos)],
            other: false,
        };
        assert!(c.inter(b).is_empty());
    }
}
