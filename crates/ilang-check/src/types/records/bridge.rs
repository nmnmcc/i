//! Safe shared handles between the legacy non-record algebra and exact graphs.
//! Payload identity follows tsrs TypeId; immutable shallow nodes follow Elixir.

use super::graph::{Arena, Function, Id, Label, Node};
use crate::types::{FnTy, Head, Ty, V};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
};

#[derive(Clone)]
pub(crate) struct Pool(Arc<RwLock<Arena>>);

impl Default for Pool {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(Arena::default())))
    }
}

impl fmt::Debug for Pool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TypePool")
    }
}

impl PartialEq for Pool {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for Pool {}
impl Hash for Pool {
    fn hash<H: Hasher>(&self, h: &mut H) {
        Arc::as_ptr(&self.0).hash(h);
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Graph {
    pool: Pool,
    pub id: Id,
}

impl fmt::Debug for Graph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Graph")
            .field("id", &self.id)
            .field("node", &self.node())
            .finish()
    }
}

impl Pool {
    pub fn node(&self, id: Id) -> Node {
        self.0.read().expect("type pool").node(id).clone()
    }
    pub fn record_domain(&self, id: Id) -> bool {
        self.0.read().expect("type pool").record_domain(id)
    }
    pub fn ty(&self, id: Id) -> Ty {
        match self.0.read().expect("type pool").node(id) {
            Node::Never => Ty::Never,
            Node::Top => Ty::Top,
            Node::Var(v) => Ty::Var(*v),
            Node::Con(h, values, parts) if parts.is_empty() => {
                Ty::Con(*h, values.clone(), Vec::new())
            }
            _ => Ty::Graph(Graph {
                pool: self.clone(),
                id,
            }),
        }
    }

    pub fn intern(&self, node: Node) -> Id {
        self.0.write().expect("type pool").intern(node)
    }

    pub fn record(&self, fields: Vec<(Label, Ty)>) -> Ty {
        let fields = fields
            .into_iter()
            .map(|(l, t)| (l, self.import(&t)))
            .collect();
        let id = self.0.write().expect("type pool").record(fields);
        self.ty(id)
    }

    pub fn without(&self, base: &Ty, labels: Vec<Label>) -> Ty {
        let base = self.import(base);
        let id = self.0.write().expect("type pool").without(base, labels);
        self.ty(id)
    }

    pub fn union(&self, members: &[Ty]) -> Ty {
        let members: Vec<_> = members.iter().map(|t| self.import(t)).collect();
        let mut arena = self.0.write().expect("type pool");
        let id = members
            .into_iter()
            .fold(Id::NEVER, |a, b| arena.union(a, b));
        drop(arena);
        self.ty(id)
    }

    pub fn inter(&self, members: &[Ty]) -> Ty {
        let members: Vec<_> = members.iter().map(|t| self.import(t)).collect();
        let mut arena = self.0.write().expect("type pool");
        let id = members.into_iter().fold(Id::TOP, |a, b| arena.inter(a, b));
        drop(arena);
        self.ty(id)
    }

