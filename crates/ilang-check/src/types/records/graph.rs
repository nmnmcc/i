//! Shared exact source types. Boolean evaluation is a separate demand.
//!
//! Stable handles follow the tsrs arena approach; local product reduction
//! follows Elixir's open-map intersection, with records.md's guarded E4.

use super::super::{Head, V, lits::ValueSet};
use crate::hir::{BindingRef, BuiltinType, TypeRef};
use rustc_hash::FxHashMap;
#[cfg(test)]
use rustc_hash::FxHashSet;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Id(pub u32);

impl Id {
    pub const NEVER: Self = Self(0);
    pub const TOP: Self = Self(1);
    pub const RECORD: Self = Self(2);
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LabelParam {
    pub owner: LabelOwner,
    pub index: u32,
    pub level: u32,
    pub name: Arc<str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LabelOwner {
    Binding(BindingRef),
    Artifact(u64),
}

impl From<BindingRef> for LabelOwner {
    fn from(binding: BindingRef) -> Self { Self::Binding(binding) }
}

impl LabelOwner {
    pub fn artifact() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        Self::Artifact(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Label {
    Name(Arc<str>),
    Param(LabelParam),
}

impl Label {
    pub fn text(&self) -> String {
        match self {
            Self::Name(s)
                if s.strip_prefix('_')
                    .unwrap_or(s)
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_lowercase)
                    && s.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') =>
            {
                s.to_string()
            }
            Self::Name(s) => ilang_syntax::print::string_literal(s),
            Self::Param(p) => p.name.to_string(),
        }
    }
    pub fn name(s: impl Into<Arc<str>>) -> Self {
        Self::Name(s.into())
    }

    pub fn may_equal(&self, other: &Self) -> bool {
        self != other && !matches!((self, other), (Self::Name(_), Self::Name(_)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Function {
    pub params: Vec<(Option<Arc<str>>, Id)>,
    pub ret: Id,
    pub err: Id,
    pub env: Id,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Node {
    Never,
    Top,
    Var(V),
    Con(Head, ValueSet, Vec<Id>),
    Fn(Function),
    Rec(Vec<(Label, Id)>),
    Union(Id, Id),
    Inter(Id, Id),
    Without(Id, Vec<Label>),
    /// Source head subtraction; never an internal Boolean complement.
    Diff(Id, Vec<Id>),
    Both(Id, Id),
}

/// U5 metadata only: never a replacement for the exact source node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Known {
    Empty,
    Other,
    Record(Vec<(Label, Id)>),
}

impl Node {
    pub fn children(&self) -> Vec<Id> {
        match self {
            Self::Never | Self::Top | Self::Var(_) => Vec::new(),
            Self::Con(_, _, children) => children.clone(),
            Self::Fn(f) => f
                .params
                .iter()
                .map(|(_, t)| *t)
                .chain([f.ret, f.err, f.env])
                .collect(),
            Self::Rec(fields) => fields.iter().map(|(_, t)| *t).collect(),
            Self::Union(a, b) | Self::Inter(a, b) | Self::Both(a, b) => vec![*a, *b],
            Self::Without(base, _) => vec![*base],
            Self::Diff(base, heads) => std::iter::once(*base)
                .chain(heads.iter().copied())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Arena {
    nodes: Vec<Node>,
    unique: FxHashMap<Node, Id>,
    inhabited: Vec<bool>,
    record_domain: Vec<bool>,
    label_level: Vec<u32>,
    plain_empty: Vec<bool>,
    known_holes: Vec<bool>,
    record_alternatives: Vec<u8>,
    known: Vec<Option<Known>>,
}

impl Default for Arena {
    fn default() -> Self {
        let mut graph = Self {
            nodes: Vec::new(),
            unique: FxHashMap::default(),
            inhabited: Vec::new(),
            record_domain: Vec::new(),
            label_level: Vec::new(),
            plain_empty: Vec::new(),
            known_holes: Vec::new(),
            record_alternatives: Vec::new(),
            known: Vec::new(),
        };
        assert_eq!(graph.intern(Node::Never), Id::NEVER);
        assert_eq!(graph.intern(Node::Top), Id::TOP);
        assert_eq!(graph.intern(Node::Rec(Vec::new())), Id::RECORD);
        graph
    }
}

impl Arena {
    pub fn node(&self, id: Id) -> &Node {
        &self.nodes[id.0 as usize]
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(super) fn intern(&mut self, node: Node) -> Id {
        if let Some(id) = self.unique.get(&node) {
            return *id;
        }
        let inhabited = match &node {
            Node::Top => true,
            Node::Con(
                Head::Named(TypeRef::Builtin(BuiltinType::Int | BuiltinType::String)),
                _,
                _,
            ) => true,
            Node::Con(Head::Tuple(_), _, parts) => parts.iter().all(|t| self.inhabited(*t)),
            Node::Rec(fields) => fields
                .iter()
                .all(|(l, t)| matches!(l, Label::Name(_)) && self.inhabited(*t)),
            Node::Union(a, b) => self.inhabited(*a) || self.inhabited(*b),
            _ => false,
        };
        let plain_empty = match &node {
            Node::Never => true,
            Node::Rec(fields) => fields.iter().any(|(_, t)| self.plain_empty(*t)),
            Node::Con(Head::Tuple(_), _, parts) => parts.iter().any(|t| self.plain_empty(*t)),
            _ => false,
        };
        let id = Id(u32::try_from(self.nodes.len()).expect("type graph handle overflow"));
        let known_holes = match &node {
            Node::Var(_) | Node::Top | Node::Fn(_) | Node::Both(..) => true,
            Node::Con(Head::Named(TypeRef::Custom(_)), _, parts) if !parts.is_empty() => true,
            Node::Con(Head::Named(TypeRef::Builtin(b)), _, parts)
                if !parts.is_empty() && !matches!(b, BuiltinType::List) =>
            {
                true
            }
            _ => node.children().into_iter().any(|t| self.known_holes(t)),
        };
        let record_domain = match &node {
            Node::Never | Node::Rec(_) | Node::Without(..) => true,
            Node::Union(a, b) | Node::Both(a, b) => {
                self.record_domain(*a) && self.record_domain(*b)
            }
            Node::Inter(a, b) => self.record_domain(*a) || self.record_domain(*b),
            Node::Diff(base, _) => self.record_domain(*base),
            _ => false,
        };
        let count = |id: Id| self.record_alternatives[id.0 as usize];
        let record_alternatives = match &node {
            Node::Never | Node::Con(..) | Node::Fn(_) => 0,
            Node::Top | Node::Var(_) | Node::Rec(_) => 1,
            Node::Union(a, b) => (count(*a) + count(*b)).min(2),
            Node::Inter(a, b) => (count(*a) * count(*b)).min(2),
            Node::Without(base, _) => count(*base),
            Node::Diff(base, heads) => {
                if heads.iter().any(|h| matches!(self.node(*h), Node::Rec(_))) {
                    0
                } else {
                    count(*base)
                }
            }
            Node::Both(a, b) => count(*a).max(count(*b)),
        };
        let own_level = match &node {
            Node::Rec(fs) => fs.iter().map(|(l, _)| l).collect::<Vec<_>>(),
            Node::Without(_, ls) => ls.iter().collect(),
            _ => Vec::new(),
        }
        .into_iter()
        .map(|l| match l {
            Label::Name(_) => 0,
            Label::Param(p) => p.level,
        })
        .max()
        .unwrap_or(0);
        let label_level = node
            .children()
            .into_iter()
            .map(|t| self.label_level(t))
            .max()
            .unwrap_or(0)
            .max(own_level);
        self.unique.insert(node.clone(), id);
        self.nodes.push(node);
        self.inhabited.push(inhabited);
        self.record_domain.push(record_domain);
        self.label_level.push(label_level);
        self.plain_empty.push(plain_empty);
        self.known_holes.push(known_holes);
        self.record_alternatives.push(record_alternatives);
        self.known.push(None);
        id
    }

    pub fn inhabited(&self, id: Id) -> bool {
        self.inhabited[id.0 as usize]
    }

    pub fn record_domain(&self, id: Id) -> bool {
        self.record_domain[id.0 as usize]
    }
    pub fn record_alternatives(&self, id: Id) -> u8 {
        self.record_alternatives[id.0 as usize]
    }
    fn known_at(&self, id: Id) -> &Known {
        self.known[id.0 as usize]
            .as_ref()
            .expect("completed known summary")
    }

    pub(super) fn known(&mut self, root: Id) -> Known {
        let mut pending = vec![(root, false)];
        while let Some((id, finish)) = pending.pop() {
            if self.known[id.0 as usize].is_some() {
                continue;
            }
            if !finish {
                pending.push((id, true));
                let children = match self.node(id) {
                    Node::Union(a, b) | Node::Inter(a, b) => vec![*a, *b],
                    Node::Without(base, _) | Node::Diff(base, _) => vec![*base],
                    Node::Both(_, b) => vec![*b],
                    _ => Vec::new(),
                };
                pending.extend(children.into_iter().map(|id| (id, false)));
                continue;
            }
            let known = match self.node(id).clone() {
                Node::Never => Known::Empty,
                Node::Rec(fields) => Known::Record(fields),
                Node::Union(a, b) => match (self.known_at(a).clone(), self.known_at(b).clone()) {
                    (Known::Empty, other) | (other, Known::Empty) => other,
                    (Known::Record(a), Known::Record(b)) => {
                        let mut fields = Vec::new();
                        for (label, ty) in a {
                            if let Ok(i) = b.binary_search_by(|(key, _)| key.cmp(&label)) {
                                fields.push((label, self.union(ty, b[i].1)));
                            }
                        }
                        Known::Record(fields)
                    }
                    _ => Known::Other,
                },
                Node::Inter(a, b) => match (self.known_at(a).clone(), self.known_at(b).clone()) {
                    (Known::Empty, _) | (_, Known::Empty) => Known::Empty,
                    (Known::Record(mut a), Known::Record(b)) => {
                        for (label, ty) in b {
                            match a.binary_search_by(|(key, _)| key.cmp(&label)) {
                                Ok(i) => a[i].1 = self.inter(a[i].1, ty),
                                Err(i) => a.insert(i, (label, ty)),
                            }
                        }
                        if a.iter().any(|(_, t)| self.plain_empty(*t)) {
                            Known::Empty
                        } else {
                            Known::Record(a)
                        }
                    }
                    (record @ Known::Record(_), _) | (_, record @ Known::Record(_)) => record,
                    _ => Known::Other,
                },
                Node::Without(base, labels) => match self.known_at(base).clone() {
                    Known::Empty => Known::Empty,
                    Known::Record(mut fields) => {
                        fields.retain(|(l, _)| !labels.iter().any(|k| k == l || k.may_equal(l)));
                        Known::Record(fields)
                    }
                    Known::Other => Known::Record(Vec::new()),
                },
                Node::Diff(base, heads) => {
                    if heads.iter().any(|h| matches!(self.node(*h), Node::Rec(_))) {
                        Known::Empty
                    } else {
                        self.known_at(base).clone()
                    }
                }
                Node::Both(_, b) => self.known_at(b).clone(),
                _ => Known::Other,
            };
            self.known[id.0 as usize] = Some(known);
        }
        self.known_at(root).clone()
    }
    pub fn label_level(&self, id: Id) -> u32 {
        self.label_level[id.0 as usize]
    }
    pub fn plain_empty(&self, id: Id) -> bool {
        self.plain_empty[id.0 as usize]
    }
    pub fn known_holes(&self, id: Id) -> bool {
        self.known_holes[id.0 as usize]
    }

    pub fn var(&mut self, v: V) -> Id {
        self.intern(Node::Var(v))
    }

    pub fn con(&mut self, head: Head, values: ValueSet, parts: Vec<Id>) -> Id {
        self.intern(Node::Con(head, values, parts))
    }

    pub fn function(&mut self, f: Function) -> Id {
        self.intern(Node::Fn(f))
    }

    pub fn record(&mut self, mut fields: Vec<(Label, Id)>) -> Id {
        fields.sort_by(|(a, _), (b, _)| a.cmp(b));
        let mut out: Vec<(Label, Id)> = Vec::with_capacity(fields.len());
        for (label, ty) in fields {
            if let Some((old, value)) = out.last_mut()
                && *old == label
            {
                *value = self.inter(*value, ty);
                if self.plain_empty(*value) {
                    return Id::NEVER;
                }
            } else {
                if self.plain_empty(ty) {
                    return Id::NEVER;
                }
                out.push((label, ty));
            }
        }
        self.intern(Node::Rec(out))
    }

    pub fn union(&mut self, a: Id, b: Id) -> Id {
        if a == b || b == Id::NEVER {
            return a;
        }
        if a == Id::NEVER {
            return b;
        }
        if a == Id::TOP || b == Id::TOP {
            return Id::TOP;
        }
        self.intern(if a < b {
            Node::Union(a, b)
        } else {
            Node::Union(b, a)
        })
    }

    pub fn inter(&mut self, a: Id, b: Id) -> Id {
        if a == b || !matches!((self.node(a), self.node(b)), (Node::Rec(_), Node::Rec(_))) {
            return self.inter_atom(a, b);
        }
        let pair = |a: Id, b: Id| if a < b { (a, b) } else { (b, a) };
        let root = pair(a, b);
        let mut pending = vec![(root, false)];
        let mut done = FxHashMap::default();
        while let Some(((a, b), finish)) = pending.pop() {
            if done.contains_key(&(a, b)) {
                continue;
            }
            let (Node::Rec(mut left), Node::Rec(right)) =
                (self.node(a).clone(), self.node(b).clone())
            else {
                let out = self.inter_atom(a, b);
                done.insert((a, b), out);
                continue;
            };
            if a == b {
                done.insert((a, b), a);
                continue;
            }
            if !finish {
                pending.push(((a, b), true));
                for (label, ty) in &left {
                    if let Ok(i) = right.binary_search_by(|(key, _)| key.cmp(label)) {
                        pending.push((pair(*ty, right[i].1), false));
                    }
                }
                continue;
            }
            for (label, ty) in &mut left {
                if let Ok(i) = right.binary_search_by(|(key, _)| key.cmp(label)) {
                    *ty = done[&pair(*ty, right[i].1)];
                }
            }
            for (label, ty) in right {
                if left.binary_search_by(|(key, _)| key.cmp(&label)).is_err() {
                    let i = left.partition_point(|(key, _)| key < &label);
                    left.insert(i, (label, ty));
                }
            }
            let out = self.record(left);
            done.insert((a, b), out);
        }
        done[&root]
    }

    fn inter_atom(&mut self, a: Id, b: Id) -> Id {
        if a == b || b == Id::TOP {
            return a;
        }
        if a == Id::TOP {
            return b;
        }
        if a == Id::NEVER || b == Id::NEVER {
            return Id::NEVER;
        }
        if b == Id::RECORD && self.record_domain(a) {
            return a;
        }
        if a == Id::RECORD && self.record_domain(b) {
            return b;
        }
        if (self.record_domain(a) && matches!(self.node(b), Node::Con(..) | Node::Fn(_)))
            || (self.record_domain(b) && matches!(self.node(a), Node::Con(..) | Node::Fn(_)))
        {
            return Id::NEVER;
        }
        match (self.node(a).clone(), self.node(b).clone()) {
            (Node::Rec(_), Node::Con(..) | Node::Fn(_))
            | (Node::Con(..) | Node::Fn(_), Node::Rec(_)) => Id::NEVER,
            (Node::Con(..), Node::Fn(_)) | (Node::Fn(_), Node::Con(..)) => Id::NEVER,
            (Node::Con(ha, ..), Node::Con(hb, ..)) if ha != hb => Id::NEVER,
            (Node::Con(ha, va, aa), Node::Con(hb, vb, bb))
                if ha == hb && aa.is_empty() && bb.is_empty() =>
            {
                match va.meet(&vb) {
                    Some(values) => self.con(ha, values, Vec::new()),
                    None => Id::NEVER,
                }
            }
            _ => self.intern(if a < b {
                Node::Inter(a, b)
            } else {
                Node::Inter(b, a)
            }),
        }
    }

    pub fn without(&mut self, base: Id, mut labels: Vec<Label>) -> Id {
        labels.sort();
        labels.dedup();
        if labels.is_empty() {
            return self.inter(base, Id::RECORD);
        }
        match self.node(base).clone() {
            Node::Never | Node::Con(..) | Node::Fn(_) => Id::NEVER,
            Node::Top => Id::RECORD,
            Node::Without(inner, mut earlier) => {
                earlier.extend(labels);
                self.without(inner, earlier)
            }
            Node::Inter(a, b) if a == Id::RECORD || b == Id::RECORD => {
                self.without(if a == Id::RECORD { b } else { a }, labels)
            }
            Node::Rec(fields)
                if fields
                    .iter()
                    .all(|(l, _)| labels.iter().all(|removed| !l.may_equal(removed)))
                    && fields
                        .iter()
                        .all(|(l, t)| !labels.contains(l) || self.inhabited(*t)) =>
            {
                self.record(
                    fields
                        .into_iter()
                        .filter(|(l, _)| !labels.contains(l))
                        .collect(),
                )
            }
            _ => self.intern(Node::Without(base, labels)),
        }
    }

    pub fn difference(&mut self, base: Id, heads: Vec<Id>) -> Id {
        self.intern(Node::Diff(base, heads))
    }

    pub fn both(&mut self, a: Id, b: Id) -> Id {
        if a == b {
            return a;
        }
        self.intern(Node::Both(a, b))
    }

    /// One node map, one variable callback and one label map for a whole scheme.
    pub fn substitute(
        &mut self,
        root: Id,
        variables: &mut impl FnMut(&mut Self, V) -> Id,
        labels: &FxHashMap<LabelParam, Label>,
    ) -> Id {
        let mut done = FxHashMap::default();
        let mut pending = vec![(root, false)];
        while let Some((id, finish)) = pending.pop() {
            if done.contains_key(&id) {
                continue;
            }
            let node = self.node(id).clone();
            if !finish {
                pending.push((id, true));
                pending.extend(node.children().into_iter().map(|child| (child, false)));
                continue;
            }
            let label = |l| match l {
                Label::Param(p) => labels.get(&p).cloned().unwrap_or(Label::Param(p)),
                l => l,
            };
            let out = match node {
                Node::Never | Node::Top => id,
                Node::Var(v) => variables(self, v),
                Node::Con(h, vs, parts) => self.con(h, vs, parts.iter().map(|t| done[t]).collect()),
                Node::Fn(f) => self.function(Function {
                    params: f
                        .params
                        .into_iter()
                        .map(|(name, t)| (name, done[&t]))
                        .collect(),
                    ret: done[&f.ret],
                    err: done[&f.err],
                    env: done[&f.env],
                }),
                Node::Rec(fields) => self.record(
                    fields
                        .into_iter()
                        .map(|(l, t)| (label(l), done[&t]))
                        .collect(),
                ),
                Node::Union(a, b) => self.union(done[&a], done[&b]),
                Node::Inter(a, b) => self.inter(done[&a], done[&b]),
                Node::Without(base, ls) => {
                    self.without(done[&base], ls.into_iter().map(label).collect())
                }
                Node::Diff(base, hs) => {
                    self.difference(done[&base], hs.iter().map(|h| done[h]).collect())
                }
                Node::Both(a, b) => self.both(done[&a], done[&b]),
            };
            done.insert(id, out);
        }
        done[&root]
    }

    #[cfg(test)]
    pub fn reachable(&self, root: Id) -> usize {
        let mut seen = FxHashSet::default();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if seen.insert(id) {
                pending.extend(self.node(id).children());
            }
        }
        seen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::ModuleId;

    fn int(g: &mut Arena) -> Id {
        g.con(
            Head::Named(TypeRef::Builtin(BuiltinType::Int)),
            ValueSet::ALL,
            Vec::new(),
        )
    }

    fn param() -> LabelParam {
        LabelParam {
            owner: BindingRef {
                module: ModuleId(0),
                index: 0,
            }.into(),
            index: 0,
            level: 1,
            name: "K".into(),
        }
    }

    #[test]
    fn source_products_keep_empty_and_unknown_witnesses_distinct() {
        let mut g = Arena::default();
        let int = int(&mut g);
        let text = g.con(
            Head::Named(TypeRef::Builtin(BuiltinType::String)),
            ValueSet::ALL,
            Vec::new(),
        );
        assert_eq!(g.record(vec![(Label::name("a"), Id::NEVER)]), Id::NEVER);
        let empty_tuple = g.con(Head::Tuple(1), ValueSet::ALL, vec![Id::NEVER]);
        assert!(matches!(
            g.node(empty_tuple),
            Node::Con(Head::Tuple(1), _, _)
        ));
        assert_eq!(g.record(vec![(Label::name("a"), empty_tuple)]), Id::NEVER);
        assert_eq!(
            g.record(vec![(Label::name("a"), int), (Label::name("a"), text)]),
            Id::NEVER
        );
        let a = g.var(V(0));
        let product = g.record(vec![(Label::name("a"), a), (Label::name("b"), int)]);
        let forgotten = g.without(product, vec![Label::name("a")]);
        assert!(matches!(g.node(forgotten), Node::Without(..)));
        for (replacement, expected) in [
            (Id::NEVER, Id::NEVER),
            (int, g.record(vec![(Label::name("b"), int)])),
        ] {
            let rewritten = g.substitute(forgotten, &mut |_, _| replacement, &FxHashMap::default());
            assert_eq!(rewritten, expected);
        }
        // The closed empty record is inhabited; an empty field is not.
        let source = g.record(vec![(Label::name("a"), Id::RECORD)]);
        assert_eq!(g.without(source, vec![Label::name("a")]), Id::RECORD);
    }

    #[test]
    fn one_substitution_preserves_invariant_and_function_occurrences() {
        let mut g = Arena::default();
        let a = g.var(V(0));
        let cell = g.con(
            Head::Named(TypeRef::Custom(crate::hir::TypeId(0))),
            ValueSet::ALL,
            vec![a],
        );
        let f = g.function(Function {
            params: vec![(None, a)],
            ret: a,
            err: Id::NEVER,
            env: Id::NEVER,
        });
        let left = g.record(vec![(Label::name("cell"), cell)]);
        let right = g.record(vec![(Label::name("use"), f)]);
        let root = g.union(left, right);
        let replacement = g.var(V(1));
        let mut calls = 0;
        let changed = g.substitute(
            root,
            &mut |_, v| {
                assert_eq!(v, V(0));
                calls += 1;
                replacement
            },
            &FxHashMap::default(),
        );
        assert_eq!(calls, 1);
        let Node::Union(l, r) = g.node(changed) else {
            panic!("exact branches")
        };
        let Node::Rec(l) = g.node(*l) else {
            panic!("record")
        };
        let Node::Rec(r) = g.node(*r) else {
            panic!("record")
        };
        let (cell, function) = if l[0].0 == Label::name("cell") {
            (l[0].1, r[0].1)
        } else {
            (r[0].1, l[0].1)
        };
        let Node::Con(_, _, args) = g.node(cell) else {
            panic!("cell")
        };
        let Node::Fn(f) = g.node(function) else {
            panic!("function")
        };
        assert_eq!(args, &[replacement]);
        assert_eq!(f.params[0].1, replacement);
        assert_eq!(f.ret, replacement);
    }

    #[test]
    fn label_substitution_merges_collisions_without_forgetting_a_variable() {
        let mut g = Arena::default();
        let int = int(&mut g);
        let key = param();
        let a = g.var(V(0));
        let source = g.record(vec![
            (Label::Param(key.clone()), a),
            (Label::name("a"), int),
        ]);
        let replaced = g.substitute(
            source,
            &mut |_, _| Id::NEVER,
            &FxHashMap::from_iter([(key, Label::name("a"))]),
        );
        assert_eq!(replaced, Id::NEVER);
        let unknown_key = g.without(source, vec![Label::name("a")]);
        assert!(matches!(g.node(unknown_key), Node::Without(..)));
        let key = Label::Param(param());
        let single = g.record(vec![(key.clone(), int)]);
        assert_eq!(g.without(single, vec![key.clone()]), Id::RECORD);
        assert_eq!(g.without(Id::RECORD, vec![key.clone()]), Id::RECORD);
        assert_eq!(g.without(Id::TOP, vec![key]), Id::RECORD);
    }

    #[test]
    fn source_fan_and_branch_families_stay_shared_before_demand() {
        let mut g = Arena::default();
        let a = g.var(V(0));
        let int = int(&mut g);
        let mut fan = Id::RECORD;
        let mut branch = Id::RECORD;
        for n in 0..20 {
            let la = Label::name(format!("a{n:02}"));
            let lb = Label::name(format!("b{n:02}"));
            let left = g.record(vec![(la.clone(), a)]);
            let right = g.record(vec![(lb.clone(), int)]);
            let choice = g.union(left, right);
            fan = g.inter(fan, choice);
            let base_a = g.without(branch, vec![la.clone()]);
            let base_b = g.without(branch, vec![lb.clone()]);
            let add_a = g.record(vec![(la, int)]);
            let add_b = g.record(vec![(lb, int)]);
            let next_a = g.inter(base_a, add_a);
            let next_b = g.inter(base_b, add_b);
            branch = g.union(next_a, next_b);
        }
        assert!(g.reachable(fan) < 100);
        assert!(g.reachable(branch) < 160);
        let closed = g.substitute(fan, &mut |_, _| Id::NEVER, &FxHashMap::default());
        assert!(g.reachable(closed) < 25);
        assert!(g.len() < 400);
    }

    #[test]
    fn deep_graph_substitution_has_no_native_recursion() {
        let mut g = Arena::default();
        let mut root = g.var(V(0));
        for _ in 0..20_000 {
            root = g.con(Head::Tuple(1), ValueSet::ALL, vec![root]);
        }
        let changed = g.substitute(root, &mut |_, _| Id::TOP, &FxHashMap::default());
        assert_eq!(g.reachable(changed), 20_001);
        assert!(g.inhabited(changed));
    }
}

#[cfg(test)]
mod differential {
    use super::*;
    use std::{path::Path, process::Command};

    // Same finite representatives as the independent oracle: absent, 1, 2,
    // a non-Int. Raw source types, including forgetting, get set meanings.
    fn den(g: &Arena, root: Id, done: &mut FxHashMap<Id, (u8, u64)>) -> (u8, u64) {
        let mut pending = vec![(root, false)];
        while let Some((id, finish)) = pending.pop() {
            if done.contains_key(&id) {
                continue;
            }
            let node = g.node(id);
            if !finish {
                pending.push((id, true));
                pending.extend(node.children().into_iter().map(|child| (child, false)));
                continue;
            }
            let value = match node {
                Node::Never => (0, 0),
                Node::Top => (7, u64::MAX),
                Node::Con(Head::Named(TypeRef::Builtin(BuiltinType::Int)), values, _) => (
                    u8::from(values.contains("1")) | (u8::from(values.contains("2")) << 1),
                    0,
                ),
                Node::Rec(fields) => {
                    let mut rows = 0;
                    for row in 0..64 {
                        if fields.iter().all(|(label, ty)| {
                            let Label::Name(label) = label else {
                                panic!("closed label")
                            };
                            let index = (label.as_bytes()[0] - b'a') as usize;
                            let value = (row >> (2 * (2 - index))) & 3;
                            value != 0 && done[ty].0 & (1 << (value - 1)) != 0
                        }) {
                            rows |= 1 << row;
                        }
                    }
                    (0, rows)
                }
                Node::Union(a, b) => (done[a].0 | done[b].0, done[a].1 | done[b].1),
                Node::Inter(a, b) => (done[a].0 & done[b].0, done[a].1 & done[b].1),
                Node::Without(base, labels) => {
                    let mut keep = 63;
                    for label in labels {
                        let Label::Name(label) = label else {
                            panic!("closed label")
                        };
                        let index = (label.as_bytes()[0] - b'a') as usize;
                        keep &= !(3 << (2 * (2 - index)));
                    }
                    let mut rests = [false; 64];
                    for row in 0..64 {
                        if done[base].1 & (1 << row) != 0 {
                            rests[row & keep] = true;
                        }
                    }
                    (
                        0,
                        (0..64)
                            .filter(|row| rests[row & keep])
                            .fold(0, |bits, row| bits | (1 << row)),
                    )
                }
                _ => panic!("finite oracle's closed flat grammar"),
            };
            done.insert(id, value);
        }
        done[&root]
    }

    #[test]
    fn source_graph_matches_both_accepted_models() {
        let oracle =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/t3-record-graph-oracle.py");
        let output = Command::new("python3").arg(oracle).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut g = Arena::default();
        let int = g.con(
            Head::Named(TypeRef::Builtin(BuiltinType::Int)),
            ValueSet::ALL,
            Vec::new(),
        );
        let one = g.con(
            Head::Named(TypeRef::Builtin(BuiltinType::Int)),
            ValueSet::only(["1".into()]).unwrap(),
            Vec::new(),
        );
        let var = g.var(V(0));
        let mut nodes = Vec::new();
        let mut values = FxHashMap::default();
        let mut checks = 0;
        for line in String::from_utf8(output.stdout).unwrap().lines() {
            let words: Vec<_> = line.split_whitespace().collect();
            let number = |i: usize| words[i].parse::<usize>().unwrap();
            let label = |i| Label::name(char::from(b'a' + i as u8).to_string());
            if words[0] == "check" {
                let actual = den(&g, nodes[number(1)], &mut values);
                assert_eq!(
                    actual,
                    (words[2].parse().unwrap(), words[3].parse().unwrap()),
                    "{line}"
                );
                checks += 1;
                continue;
            }
            assert_eq!(words[0], "node");
            let id = match words[1] {
                "never" => Id::NEVER,
                "top" => Id::TOP,
                "int" => int,
                "one" => one,
                "var" => var,
                "rec" => g.record(
                    (0..number(2))
                        .map(|i| (label(number(3 + 2 * i)), nodes[number(4 + 2 * i)]))
                        .collect(),
                ),
                "union" => g.union(nodes[number(2)], nodes[number(3)]),
                "inter" => g.inter(nodes[number(2)], nodes[number(3)]),
                "without" => g.without(
                    nodes[number(2)],
                    (0..3)
                        .filter(|i| number(3) & (1 << i) != 0)
                        .map(label)
                        .collect(),
                ),
                "subst" => {
                    let replacement = nodes[number(3)];
                    g.substitute(
                        nodes[number(2)],
                        &mut |_, _| replacement,
                        &FxHashMap::default(),
                    )
                }
                _ => panic!("oracle protocol"),
            };
            nodes.push(id);
        }
        assert_eq!(checks, 36_328);
    }
}
