//! Known types and expected types (§4.8).
//!
//! A known type is found from what is written, by a walk over the
//! expression that goes with inference, in its order: groups in the order
//! of their uses, left to right in a binding, and a local binding that is
//! generalized before the rest of its block. It is a `Ty` in which `Top`
//! is `_`, a part that is not known. Its type variables are those of type
//! schemes, of constructors, and of annotations; it never takes part in
//! constraints, so they only stand for the parts that matching fills in.

use super::display;
use super::infer::Checker;
use super::lits::ValueSet;
use super::{FnTy, Head, Ty, V};
use crate::hir::*;
use std::collections::HashMap;

/// The types each type variable gets by matching (§4.8).
pub(super) type Sigma = HashMap<V, Vec<Ty>>;

/// The members of a known type (§4.4): a difference is replaced by its
/// members, and an intersection `a & T` by `T`.
pub(super) fn members(k: &Ty) -> Vec<Ty> {
    match k {
        Ty::Graph(g) if g.record_only() => match g.known_summary() {
            Some(Ty::Never) => Vec::new(),
            Some(summary) => vec![summary],
            None => vec![k.clone()],
        },
        Ty::Graph(g) => match g.view() {
            Some(view) => members(&view),
            None => vec![k.clone()],
        },
        Ty::Union(ms) => ms.iter().flat_map(members).collect(),
        Ty::Diff(a, h) => members(a)
            .into_iter()
            .filter_map(|m| {
                let excluded: Vec<_> = h
                    .iter()
                    .flat_map(members)
                    .filter(|x| x.head() == m.head())
                    .collect();
                if excluded.is_empty() || m.head().is_none() {
                    Some(m)
                } else if m.scalar().is_some() {
                    let sets: Vec<_> = excluded.iter().map(|x| x.values().clone()).collect();
                    m.values()
                        .minus(&super::lits::ValueSet::join_all(&sets))
                        .map(|v| m.with_values(Some(v)))
                } else {
                    None
                }
            })
            .collect(),
        Ty::Inter(ms) => match ms.iter().find(|m| m.head().is_some()) {
            Some(m) => {
                let mut out = m.clone();
                for other in ms.iter().flat_map(members).filter(|x| x.head().is_some()) {
                    if other.head() != out.head() {
                        return Vec::new();
                    }
                    if out.scalar().is_some() {
                        out = out.with_values(out.values().meet(other.values()));
                    }
                    if out == Ty::Never {
                        return Vec::new();
                    }
                }
                vec![out]
            }
            None => vec![k.clone()],
        },
        Ty::Both(_, b) => members(b),
        Ty::Never => Vec::new(),
        t => vec![t.clone()],
    }
}

/// The member of `k` with the type constructor `h`, if there is one.
pub(super) fn member(k: &Ty, h: Option<Head>) -> Option<Ty> {
    let h = h?;
    members(k).into_iter().find(|m| m.head() == Some(h))
}

/// The one member of `k` with the type constructor `h` (§4.8, names in
/// patterns): `None` when there is not exactly one.
pub(super) fn one_member(k: &Ty, h: Head) -> Option<Ty> {
    let mut it = members(k).into_iter().filter(|m| m.head() == Some(h));
    let m = it.next()?;
    it.next().is_none().then_some(m)
}

/// A known function type: `k` has one member, a function type.
pub(super) fn one_fn(k: &Ty) -> Option<FnTy> {
    match members(k).as_slice() {
        [Ty::Fn(f)] => Some((**f).clone()),
        _ => None,
    }
}

/// §6.9: two known types are disjoint (§4.5 rule 3) when every member of
/// one has a different type constructor from every member of the other. A
/// member that is `_` or a type variable is never disjoint.
pub(super) fn disjoint(x: &Ty, y: &Ty) -> bool {
    let (xs, ys) = (members(x), members(y));
    if xs.iter().chain(&ys).any(|m| m.head().is_none()) {
        return false;
    }
    xs.iter().all(|a| {
        ys.iter()
            .all(|b| a.head() != b.head() || a.values().disjoint(b.values()))
    })
}

/// The type at the top of a pattern, through `x = …`: `x: T`.
pub(super) fn top_written(q: &Pattern) -> Option<&Type> {
    match &q.kind {
        PatternKind::Typed { ty, .. } => Some(ty),
        PatternKind::Named { pattern, .. } => top_written(pattern),
        _ => None,
    }
}

/// An argument that is a function expression with a parameter that has no
/// type written at its top: it is left out of the matching that gives the
/// expected types of the other arguments (§4.8).
pub(super) fn open_fn(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Fn(f) => f.params.iter().any(|p| match p {
            Param::Pattern { pattern, .. } => top_written(pattern).is_none(),
            Param::Env { .. } => false,
        }),
        _ => false,
    }
}