    /// Import an actual legacy tree iteratively, without recursively hashing
    /// it. Existing graph children transfer once per handle, never per path.
    pub fn import(&self, ty: &Ty) -> Id {
        enum Task<'a> {
            Enter(&'a Ty),
            Finish(&'a Ty),
        }
        let mut tasks = vec![Task::Enter(ty)];
        let mut values = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Enter(t) => match t {
                    Ty::Graph(g) => values.push(self.import_graph(g)),
                    Ty::Never => values.push(Id::NEVER),
                    Ty::Top => values.push(Id::TOP),
                    Ty::Var(v) => values.push(self.0.write().expect("type pool").var(*v)),
                    _ => {
                        tasks.push(Task::Finish(t));
                        let parts: Vec<&Ty> = match t {
                            Ty::Con(_, _, parts) | Ty::Union(parts) | Ty::Inter(parts) => {
                                parts.iter().collect()
                            }
                            Ty::Fn(f) => f
                                .params
                                .iter()
                                .map(|(_, t)| t)
                                .chain([&f.ret, &f.err, &f.env])
                                .collect(),
                            Ty::Diff(base, heads) => {
                                std::iter::once(base.as_ref()).chain(heads).collect()
                            }
                            Ty::Both(a, b) => vec![a, b],
                            _ => unreachable!(),
                        };
                        tasks.extend(parts.into_iter().rev().map(Task::Enter));
                    }
                },
                Task::Finish(t) => {
                    let arity = match t {
                        Ty::Con(_, _, parts) | Ty::Union(parts) | Ty::Inter(parts) => parts.len(),
                        Ty::Fn(f) => f.params.len() + 3,
                        Ty::Diff(_, heads) => heads.len() + 1,
                        Ty::Both(..) => 2,
                        _ => unreachable!(),
                    };
                    let parts: Vec<_> = values.drain(values.len() - arity..).collect();
                    let mut arena = self.0.write().expect("type pool");
                    let id = match t {
                        Ty::Con(h, vs, _) => arena.con(*h, vs.clone(), parts),
                        Ty::Fn(f) => {
                            let n = f.params.len();
                            arena.function(Function {
                                params: f
                                    .params
                                    .iter()
                                    .zip(&parts)
                                    .map(|((l, _), id)| (l.as_deref().map(Arc::from), *id))
                                    .collect(),
                                ret: parts[n],
                                err: parts[n + 1],
                                env: parts[n + 2],
                            })
                        }
                        Ty::Union(_) => parts.into_iter().fold(Id::NEVER, |a, b| arena.union(a, b)),
                        Ty::Inter(_) => parts.into_iter().fold(Id::TOP, |a, b| arena.inter(a, b)),
                        Ty::Diff(..) => arena.difference(parts[0], parts[1..].to_vec()),
                        Ty::Both(..) => arena.both(parts[0], parts[1]),
                        _ => unreachable!(),
                    };
                    values.push(id);
                }
            }
        }
        assert_eq!(values.len(), 1);
        values[0]
    }

    fn import_graph(&self, g: &Graph) -> Id {
        if self == &g.pool {
            return g.id;
        }
        let mut done = FxHashMap::default();
        let mut pending = vec![(g.id, false)];
        while let Some((id, finish)) = pending.pop() {
            if done.contains_key(&id) {
                continue;
            }
            let node = g.node_at(id);
            if !finish {
                pending.push((id, true));
                pending.extend(node.children().into_iter().map(|t| (t, false)));
                continue;
            }
            let node = remap(node, &done);
            let id2 = self.intern(node);
            done.insert(id, id2);
        }
        done[&g.id]
    }
}

fn remap(node: Node, done: &FxHashMap<Id, Id>) -> Node {
    match node {
        Node::Never | Node::Top | Node::Var(_) => node,
        Node::Con(h, vs, parts) => Node::Con(h, vs, parts.iter().map(|t| done[t]).collect()),
        Node::Fn(f) => Node::Fn(Function {
            params: f.params.into_iter().map(|(l, t)| (l, done[&t])).collect(),
            ret: done[&f.ret],
            err: done[&f.err],
            env: done[&f.env],
        }),
        Node::Rec(fs) => Node::Rec(fs.into_iter().map(|(l, t)| (l, done[&t])).collect()),
        Node::Union(a, b) => Node::Union(done[&a], done[&b]),
        Node::Inter(a, b) => Node::Inter(done[&a], done[&b]),
        Node::Without(base, ls) => Node::Without(done[&base], ls),
        Node::Diff(base, hs) => Node::Diff(done[&base], hs.iter().map(|t| done[t]).collect()),
        Node::Both(a, b) => Node::Both(done[&a], done[&b]),
    }
}