impl Checker<'_> {
    pub fn hidden_inputs(&self, f: &FnTy) -> Vec<V> {
        hidden_inputs(&self.s, f)
    }

    pub fn whole_sigma(&self, sigma: &Sigma, hidden: &[V]) -> Sigma {
        sigma
            .iter()
            .map(|(v, ts)| {
                (
                    *v,
                    ts.iter()
                        .map(|t| {
                            if hidden.contains(v) {
                                whole_lits(&self.s, t)
                            } else {
                                t.clone()
                            }
                        })
                        .collect(),
                )
            })
            .collect()
    }

    pub fn kunion(&self, ms: Vec<Ty>) -> Ty {
        display::known_union(&self.s, ms)
    }

    /// A written type as a known type: `_` is `Top`, and a type variable is
    /// the variable of its binding, or `local` for the type parameters of a
    /// definition.
    pub fn kconv(&self, t: &Type, local: &HashMap<TyVarId, Ty>) -> Ty {
        self.kconv_in(t, local, 0)
    }

    fn kconv_in(&self, t: &Type, local: &HashMap<TyVarId, Ty>, depth: u32) -> Ty {
        if depth as usize > self.p.types.len() + 1 {
            return Ty::Top;
        }
        let go = |x: &Type| self.kconv_in(x, local, depth);
        match &t.kind {
            TypeKind::Lit(v) => Ty::literal(v),
            TypeKind::Named { ty, args } => {
                if let TypeRef::Custom(id) = ty
                    && let Some(None) = self.reads.aliases.get(id)
                {
                    return Ty::Top;
                }

                let args: Vec<Ty> = args.iter().map(go).collect();
                if let TypeRef::Custom(id) = ty
                    && let TypeBody::Alias(body) = &self.p.ty(*id).body
                {
                    if let Some(ty) = self.known_alias(*id, args.clone()) {
                        return ty;
                    }
                    let map = self
                        .p
                        .ty(*id)
                        .params
                        .iter()
                        .map(|p| p.1)
                        .zip(args)
                        .collect();
                    return self.kconv_in(body, &map, depth + 1);
                }
                if *ty == TypeRef::Builtin(BuiltinType::Never) {
                    return Ty::Never;
                }
                Ty::named(*ty, args)
            }
            TypeKind::Var(tv) => local
                .get(tv)
                .or_else(|| self.tyvars.get(tv))
                .cloned()
                .unwrap_or(Ty::Top),
            TypeKind::Of { value, site, .. } => self.read_shape(*value, *site),
            TypeKind::Hole | TypeKind::Error => Ty::Top,
            TypeKind::Tuple(ts) => Ty::tuple(ts.iter().map(go).collect()),
            TypeKind::Record {
                base,
                fields,
                removed,
            } => {
                let pool = super::records::Pool::default();
                let labels: Vec<_> = fields
                    .iter()
                    .map(|(name, _)| super::records::written_label(name, self.s.level))
                    .chain(
                        removed
                            .iter()
                            .map(|name| super::records::written_label(name, self.s.level)),
                    )
                    .collect();
                let record = pool.record(
                    fields
                        .iter()
                        .map(|(name, ty)| {
                            (super::records::written_label(name, self.s.level), go(ty))
                        })
                        .collect(),
                );
                match base {
                    Some(base) => {
                        let rest = pool.without(&go(base), labels);
                        pool.inter(&[rest, record])
                    }
                    None => record,
                }
            }
            TypeKind::Fn {
                params,
                ret,
                err,
                env,
            } => Ty::Fn(Box::new(FnTy {
                params: params
                    .iter()
                    .map(|p| (p.label.as_ref().map(|l| l.name.clone()), go(&p.ty)))
                    .collect(),
                ret: go(ret),
                err: err.as_deref().map_or(Ty::Never, go),
                env: env.as_deref().map_or(Ty::Never, go),
            })),
            TypeKind::Union(ts) => self.kunion(ts.iter().map(go).collect()),
            TypeKind::Inter(ts) => display::known_inter(&self.s, ts.iter().map(go).collect()),
            TypeKind::Diff(ts) => {
                let mut a = go(&ts[0]);
                for x in &ts[1..] {
                    a = display::known_diff(&self.s, a, vec![go(x)]);
                }
                a
            }
        }
    }

    /// The known type of a field of `c`, with the arguments of the member
    /// of its type in `args`. With no arguments, a field whose type has a
    /// type parameter is not known.
    pub fn field_known(&self, c: CtorRef, index: usize, args: Option<&[Ty]>) -> Ty {
        let def = self.p.ty(c.ty);
        let TypeBody::Custom(ctors) = &def.body else {
            return Ty::Top;
        };
        let Some(f) = ctors
            .get(c.index as usize)
            .and_then(|x| x.fields.get(index))
        else {
            return Ty::Top;
        };
        match args {
            Some(args) => {
                let map = def.params.iter().map(|p| p.1).zip(args.iter().cloned());
                self.kconv(&f.ty, &map.collect())
            }
            None if def.params.is_empty() => self.kconv(&f.ty, &HashMap::new()),
            None => Ty::Top,
        }
    }

    /// A constructor has its function type, with the field labels (§4.8);
    /// with no fields, it is a value of its type.
    pub fn ctor_known(&mut self, c: CtorRef) -> Ty {
        let def = self.p.ty(c.ty);
        let vars: Vec<Ty> = def.params.iter().map(|_| self.s.fresh()).collect();
        let TypeBody::Custom(ctors) = &def.body else {
            return Ty::Top;
        };
        let n = ctors[c.index as usize].fields.len();
        // `∀a. …`: the variables are generalized (§4.8).
        for v in &vars {
            self.note_general(v);
        }
        let ret = Ty::named(TypeRef::Custom(c.ty), vars.clone());
        if n == 0 {
            // `∀a. T(a)`, in its simpler form (§4.7): a parameter that goes
            // in the same direction is `Never`.
            let vs = self
                .s
                .variances(Head::Named(TypeRef::Custom(c.ty)), vars.len());
            let args = vs
                .iter()
                .map(|v| match v {
                    super::Variance::Co | super::Variance::Bi => Ty::Never,
                    _ => Ty::Top,
                })
                .collect();
            return Ty::named(TypeRef::Custom(c.ty), args);
        }
        let params = (0..n)
            .map(|i| {
                let label = ctors[c.index as usize].fields[i]
                    .label
                    .as_ref()
                    .map(|l| l.name.clone());
                (label, self.field_known(c, i, Some(&vars)))
            })
            .collect();
        Ty::Fn(Box::new(FnTy {
            params,
            ret,
            err: Ty::Never,
            env: Ty::Never,
        }))
    }

    /// A known type as an upper bound for inference (§4.8 rule 13: the type
    /// of `v` is a subtype of its known type). Its parts that are `_` or
    /// type variables are new type variables, because a known type takes
    /// no part in constraints.
    pub fn known_bound(
        &mut self,
        k: &Ty,
        demand: &mut super::records::Demand,
    ) -> Result<Ty, super::records::ResourceLimit> {
        Ok(match k {
            Ty::Graph(g) => g.known_bound(&mut self.s, demand)?,
            Ty::Var(_) | Ty::Top | Ty::Both(..) => self.s.fresh(),
            Ty::Never => Ty::Never,
            Ty::Con(h, values, xs) => {
                let mut args = Vec::with_capacity(xs.len());
                for (x, v) in xs.iter().zip(self.s.variances(*h, xs.len())) {
                    args.push(if v == super::Variance::Co {
                        self.known_bound(x, demand)?
                    } else {
                        self.s.fresh()
                    });
                }
                Ty::Con(
                    *h,
                    if k.scalar().is_some() {
                        ValueSet::ALL
                    } else {
                        values.clone()
                    },
                    args,
                )
            }
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f
                    .params
                    .iter()
                    .map(|(l, _)| (l.clone(), self.s.fresh()))
                    .collect(),
                ret: self.known_bound(&f.ret, demand)?,
                err: self.known_bound(&f.err, demand)?,
                env: self.s.fresh(),
            })),
            Ty::Union(xs) => Ty::Union(
                xs.iter()
                    .map(|x| self.known_bound(x, demand))
                    .collect::<Result<_, _>>()?,
            ),
            Ty::Inter(xs) => Ty::Inter(
                xs.iter()
                    .map(|x| self.known_bound(x, demand))
                    .collect::<Result<_, _>>()?,
            ),
            Ty::Diff(a, h)
                if h.iter()
                    .any(|t| t.scalar().is_some() && !t.values().is_all()) =>
            {
                self.known_bound(a, demand)?
            }
            Ty::Diff(a, h) => Ty::Diff(Box::new(self.known_bound(a, demand)?), h.clone()),
        })
    }

    /// Notes the type variables of `t` as variables of a type scheme that
    /// are generalized: in a known type, they stand for any type, so they
    /// match nothing (§4.8).
    pub fn note_general(&mut self, t: &Ty) {
        match t {
            Ty::Graph(g) => self.kgen.extend(g.variables()),
            Ty::Var(v) => {
                self.kgen.insert(*v);
            }
            Ty::Con(_, _, xs) | Ty::Union(xs) | Ty::Inter(xs) => {
                xs.iter().for_each(|x| self.note_general(x))
            }
            Ty::Fn(f) => {
                f.params.iter().for_each(|p| self.note_general(&p.1));
                [&f.ret, &f.err, &f.env]
                    .into_iter()
                    .for_each(|x| self.note_general(x));
            }
            Ty::Diff(a, h) => {
                self.note_general(a);
                h.iter().for_each(|x| self.note_general(x));
            }
            Ty::Both(a, b) => {
                self.note_general(a);
                self.note_general(b);
            }
            Ty::Never | Ty::Top => {}
        }
    }

    /// The known type of a name with the type scheme `t`, whose variables
    /// deeper than `level` are generalized (§4.8): the scheme in its simpler
    /// form, where a variable that is not generalized is replaced by the
    /// matching part of `value`, the known type of the value of the
    /// binding, or else by `_`.
    pub fn scheme_known(&mut self, t: &Ty, level: u32, value: &Ty) -> Ty {
        let simple = display::known_keep(&self.s, t, level);
        let general = |c: &Self, v: &V| c.s.vars.get(v.0 as usize).is_some_and(|x| x.level > level);
        let mut sigma = Sigma::new();
        self.match_k(&simple, value, &mut sigma, 0);
        sigma.retain(|v, _| !general(self, v));
        let mut vars = Vec::new();
        ty_vars(&simple, &mut vars);
        for v in vars {
            if general(self, &v) {
                self.kgen.insert(v);
                sigma.insert(v, vec![Ty::Var(v)]);
            }
        }
        self.ksubst(&simple, &sigma, true).unwrap_or(Ty::Top)
    }

    /// The known type of a name of another module (§4.8): its type scheme,
    /// in its simpler form.
    pub fn other_known(&mut self, t: &Ty) -> Ty {
        let k = display::known_shown(&self.s, t);
        self.note_general(&k);
        k
    }

    /// Matches a type `p` with a known type `k` (§4.8), adding to `sigma`
    /// the types its type variables get. `_` in `k`, and a type variable
    /// that is generalized in `k`, match nothing.
    pub fn match_k(&self, p: &Ty, k: &Ty, sigma: &mut Sigma, depth: u32) {
        if depth > 64 {
            return;
        }
        if *k == Ty::Top || matches!(k, Ty::Var(v) if self.kgen.contains(v)) {
            let mut vars = Vec::new();
            ty_vars(p, &mut vars);
            for v in vars {
                sigma.entry(v).or_default().push(Ty::Top);
            }
            return;
        }
        match p {
            Ty::Var(v) => sigma.entry(*v).or_default().push(k.clone()),
            Ty::Con(..) | Ty::Fn(_) => match (p, member(k, p.head())) {
                (Ty::Con(_, _, ps), Some(Ty::Con(_, _, ks))) if ps.len() == ks.len() => {
                    for (x, y) in ps.iter().zip(&ks) {
                        self.match_k(x, y, sigma, depth + 1);
                    }
                }
                (Ty::Fn(f), Some(Ty::Fn(g))) if f.params.len() == g.params.len() => {
                    for (x, y) in f.params.iter().zip(&g.params) {
                        self.match_k(&x.1, &y.1, sigma, depth + 1);
                    }
                    self.match_k(&f.ret, &g.ret, sigma, depth + 1);
                    self.match_k(&f.err, &g.err, sigma, depth + 1);
                    self.match_k(&f.env, &g.env, sigma, depth + 1);
                }
                _ => {}
            },
            Ty::Union(ps) => {
                let concrete: Vec<Ty> = ps.iter().filter(|t| t.head().is_some()).cloned().collect();
                for m in ps {
                    if m.head().is_some() {
                        self.match_k(m, k, sigma, depth + 1);
                    } else if let Ty::Var(v) = m {
                        let rest = display::known_diff(&self.s, k.clone(), concrete.clone());
                        sigma.entry(*v).or_default().push(rest);
                    }
                }
            }
            Ty::Inter(ps) => {
                for m in ps {
                    self.match_k(m, k, sigma, depth + 1);
                }
            }
            Ty::Diff(a, _) => self.match_k(a, k, sigma, depth + 1),
            _ => {}
        }
    }

    /// `t` with the types of its type variables from `sigma` put in. A
    /// variable that got no type makes the result not known, or, when
    /// `partial`, is `_`.
    pub fn ksubst(&self, t: &Ty, sigma: &Sigma, partial: bool) -> Option<Ty> {
        let t = self.ksubst_in(t, sigma, partial)?;
        Some(display::known_norm(&self.s, &t))
    }

    fn ksubst_in(&self, t: &Ty, sigma: &Sigma, partial: bool) -> Option<Ty> {
        let go = |x: &Ty| self.ksubst_in(x, sigma, partial);
        let all = |xs: &[Ty]| xs.iter().map(go).collect::<Option<Vec<Ty>>>();
        Some(match t {
            Ty::Graph(g) => g.try_map_vars(&mut |v| go(&Ty::Var(v)).ok_or(())).ok()?,
            Ty::Var(v) => match sigma.get(v) {
                Some(ts) => self.kunion(ts.clone()),
                None if partial => Ty::Top,
                None => return None,
            },
            Ty::Con(h, values, xs) => Ty::Con(*h, values.clone(), all(xs)?),
            Ty::Union(xs) => Ty::Union(all(xs)?),
            Ty::Inter(xs) => Ty::Inter(all(xs)?),
            Ty::Diff(a, h) => Ty::Diff(Box::new(go(a)?), all(h)?),
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f
                    .params
                    .iter()
                    .map(|(l, p)| Some((l.clone(), go(p)?)))
                    .collect::<Option<_>>()?,
                ret: go(&f.ret)?,
                err: go(&f.err)?,
                env: go(&f.env)?,
            })),
            Ty::Both(a, b) => Ty::Both(Box::new(go(a)?), Box::new(go(b)?)),
            Ty::Never | Ty::Top => t.clone(),
        })
    }

    /// The known type of a call of a function with the known type `f`, with
    /// arguments of the known types `args`, in the order of the parameters
    /// (§4.8): every type variable of its result gets a type by matching.
    pub fn call_known(&self, f: &FnTy, args: &[Ty]) -> Ty {
        let mut sigma = Sigma::new();
        for ((_, p), k) in f.params.iter().zip(args) {
            self.match_k(p, k, &mut sigma, 0);
        }
        let sigma = self.whole_sigma(&sigma, &self.hidden_inputs(f));
        self.ksubst(&f.ret, &sigma, false).unwrap_or(Ty::Top)
    }

    /// Gives a name of a pattern its known type. In a `case`, the choices of
    /// a clause bind one name together, so it gets the union.
    pub fn kbind(&mut self, var: VarId, k: Ty, in_case: bool) {
        let k = match self.kvars.get(&var) {
            Some(old) if in_case => self.kunion(vec![old.clone(), k]),
            _ => k,
        };
        self.kvars.insert(var, k);
    }

    /// The known type that `x` gets in `x = q`, when the value has the
    /// known type `b`: the member of `b` with the type constructor of the
    /// pattern type of `q`, or what is written in `q` (§4.8).
    pub fn member_for(&self, q: &Pattern, b: &Ty) -> Ty {
        let prim = |t| Head::Named(TypeRef::Builtin(t));
        let head = match &q.kind {
            PatternKind::Var(_) | PatternKind::Discard => return b.clone(),
            PatternKind::Typed { ty, .. } => return self.kconv(ty, &HashMap::new()),
            PatternKind::Named { pattern, .. } => return self.member_for(pattern, b),
            PatternKind::Int { .. } => prim(BuiltinType::Int),
            PatternKind::Float { .. } => prim(BuiltinType::Float),
            PatternKind::String(_) | PatternKind::Prefix { .. } => prim(BuiltinType::String),
            PatternKind::BitArray(_) => prim(BuiltinType::BitArray),
            PatternKind::Constructor { ctor, .. } => Head::Named(TypeRef::Custom(ctor.ty)),
            PatternKind::Tuple(ps) => Head::Tuple(ps.len() as u32),
            PatternKind::List { .. } => Head::Named(TypeRef::Builtin(BuiltinType::List)),
            PatternKind::Error(_) => return Ty::Top,
        };
        let m = one_member(b, head).unwrap_or(Ty::Top);
        match coverage_literal(q) {
            Some(t) => display::known_inter(&self.s, vec![m, t]),
            None => m,
        }
    }

    /// Names in patterns (§4.8): when `q` is fitted to a value with the
    /// known type `b`, its names get their known types. A catch-all gets
    /// `b`; any other pattern takes the member of `b` with its type
    /// constructor, and its parts get the parts of that member. `x: T` gets
    /// `T`, whatever the value.
    pub fn fit_known(&mut self, q: &Pattern, b: &Ty, in_case: bool) {
        let prim = |t| Ty::named(TypeRef::Builtin(t), Vec::new());
        match &q.kind {
            PatternKind::Var(x) => self.kbind(x.var, b.clone(), in_case),
            PatternKind::Typed {
                binder: Some(x),
                ty,
            } => {
                let k = self.kconv(ty, &HashMap::new());
                self.kbind(x.var, k, in_case);
            }
            PatternKind::Named { binder, pattern } => {
                let k = self.member_for(pattern, b);
                self.kbind(binder.var, k, in_case);
                self.fit_known(pattern, b, in_case);
            }
            PatternKind::Prefix { rest: Some(x), .. } => {
                self.kbind(x.var, prim(BuiltinType::String), in_case);
            }
            PatternKind::Constructor { ctor, args, spread } => {
                let args_k = match member(b, Some(Head::Named(TypeRef::Custom(ctor.ty)))) {
                    Some(Ty::Con(_, _, xs)) => Some(xs),
                    _ => None,
                };
                let n = match &self.p.ty(ctor.ty).body {
                    TypeBody::Custom(cs) => cs[ctor.index as usize].fields.len(),
                    TypeBody::Alias(_) => 0,
                };
                let fields: Vec<(Option<String>, Ty)> = (0..n)
                    .map(|i| {
                        let label = match &self.p.ty(ctor.ty).body {
                            TypeBody::Custom(cs) => cs[ctor.index as usize].fields[i]
                                .label
                                .as_ref()
                                .map(|l| l.name.clone()),
                            TypeBody::Alias(_) => None,
                        };
                        let k = match &args_k {
                            Some(xs) => self.field_known(*ctor, i, Some(xs)),
                            None => Ty::Top,
                        };
                        (label, k)
                    })
                    .collect();
                for (arg, i) in self.match_fields(q.span, &fields, args, *spread) {
                    let k = fields[i].1.clone();
                    self.fit_known(arg, &k, in_case);
                }
            }
            PatternKind::Tuple(ps) => {
                let parts = match member(b, Some(Head::Tuple(ps.len() as u32))) {
                    Some(Ty::Con(_, _, xs)) => xs,
                    _ => vec![Ty::Top; ps.len()],
                };
                for (x, k) in ps.iter().zip(&parts) {
                    self.fit_known(x, k, in_case);
                }
            }
            PatternKind::List { items, rest } => {
                let list = member(b, Some(Head::Named(TypeRef::Builtin(BuiltinType::List))));
                let e = match &list {
                    Some(Ty::Con(_, _, xs)) if xs.len() == 1 => xs[0].clone(),
                    _ => Ty::Top,
                };
                for x in items {
                    self.fit_known(x, &e, in_case);
                }
                if let Some(Some(x)) = rest {
                    self.kbind(x.var, list.unwrap_or(Ty::Top), in_case);
                }
            }
            PatternKind::BitArray(segs) => {
                for s in segs {
                    let k = match s.format.as_ref() {
                        None => prim(BuiltinType::Int),
                        Some(f) => format_prim(&f.name.name).map_or(Ty::Top, prim),
                    };
                    self.fit_known(&s.value, &k, in_case);
                }
            }
            PatternKind::Error(ps) => {
                for x in ps {
                    self.fit_known(x, &Ty::Top, in_case);
                }
            }
            _ => {}
        }
    }

    /// What is written in a value, as a known type (§4.8): the known type
    /// of a name inside the value of its own binding, and of a top-level
    /// name of the same group. The body of a function is not looked at.
    pub fn written_known(&self, e: &Expr) -> Ty {
        let prim = |t| Ty::named(TypeRef::Builtin(t), Vec::new());
        match &e.kind {
            ExprKind::Int(n) => scalar_literal(BuiltinType::Int, n.to_string()),
            ExprKind::Float(_) => prim(BuiltinType::Float),
            ExprKind::String(s) => scalar_literal(BuiltinType::String, s.clone()),
            ExprKind::BitArray(_) => prim(BuiltinType::BitArray),
            ExprKind::Unary {
                op: ilang_syntax::ast::UnOp::Neg,
                expr,
            } if neg_literal(expr).is_some() => {
                scalar_literal(BuiltinType::Int, neg_literal(expr).unwrap())
            }
            ExprKind::Unary { op, .. } => prim(match op {
                ilang_syntax::ast::UnOp::Neg => BuiltinType::Int,
                ilang_syntax::ast::UnOp::NegFloat => BuiltinType::Float,
            }),
            ExprKind::Tuple(items) => {
                Ty::tuple(items.iter().map(|x| self.written_known(x)).collect())
            }
            ExprKind::Block(b) => match b.stmts.as_slice() {
                [Stmt::Expr(x)] => self.written_known(x),
                _ => Ty::Top,
            },
            ExprKind::Fn(f) => self.fn_known(f, &[], Ty::Top),
            _ => Ty::Top,
        }
    }

    /// The known type of a function expression (§4.8): its labels; for each
    /// parameter, the type written at its top, or else the one from the
    /// expected type in `expected`, or else `_`; and the written `->` part,
    /// or else `body`.
    pub fn fn_known(&self, f: &Function, expected: &[Ty], body: Ty) -> Ty {
        let local = HashMap::new();
        let params = f
            .params
            .iter()
            .filter_map(|p| match p {
                Param::Pattern { pattern, label } => Some((pattern, label)),
                Param::Env { .. } => None,
            })
            .enumerate()
            .map(|(i, (pattern, label))| {
                let k = match top_written(pattern) {
                    Some(t) => self.kconv(t, &local),
                    None => expected.get(i).cloned().unwrap_or(Ty::Top),
                };
                (label.clone(), k)
            })
            .collect();
        let (ret, err, env) = match &f.ret {
            Some(r) => (
                self.kconv(&r.value, &local),
                r.err.as_ref().map_or(Ty::Never, |t| self.kconv(t, &local)),
                r.env.as_ref().map_or(Ty::Never, |t| self.kconv(t, &local)),
            ),
            None => (body, Ty::Top, Ty::Top),
        };
        Ty::Fn(Box::new(FnTy {
            params,
            ret,
            err,
            env,
        }))
    }

    /// The known types of the names of a binding inside its own value, and
    /// of a top-level binding inside its group (§4.8): from `x: T` at the
    /// top of the pattern, or else from what is written in the value.
    pub fn own_known(&mut self, b: &Binding) {
        let k = match top_written(&b.pattern) {
            Some(t) => self.kconv(t, &HashMap::new()),
            None => self.written_known(&b.value),
        };
        self.fit_known(&b.pattern, &k, false);
    }

    /// The expected type of the value of a binding (§4.8): `T` when the
    /// pattern has `x: T` at its top.
    pub fn binding_expected(&self, b: &Binding) -> Ty {
        top_written(&b.pattern).map_or(Ty::Top, |t| self.kconv(t, &HashMap::new()))
    }
}