impl Graph {
    /// Bottom-up rewrite of each shared node once per environment mode.
    /// The callback receives one shallow legacy step; record nodes stay exact.
    pub fn map_nodes(&self, env: bool, f: &mut impl FnMut(Ty, bool) -> Ty) -> Ty {
        let mut done: FxHashMap<(Id, bool), Ty> = FxHashMap::default();
        let mut pending = vec![(self.id, env, false)];
        while let Some((id, env, finish)) = pending.pop() {
            if done.contains_key(&(id, env)) {
                continue;
            }
            let node = self.node_at(id);
            let edges: Vec<_> = node
                .children()
                .into_iter()
                .enumerate()
                .map(|(i, child)| {
                    let env = match &node {
                        Node::Fn(fun) => i == fun.params.len() + 2,
                        Node::Con(..) | Node::Rec(_) | Node::Without(..) => false,
                        Node::Diff(..) => i == 0 && env,
                        _ => env,
                    };
                    (child, env)
                })
                .collect();
            if !finish {
                pending.push((id, env, true));
                pending.extend(edges.into_iter().map(|(id, env)| (id, env, false)));
                continue;
            }
            let mut parts = edges.iter().map(|key| done[key].clone());
            let mut next = || parts.next().expect("rewritten child");
            let shallow = match node {
                Node::Never => Ty::Never,
                Node::Top => Ty::Top,
                Node::Var(v) => Ty::Var(v),
                Node::Con(h, vs, args) => Ty::Con(h, vs, args.iter().map(|_| next()).collect()),
                Node::Fn(fun) => Ty::Fn(Box::new(FnTy {
                    params: fun
                        .params
                        .into_iter()
                        .map(|(label, _)| (label.map(|l| l.to_string()), next()))
                        .collect(),
                    ret: next(),
                    err: next(),
                    env: next(),
                })),
                Node::Rec(fields) => self
                    .pool
                    .record(fields.into_iter().map(|(l, _)| (l, next())).collect()),
                Node::Union(..) => Ty::Union(vec![next(), next()]),
                Node::Inter(..) => Ty::Inter(vec![next(), next()]),
                Node::Without(_, labels) => self.pool.without(&next(), labels),
                Node::Diff(_, heads) => {
                    Ty::Diff(Box::new(next()), heads.iter().map(|_| next()).collect())
                }
                Node::Both(..) => Ty::Both(Box::new(next()), Box::new(next())),
            };
            let rewritten = f(shallow, env);
            let rewritten = self.pool.ty(self.pool.import(&rewritten));
            done.insert((id, env), rewritten);
        }
        done[&(self.id, env)].clone()
    }

    /// MLstruct recordUnion supplies only the common-field summary of U5.
    /// It is cached as metadata when requested, without forcing
    /// choices or changing the exact type stored in this graph.
    pub fn known_summary(&self) -> Option<Ty> {
        let known = self.pool.0.write().expect("type pool").known(self.id);
        match known {
            super::graph::Known::Empty => Some(Ty::Never),
            super::graph::Known::Other => None,
            super::graph::Known::Record(fields) => Some(
                self.pool.record(
                    fields
                        .into_iter()
                        .map(|(l, t)| (l, self.child(t)))
                        .collect(),
                ),
            ),
        }
    }
    pub fn pool(&self) -> &Pool {
        &self.pool
    }
    /// Label parameters in the graph, visited once per shared node.
    pub fn label_params(&self) -> FxHashSet<super::graph::LabelParam> {
        let arena = self.pool.0.read().expect("type pool");
        let mut seen = FxHashSet::default();
        let mut params = FxHashSet::default();
        let mut todo = vec![self.id];
        while let Some(id) = todo.pop() {
            if !seen.insert(id) {
                continue;
            }
            let node = arena.node(id);
            match node {
                Node::Rec(fields) => {
                    for (label, _) in fields {
                        if let Label::Param(p) = label {
                            params.insert(p.clone());
                        }
                    }
                }
                Node::Without(_, labels) => {
                    for label in labels {
                        if let Label::Param(p) = label {
                            params.insert(p.clone());
                        }
                    }
                }
                _ => {}
            }
            todo.extend(node.children());
        }
        params
    }

    pub fn node(&self) -> Node {
        self.node_at(self.id)
    }
    pub fn node_at(&self, id: Id) -> Node {
        self.pool.0.read().expect("type pool").node(id).clone()
    }
    pub fn child(&self, id: Id) -> Ty {
        self.pool.ty(id)
    }

    pub fn head(&self) -> Option<Head> {
        match self.pool.0.read().expect("type pool").node(self.id) {
            Node::Rec(_) => Some(Head::Record),
            Node::Con(h, _, _) => Some(*h),
            Node::Fn(_) => Some(Head::Fn),
            _ => None,
        }
    }

    pub fn is_ordinary_head(&self) -> bool {
        matches!(
            self.head(),
            Some(Head::Named(_) | Head::Tuple(_) | Head::Fn)
        )
    }

    /// One shallow step for ordinary non-record rules; products and forgetting
    /// stay graph nodes and are handled by the record relation.
    pub fn view(&self) -> Option<Ty> {
        Some(match self.node() {
            Node::Never => Ty::Never,
            Node::Top => Ty::Top,
            Node::Var(v) => Ty::Var(v),
            Node::Con(h, vs, parts) => {
                Ty::Con(h, vs, parts.into_iter().map(|t| self.child(t)).collect())
            }
            Node::Fn(f) => Ty::Fn(Box::new(FnTy {
                params: f
                    .params
                    .into_iter()
                    .map(|(l, t)| (l.map(|l| l.to_string()), self.child(t)))
                    .collect(),
                ret: self.child(f.ret),
                err: self.child(f.err),
                env: self.child(f.env),
            })),
            Node::Union(a, b) => Ty::Union(vec![self.child(a), self.child(b)]),
            Node::Inter(a, b) => Ty::Inter(vec![self.child(a), self.child(b)]),
            Node::Diff(a, hs) => Ty::Diff(
                Box::new(self.child(a)),
                hs.into_iter().map(|t| self.child(t)).collect(),
            ),
            Node::Both(a, b) => Ty::Both(Box::new(self.child(a)), Box::new(self.child(b))),
            Node::Rec(_) | Node::Without(..) => return None,
        })
    }