/// The type variables of `t`.
fn ty_vars(t: &Ty, out: &mut Vec<V>) {
    match t {
        Ty::Graph(g) => {
            for v in g.variables() {
                if !out.contains(&v) {
                    out.push(v);
                }
            }
        }
        Ty::Var(v) => {
            if !out.contains(v) {
                out.push(*v);
            }
        }
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

/// §8.2: the type of the value of a segment with the format `name`, when
/// there is such a format.
pub(super) fn format_prim(name: &str) -> Option<BuiltinType> {
    let int = |s: &str| {
        let (digits, end) = match s.strip_suffix("be").or_else(|| s.strip_suffix("le")) {
            Some(d) => (d, true),
            None => (s, false),
        };
        let n: u32 = match digits.parse() {
            Ok(n) if !digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit()) => n,
            _ => return false,
        };
        if end {
            n >= 16 && n.is_multiple_of(8)
        } else {
            n >= 1 && (n <= 8 || !n.is_multiple_of(8))
        }
    };
    Some(match name {
        "u" | "i" => BuiltinType::Int,
        "bits" | "bytes" => BuiltinType::BitArray,
        "f16be" | "f16le" | "f32be" | "f32le" | "f64be" | "f64le" => BuiltinType::Float,
        "utf8" | "utf16be" | "utf16le" | "utf32be" | "utf32le" => BuiltinType::String,
        "utf8_codepoint" | "utf16be_codepoint" | "utf16le_codepoint" | "utf32be_codepoint"
        | "utf32le_codepoint" => BuiltinType::UtfCodepoint,
        _ if name
            .strip_prefix('u')
            .or_else(|| name.strip_prefix('i'))
            .is_some_and(int) =>
        {
            BuiltinType::Int
        }
        _ => return None,
    })
}

/// Widen known literal facts only in covariant parts (literal-types decision 7).
pub(super) fn whole_lits(s: &super::solve::Solver, t: &Ty) -> Ty {
    match t {
        Ty::Graph(g) => g.map_nodes(false, &mut |t, _| match t {
            Ty::Never | Ty::Both(..) => Ty::Top,
            Ty::Con(h, values, args) => {
                let values = if args.is_empty()
                    && matches!(
                        h,
                        Head::Named(TypeRef::Builtin(BuiltinType::Int | BuiltinType::String))
                    ) {
                    ValueSet::ALL
                } else {
                    values
                };
                let variance = s.variances(h, args.len());
                let args = args
                    .into_iter()
                    .zip(variance)
                    .map(|(t, v)| if v == super::Variance::Co { t } else { Ty::Top })
                    .collect();
                Ty::Con(h, values, args)
            }
            Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f.params.into_iter().map(|(l, _)| (l, Ty::Top)).collect(),
                ret: f.ret,
                err: f.err,
                env: Ty::Top,
            })),
            Ty::Union(ms) => display::known_union(s, ms),
            Ty::Inter(ms) => display::known_inter(s, ms),
            Ty::Diff(base, heads)
                if heads
                    .iter()
                    .any(|t| t.scalar().is_some() && !t.values().is_all()) =>
            {
                *base
            }
            Ty::Diff(base, heads) => display::known_diff(s, *base, heads),
            t => t,
        }),
        Ty::Never => Ty::Top,
        Ty::Con(h, values, xs) => Ty::Con(
            *h,
            if t.scalar().is_some() {
                super::lits::ValueSet::ALL
            } else {
                values.clone()
            },
            xs.iter()
                .zip(s.variances(*h, xs.len()))
                .map(|(x, v)| {
                    if v == super::Variance::Co {
                        whole_lits(s, x)
                    } else {
                        Ty::Top
                    }
                })
                .collect(),
        ),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f.params.iter().map(|(l, _)| (l.clone(), Ty::Top)).collect(),
            ret: whole_lits(s, &f.ret),
            err: whole_lits(s, &f.err),
            env: Ty::Top,
        })),
        Ty::Union(ms) => display::known_union(s, ms.iter().map(|t| whole_lits(s, t)).collect()),
        Ty::Inter(ms) => display::known_inter(s, ms.iter().map(|t| whole_lits(s, t)).collect()),
        Ty::Diff(a, h)
            if h.iter()
                .any(|t| t.scalar().is_some() && !t.values().is_all()) =>
        {
            whole_lits(s, a)
        }
        Ty::Diff(a, h) => display::known_diff(s, whole_lits(s, a), h.clone()),
        Ty::Both(..) => Ty::Top,
        t => t.clone(),
    }
}