    pub fn nodes(&self) -> Vec<(Id, Node)> {
        let arena = self.pool.0.read().expect("type pool");
        let mut seen = FxHashSet::default();
        let mut pending = vec![self.id];
        let mut out = Vec::new();
        while let Some(id) = pending.pop() {
            if seen.insert(id) {
                let node = arena.node(id);
                pending.extend(node.children());
                out.push((id, node.clone()));
            }
        }
        out
    }

    pub fn variables(&self) -> Vec<V> {
        let mut out: Vec<_> = self
            .nodes()
            .into_iter()
            .filter_map(|(_, node)| match node {
                Node::Var(v) => Some(v),
                _ => None,
            })
            .collect();
        out.sort_unstable();
        out
    }

    pub fn record_only(&self) -> bool {
        self.pool
            .0
            .read()
            .expect("type pool")
            .record_domain(self.id)
    }

    pub fn has_choices(&self) -> bool {
        self.pool
            .0
            .read()
            .expect("type pool")
            .record_alternatives(self.id)
            > 1
    }

    pub fn label_level(&self) -> u32 {
        self.pool.0.read().expect("type pool").label_level(self.id)
    }

    pub fn map_vars(&self, f: &mut impl FnMut(V) -> Ty) -> Ty {
        self.map_binders(f, &FxHashMap::default())
    }

    pub fn map_binders(
        &self,
        f: &mut impl FnMut(V) -> Ty,
        labels: &FxHashMap<super::graph::LabelParam, Label>,
    ) -> Ty {
        // Never hold the pool lock across a caller that can recursively inspect
        // bounds or import another graph into this pool.
        let replacements: FxHashMap<_, _> =
            self.variables().into_iter().map(|v| (v, f(v))).collect();
        let replacements: FxHashMap<_, _> = replacements
            .into_iter()
            .map(|(v, t)| (v, self.pool.import(&t)))
            .collect();
        let id = self.pool.0.write().expect("type pool").substitute(
            self.id,
            &mut |_, v| replacements[&v],
            labels,
        );
        self.pool.ty(id)
    }

    pub fn try_map_vars<E>(&self, f: &mut impl FnMut(V) -> Result<Ty, E>) -> Result<Ty, E> {
        let replacements: FxHashMap<_, _> = self
            .variables()
            .into_iter()
            .map(|v| f(v).map(|t| (v, t)))
            .collect::<Result<_, _>>()?;
        Ok(self.map_vars(&mut |v| replacements[&v].clone()))
    }
}

impl Graph {
    fn edges(
        node: &Node,
        pol: crate::types::solve::Pol,
        variances: &std::collections::HashMap<crate::hir::TypeId, Vec<crate::types::Variance>>,
    ) -> Vec<(Id, crate::types::solve::Pol)> {
        use crate::types::solve::Pol;
        match node {
            Node::Con(h, _, parts) => {
                let vs = match h {
                    Head::Tuple(_) => vec![crate::types::Variance::Co; parts.len()],
                    Head::Named(crate::hir::TypeRef::Builtin(b)) => {
                        crate::types::solve::builtin_variance(*b)
                    }
                    Head::Named(crate::hir::TypeRef::Custom(id)) => variances
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| vec![crate::types::Variance::Inv; parts.len()]),
                    Head::Record | Head::Fn => unreachable!("separate graph forms"),
                };
                parts.iter().zip(vs).map(|(t, v)| (*t, pol.by(v))).collect()
            }
            Node::Fn(f) => f
                .params
                .iter()
                .map(|(_, t)| (*t, pol.flip()))
                .chain([(f.ret, pol), (f.err, pol), (f.env, pol)])
                .collect(),
            Node::Diff(base, heads) => std::iter::once((*base, pol))
                .chain(heads.iter().map(|t| (*t, Pol::Both)))
                .collect(),
            Node::Both(a, b) => vec![(*a, Pol::Pos), (*b, Pol::Neg)],
            _ => node.children().into_iter().map(|t| (t, pol)).collect(),
        }
    }

    pub fn map_leaves(
        &self,
        pol: crate::types::solve::Pol,
        variances: &std::collections::HashMap<crate::hir::TypeId, Vec<crate::types::Variance>>,
        f: &mut impl FnMut(&Ty, crate::types::solve::Pol) -> Ty,
    ) -> Ty {
        self.rewrite_leaves(pol, None, variances, &mut |t, p, _| f(t, p))
    }

    pub fn map_leaves_env(
        &self,
        pol: crate::types::solve::Pol,
        env: bool,
        variances: &std::collections::HashMap<crate::hir::TypeId, Vec<crate::types::Variance>>,
        f: &mut impl FnMut(&Ty, crate::types::solve::Pol, bool) -> Ty,
    ) -> Ty {
        self.rewrite_leaves(pol, Some(env), variances, &mut |t, p, env| {
            f(t, p, env.unwrap())
        })
    }

    fn rewrite_leaves(
        &self,
        pol: crate::types::solve::Pol,
        env: Option<bool>,
        variances: &std::collections::HashMap<crate::hir::TypeId, Vec<crate::types::Variance>>,
        f: &mut impl FnMut(&Ty, crate::types::solve::Pol, Option<bool>) -> Ty,
    ) -> Ty {
        use crate::types::solve::Pol;
        let mut done = FxHashMap::default();
        let mut pending = vec![(self.id, pol, env, false)];
        while let Some((id, pol, env, finish)) = pending.pop() {
            if done.contains_key(&(id, pol, env)) {
                continue;
            }
            let node = self.node_at(id);
            let edges: Vec<_> = Self::edges(&node, pol, variances)
                .into_iter()
                .enumerate()
                .map(|(index, (child, p))| {
                    let (p, env) = match (&node, env) {
                        (Node::Con(..), Some(env)) => {
                            (if env { Pol::Both } else { p }, Some(false))
                        }
                        (Node::Fn(fun), Some(_)) => (p, Some(index == fun.params.len() + 2)),
                        (Node::Rec(_) | Node::Without(..), Some(_)) => (p, Some(false)),
                        (Node::Diff(..), Some(env)) => (pol, Some(index == 0 && env)),
                        _ => (p, env),
                    };
                    (child, p, env)
                })
                .collect();
            if !finish {
                pending.push((id, pol, env, true));
                pending.extend(
                    edges
                        .into_iter()
                        .map(|(child, p, env)| (child, p, env, false)),
                );
                continue;
            }
            let leaf = match &node {
                Node::Var(_) | Node::Top | Node::Never => true,
                Node::Con(_, _, parts) => parts.is_empty(),
                _ => false,
            };
            let out = if leaf {
                // Rewritten leaves can inspect or allocate solver bounds. The
                // arena lock is not held while calling the visitor.
                self.pool.import(&f(&self.child(id), pol, env))
            } else {
                let mut children = edges
                    .into_iter()
                    .map(|(child, p, env)| done[&(child, p, env)]);
                let mut next = || children.next().expect("rewritten graph child");
                let node = match node {
                    Node::Con(h, vs, parts) => {
                        Node::Con(h, vs, parts.iter().map(|_| next()).collect())
                    }
                    Node::Fn(f) => Node::Fn(Function {
                        params: f.params.into_iter().map(|(l, _)| (l, next())).collect(),
                        ret: next(),
                        err: next(),
                        env: next(),
                    }),
                    Node::Rec(fs) => Node::Rec(fs.into_iter().map(|(l, _)| (l, next())).collect()),
                    Node::Union(..) => Node::Union(next(), next()),
                    Node::Inter(..) => Node::Inter(next(), next()),
                    Node::Without(_, labels) => Node::Without(next(), labels),
                    Node::Diff(_, heads) => {
                        Node::Diff(next(), heads.iter().map(|_| next()).collect())
                    }
                    Node::Both(..) => Node::Both(next(), next()),
                    _ => unreachable!("leaf handled above"),
                };
                let mut arena = self.pool.0.write().expect("type pool");
                match node {
                    Node::Rec(fields) => arena.record(fields),
                    Node::Union(a, b) => arena.union(a, b),
                    Node::Inter(a, b) => arena.inter(a, b),
                    Node::Without(base, labels) => arena.without(base, labels),
                    Node::Both(a, b) => arena.both(a, b),
                    node => arena.intern(node),
                }
            };
            done.insert((id, pol, env), out);
        }
        self.pool.ty(done[&(self.id, pol, env)])
    }

    pub fn variable_sites(
        &self,
        solver: &crate::types::solve::Solver,
        pol: crate::types::solve::Pol,
        con: bool,
        env: bool,
    ) -> Vec<(V, crate::types::solve::Pol, bool)> {
        use crate::types::solve::Pol;
        let mut out = Vec::new();
        let mut seen = FxHashSet::default();
        let mut pending = vec![(self.id, pol, con, env)];
        while let Some((id, pol, con, env)) = pending.pop() {
            if !seen.insert((id, pol, con, env)) {
                continue;
            }
            let node = self.node_at(id);
            if let Node::Var(v) = node {
                out.push((v, pol, con));
                continue;
            }
            match node {
                Node::Con(h, _, parts) => {
                    let vs = solver.variances(h, parts.len());
                    for (t, v) in parts.into_iter().zip(vs) {
                        pending.push((t, if env { Pol::Both } else { pol.by(v) }, true, false));
                    }
                }
                Node::Fn(f) => {
                    pending.extend(
                        f.params
                            .into_iter()
                            .map(|(_, t)| (t, pol.flip(), true, false)),
                    );
                    pending.push((f.ret, pol, true, false));
                    pending.push((f.err, pol, true, false));
                    pending.push((f.env, pol, true, true));
                }
                Node::Rec(fields) => {
                    pending.extend(fields.into_iter().map(|(_, t)| (t, pol, true, false)))
                }
                Node::Without(base, _) => pending.push((base, pol, true, false)),
                Node::Both(a, b) => {
                    pending.extend([(a, Pol::Pos, con, false), (b, Pol::Neg, con, false)])
                }
                _ => pending.extend(node.children().into_iter().map(|t| (t, pol, con, env))),
            }
        }
        out
    }
}