/// Inputs not shown by a covariant structural part of an actual argument.
pub(super) fn hidden_inputs(s: &super::solve::Solver, f: &FnTy) -> Vec<V> {
    fn walk(s: &super::solve::Solver, t: &Ty, given: bool, shown: bool, out: &mut Vec<V>) {
        match t {
            Ty::Graph(g) => {
                use super::records::Node;
                let mut seen = std::collections::HashSet::new();
                let mut pending = vec![(g.id, given, shown)];
                while let Some((id, given, shown)) = pending.pop() {
                    if !seen.insert((id, given, shown)) {
                        continue;
                    }
                    match g.node_at(id) {
                        Node::Var(v) if given && !shown => {
                            if !out.contains(&v) {
                                out.push(v);
                            }
                        }
                        Node::Con(h, _, args) => {
                            let variance = s.variances(h, args.len());
                            for (id, v) in args.into_iter().zip(variance) {
                                let (given, shown) = match v {
                                    super::Variance::Co => (given, shown),
                                    super::Variance::Contra => (!given, false),
                                    _ => (true, false),
                                };
                                pending.push((id, given, shown));
                            }
                        }
                        Node::Fn(fun) => {
                            pending
                                .extend(fun.params.into_iter().map(|(_, id)| (id, !given, false)));
                            pending.extend([
                                (fun.ret, given, false),
                                (fun.err, given, false),
                                (fun.env, true, false),
                            ]);
                        }
                        Node::Both(a, b) => pending.extend([(a, true, false), (b, true, false)]),
                        node => {
                            pending.extend(node.children().into_iter().map(|id| (id, given, shown)))
                        }
                    }
                }
            }
            Ty::Var(v) if given && !shown => {
                if !out.contains(v) {
                    out.push(*v);
                }
            }
            Ty::Con(h, _, xs) => {
                for (x, v) in xs.iter().zip(s.variances(*h, xs.len())) {
                    match v {
                        super::Variance::Co => walk(s, x, given, shown, out),
                        super::Variance::Contra => walk(s, x, !given, false, out),
                        _ => walk(s, x, true, false, out),
                    }
                }
            }
            Ty::Fn(f) => {
                for (_, p) in &f.params {
                    walk(s, p, !given, false, out);
                }
                walk(s, &f.ret, given, false, out);
                walk(s, &f.err, given, false, out);
                walk(s, &f.env, true, false, out);
            }
            Ty::Union(ms) | Ty::Inter(ms) => {
                for t in ms {
                    walk(s, t, given, shown, out);
                }
            }
            Ty::Diff(a, h) => {
                walk(s, a, given, shown, out);
                for t in h {
                    walk(s, t, given, shown, out);
                }
            }
            Ty::Both(a, b) => {
                walk(s, a, true, false, out);
                walk(s, b, true, false, out);
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for (_, p) in &f.params {
        walk(s, p, true, true, &mut out);
    }
    walk(s, &f.ret, false, false, &mut out);
    walk(s, &f.env, true, false, &mut out);
    out
}

/// A scalar singleton uses the same canonical text in inference and coverage.
pub(super) fn scalar_literal(b: BuiltinType, text: String) -> Ty {
    Ty::lit(b, ValueSet::only([text.into()]).unwrap())
}

fn coverage_literal(q: &Pattern) -> Option<Ty> {
    match super::coverage::literal(q)? {
        super::coverage::Lit::Int(n) => Some(scalar_literal(BuiltinType::Int, n)),
        super::coverage::Lit::Str(s) => Some(scalar_literal(BuiltinType::String, s)),
        _ => None,
    }
}

/// Only brace groups around an unsigned literal are transparent to unary minus.
pub(super) fn neg_literal(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Int(n) => Some(super::lits::int_text(true, n)),
        ExprKind::Block(b) => match b.stmts.as_slice() {
            [Stmt::Expr(x)] => neg_literal(x),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod literal_known_tests {
    use super::*;
    #[test]
    fn hidden_inputs_and_recursive_widening() {
        // decision 7: callback outputs and environments, not callback inputs.
        let s = super::super::solve::Solver::new(Default::default(), Vec::new());
        let f = FnTy {
            params: vec![(
                None,
                Ty::Fn(Box::new(FnTy {
                    params: vec![(None, Ty::Var(V(0)))],
                    ret: Ty::Var(V(1)),
                    err: Ty::Never,
                    env: Ty::Never,
                })),
            )],
            ret: Ty::Var(V(2)),
            err: Ty::Never,
            env: Ty::Var(V(3)),
        };
        assert_eq!(hidden_inputs(&s, &f), [V(1), V(3)]);
        let lit = Ty::lit(
            BuiltinType::Int,
            super::super::lits::ValueSet::only([std::sync::Arc::from("0")]).unwrap(),
        );
        assert_eq!(
            whole_lits(&s, &Ty::tuple(vec![lit.clone(), Ty::Never])),
            Ty::tuple(vec![
                Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new()),
                Ty::Top
            ])
        );
        let a = Ty::Diff(
            Box::new(Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new())),
            vec![lit.clone()],
        );
        assert!(disjoint(&lit, &a));
        assert_eq!(members(&Ty::Inter(vec![lit.clone(), a])), Vec::<Ty>::new());
    }
}