impl Graph {
    pub fn invalid_places(
        &self,
        solver: &crate::types::solve::Solver,
        pol: crate::types::solve::Pol,
    ) -> bool {
        use crate::types::solve::Pol;
        let mut seen = FxHashSet::default();
        let mut pending = vec![(self.id, pol)];
        while let Some((id, pol)) = pending.pop() {
            if !seen.insert((id, pol)) {
                continue;
            }
            let node = self.node_at(id);
            let check_union = matches!(node, Node::Union(..)) && pol != Pol::Pos;
            let check_inter = matches!(node, Node::Inter(..)) && pol != Pol::Neg;
            if check_union || check_inter {
                let mut group = vec![id];
                let mut group_seen = FxHashSet::default();
                let mut variables = FxHashSet::default();
                while let Some(member) = group.pop() {
                    if !group_seen.insert(member) {
                        continue;
                    }
                    match self.node_at(member) {
                        Node::Var(v) => {
                            variables.insert(v);
                        }
                        Node::Union(a, b) if check_union => group.extend([a, b]),
                        Node::Inter(a, b) if check_inter => group.extend([a, b]),
                        _ => {}
                    }
                    if (check_inter && !variables.is_empty()) || variables.len() > 1 {
                        return true;
                    }
                }
            }
            pending.extend(Self::edges(&node, pol, &solver.variance));
        }
        false
    }

    /// Each unknown occurrence gets a fresh hole; tying unrelated holes would
    /// add an equality. Closed subgraphs can share their completed copies.
    pub fn known_bound(
        &self,
        solver: &mut crate::types::solve::Solver,
        demand: &mut super::bdd::Demand,
    ) -> Result<Ty, super::bdd::ResourceLimit> {
        enum Task {
            Enter(Id, u64),
            Finish(Id, u64, Node, Vec<(Id, u64)>),
        }
        let pool_key = Arc::as_ptr(&self.pool.0) as usize as u64;
        let scope = |id: Id, demand: &mut super::bdd::Demand| {
            if self.pool.0.read().expect("type pool").known_holes(id) {
                demand.new_scope()
            } else {
                0
            }
        };
        let root_scope = scope(self.id, demand);
        let mut done: FxHashMap<(Id, u64), Ty> = FxHashMap::default();
        let mut pending = vec![Task::Enter(self.id, root_scope)];
        while let Some(task) = pending.pop() {
            match task {
                Task::Enter(id, context) => {
                    demand.charge_state(&[4, pool_key, id.0 as u64, context])?;
                    if done.contains_key(&(id, context)) {
                        continue;
                    }
                    let node = self.node_at(id);
                    let children: Vec<_> = match &node {
                        Node::Var(_) | Node::Top | Node::Never | Node::Both(..) => Vec::new(),
                        Node::Con(h, _, parts) => parts
                            .iter()
                            .zip(solver.variances(*h, parts.len()))
                            .filter_map(|(t, v)| (v == crate::types::Variance::Co).then_some(*t))
                            .collect(),
                        Node::Fn(f) => vec![f.ret, f.err],
                        Node::Diff(base, _) => vec![*base],
                        _ => node.children(),
                    };
                    let children: Vec<_> = children
                        .into_iter()
                        .map(|t| (t, scope(t, demand)))
                        .collect();
                    pending.push(Task::Finish(id, context, node, children.clone()));
                    pending.extend(
                        children
                            .into_iter()
                            .rev()
                            .map(|(id, scope)| Task::Enter(id, scope)),
                    );
                }
                Task::Finish(id, context, node, children) => {
                    let mut children = children.into_iter().map(|key| done[&key].clone());
                    let mut next = || children.next().expect("known graph child");
                    let value = match node {
                        Node::Var(_) | Node::Top | Node::Both(..) => solver.fresh(),
                        Node::Never => Ty::Never,
                        Node::Con(h, vs, parts) => Ty::Con(
                            h,
                            if matches!(
                                h,
                                Head::Named(crate::hir::TypeRef::Builtin(
                                    crate::hir::BuiltinType::Int | crate::hir::BuiltinType::String
                                ))
                            ) {
                                crate::types::lits::ValueSet::ALL
                            } else {
                                vs
                            },
                            parts
                                .iter()
                                .zip(solver.variances(h, parts.len()))
                                .map(|(_, v)| {
                                    if v == crate::types::Variance::Co {
                                        next()
                                    } else {
                                        solver.fresh()
                                    }
                                })
                                .collect(),
                        ),
                        Node::Fn(f) => Ty::Fn(Box::new(FnTy {
                            params: f
                                .params
                                .into_iter()
                                .map(|(l, _)| (l.map(|l| l.to_string()), solver.fresh()))
                                .collect(),
                            ret: next(),
                            err: next(),
                            env: solver.fresh(),
                        })),
                        Node::Rec(fields) => self
                            .pool
                            .record(fields.into_iter().map(|(l, _)| (l, next())).collect()),
                        Node::Union(..) => self.pool.union(&[next(), next()]),
                        Node::Inter(..) => self.pool.inter(&[next(), next()]),
                        Node::Without(_, labels) => self.pool.without(&next(), labels),
                        Node::Diff(_, heads) => {
                            let base = next();
                            let heads: Vec<_> = heads.into_iter().map(|h| self.child(h)).collect();
                            if heads
                                .iter()
                                .any(|h| h.scalar().is_some() && !h.values().is_all())
                            {
                                base
                            } else {
                                Ty::Diff(Box::new(base), heads)
                            }
                        }
                    };
                    done.insert((id, context), value);
                }
            }
        }
        Ok(done
            .remove(&(self.id, root_scope))
            .expect("known graph root"))
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Field {
    Found(Ty),
    MayBe(Label),
    Missing,
    NotProduct,
}

impl Graph {
    pub fn fields(&self) -> Option<Vec<(Label, Ty)>> {
        let fields = match self.node() {
            Node::Rec(fields) => fields,
            _ => return None,
        };
        Some(
            fields
                .into_iter()
                .map(|(l, t)| (l, self.child(t)))
                .collect(),
        )
    }

    pub fn field(&self, wanted: &Label) -> Field {
        let lookup = {
            let arena = self.pool.0.read().expect("type pool");
            let Node::Rec(fields) = arena.node(self.id) else {
                return Field::NotProduct;
            };
            match fields.binary_search_by(|(l, _)| l.cmp(wanted)) {
                Ok(index) => Ok(fields[index].1),
                Err(_) => {
                    let candidate = match wanted {
                        Label::Name(_) => fields
                            .get(fields.partition_point(|(l, _)| matches!(l, Label::Name(_))))
                            .map(|(l, _)| l.clone()),
                        Label::Param(_) => fields.first().map(|(l, _)| l.clone()),
                    };
                    Err(candidate)
                }
            }
        };
        match lookup {
            Ok(id) => Field::Found(self.child(id)),
            Err(Some(label)) => Field::MayBe(label),
            Err(None) => Field::Missing,
        }
    }

    pub fn forgotten(&self) -> Option<(Ty, Vec<Label>)> {
        match self.node() {
            Node::Without(base, labels) => Some((self.child(base), labels)),
            _ => None,
        }
    }

    pub fn contains_member(&self, ty: &Ty, union: bool) -> bool {
        let wanted = self.pool.import(ty);
        let mut seen = FxHashSet::default();
        let mut pending = vec![self.id];
        while let Some(id) = pending.pop() {
            if id == wanted {
                return true;
            }
            if !seen.insert(id) {
                continue;
            }
            match self.node_at(id) {
                Node::Union(a, b) if union => pending.extend([a, b]),
                Node::Inter(a, b) if !union => pending.extend([a, b]),
                _ => {}
            }
        }
        false
    }

    /// Reapply the ordinary leaf algebra after substitution. This includes
    /// invariant nominal constraints; the record graph never replaces them
    /// by a common-field approximation or an unrecorded equality.
    pub fn normalize(
        &self,
        solver: &mut crate::types::solve::Solver,
        mode: crate::types::solve::Mode,
    ) -> Ty {
        use crate::types::solve::Mode;
        let mut needed = FxHashSet::default();
        let mut pending = vec![(self.id, mode)];
        while let Some((id, mode)) = pending.pop() {
            if !needed.insert((id, mode)) {
                continue;
            }
            match self.node_at(id) {
                Node::Con(head, _, parts) => pending.extend(
                    parts
                        .into_iter()
                        .zip(solver.rows(head, self.node_at(id).children().len())),
                ),
                Node::Fn(f) => {
                    pending.extend(f.params.into_iter().map(|(_, id)| (id, Mode::Value)));
                    pending.extend([(f.ret, Mode::Value), (f.err, Mode::Row), (f.env, Mode::Env)]);
                }
                Node::Rec(fields) => {
                    pending.extend(fields.into_iter().map(|(_, id)| (id, Mode::Value)))
                }
                Node::Without(base, _) => pending.push((base, Mode::Value)),
                node => pending.extend(node.children().into_iter().map(|id| (id, mode))),
            }
        }
        let mut order: Vec<_> = needed.into_iter().collect();
        order.sort_unstable();
        let mut done: FxHashMap<(Id, Mode), Ty> = FxHashMap::default();
        for (id, mode) in order {
            let value = match self.node_at(id) {
                Node::Never => Ty::Never,
                Node::Top => Ty::Top,
                Node::Var(v) => Ty::Var(v),
                Node::Con(head, values, parts) => {
                    let modes = solver.rows(head, parts.len());
                    Ty::Con(
                        head,
                        values,
                        parts
                            .into_iter()
                            .zip(modes)
                            .map(|(id, mode)| done[&(id, mode)].clone())
                            .collect(),
                    )
                }
                Node::Fn(f) => Ty::Fn(Box::new(FnTy {
                    params: f
                        .params
                        .into_iter()
                        .map(|(label, id)| {
                            (
                                label.map(|s| s.to_string()),
                                done[&(id, Mode::Value)].clone(),
                            )
                        })
                        .collect(),
                    ret: done[&(f.ret, Mode::Value)].clone(),
                    err: done[&(f.err, Mode::Row)].clone(),
                    env: done[&(f.env, Mode::Env)].clone(),
                })),
                Node::Rec(fields) => solver.record(
                    fields
                        .into_iter()
                        .map(|(label, id)| (label, done[&(id, Mode::Value)].clone()))
                        .collect(),
                ),
                Node::Union(a, b) => solver.union_mode(
                    vec![done[&(a, mode)].clone(), done[&(b, mode)].clone()],
                    mode,
                ),
                Node::Inter(a, b) => solver.inter_mode(
                    vec![done[&(a, mode)].clone(), done[&(b, mode)].clone()],
                    mode,
                ),
                Node::Without(base, labels) => solver.without(&done[&(base, Mode::Value)], labels),
                Node::Diff(base, heads) => solver.diff_mode(
                    done[&(base, mode)].clone(),
                    heads
                        .into_iter()
                        .map(|id| done[&(id, mode)].clone())
                        .collect(),
                    mode,
                ),
                Node::Both(a, b) => Ty::Both(
                    Box::new(done[&(a, mode)].clone()),
                    Box::new(done[&(b, mode)].clone()),
                ),
            };
            done.insert((id, mode), self.pool.ty(self.pool.import(&value)));
        }
        done.remove(&(self.id, mode))
            .expect("normalized graph root")
    }
}
