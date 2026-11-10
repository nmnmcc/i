//! Finite and cofinite subsets of one infinite scalar domain.
//!
//! `None` is the empty set in operation results; `ValueSet(None)` is All.
//! Keys are decoded strings or canonical decimal integers. The persistent
//! B-tree copies only the search path when a stored bound gains one value.

use num_bigint::{BigInt, BigUint};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LitValue {
    Int(BigInt),
    Str(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TestValues {
    All,
    Only(Vec<LitValue>),
    Except(Vec<LitValue>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct ValueSet(Option<Lits>);

#[derive(Clone, Debug)]
struct Lits(Arc<LitsInner>);

#[derive(Debug)]
struct LitsInner {
    except: bool,
    items: Set,
}

impl PartialEq for Lits {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.0.except == other.0.except
                && self.0.items.hash == other.0.items.hash
                && self.0.items.len == other.0.items.len
                && self.0.items.iter().eq(other.0.items.iter()))
    }
}

impl Eq for Lits {}

impl Hash for Lits {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.0.except.hash(h);
        h.write_u64(self.0.items.hash);
    }
}

impl ValueSet {
    pub const ALL: Self = Self(None);

    pub fn only(items: impl IntoIterator<Item = Arc<str>>) -> Option<Self> {
        Self::from_set(false, Set::from_items(items))
    }

    pub fn except(items: impl IntoIterator<Item = Arc<str>>) -> Self {
        Self::from_set(true, Set::from_items(items)).unwrap()
    }

    fn from_set(except: bool, items: Set) -> Option<Self> {
        if items.len == 0 {
            except.then_some(Self::ALL)
        } else {
            Some(Self(Some(Lits(Arc::new(LitsInner { except, items })))))
        }
    }

    pub fn is_all(&self) -> bool {
        self.0.is_none()
    }

    pub fn is_finite(&self) -> bool {
        self.0.as_ref().is_some_and(|l| !l.0.except)
    }

    /// The listed values, not the (possibly infinite) domain of the set.
    pub fn items(&self) -> impl Iterator<Item = &Arc<str>> {
        self.0.iter().flat_map(|l| l.0.items.iter())
    }

    pub fn contains(&self, value: &str) -> bool {
        self.0
            .as_ref()
            .is_none_or(|l| l.0.items.contains(value) != l.0.except)
    }

    pub fn union(&self, other: &Self) -> Self {
        let (Some(a), Some(b)) = (&self.0, &other.0) else {
            return Self::ALL;
        };
        let (a, b) = (&a.0, &b.0);
        let (except, items) = match (a.except, b.except) {
            (false, false) => (false, a.items.combine(&b.items, Op::Union)),
            (false, true) => (true, b.items.combine(&a.items, Op::Minus)),
            (true, false) => (true, a.items.combine(&b.items, Op::Minus)),
            (true, true) => (true, a.items.combine(&b.items, Op::Meet)),
        };
        Self::from_set(except, items).unwrap()
    }

    pub fn meet(&self, other: &Self) -> Option<Self> {
        let (a, b) = match (&self.0, &other.0) {
            (None, _) => return Some(other.clone()),
            (_, None) => return Some(self.clone()),
            (Some(a), Some(b)) => (&a.0, &b.0),
        };
        let (except, items) = match (a.except, b.except) {
            (false, false) => (false, a.items.combine(&b.items, Op::Meet)),
            (false, true) => (false, a.items.combine(&b.items, Op::Minus)),
            (true, false) => (false, b.items.combine(&a.items, Op::Minus)),
            (true, true) => (true, a.items.combine(&b.items, Op::Union)),
        };
        Self::from_set(except, items)
    }

    pub fn complement(&self) -> Option<Self> {
        let l = self.0.as_ref()?;
        Self::from_set(!l.0.except, l.0.items.clone())
    }

    pub fn minus(&self, other: &Self) -> Option<Self> {
        self.meet(&other.complement()?)
    }

    pub fn subset(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (_, None) => true,
            (None, _) => false,
            (Some(a), Some(b)) => match (a.0.except, b.0.except) {
                (false, _) => a.0.items.iter().all(|v| other.contains(v)),
                (true, false) => false,
                (true, true) => b.0.items.iter().all(|v| a.0.items.contains(v)),
            },
        }
    }

    pub fn disjoint(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), _) if !a.0.except => a.0.items.iter().all(|v| !other.contains(v)),
            (_, Some(b)) if !b.0.except => b.0.items.iter().all(|v| !self.contains(v)),
            _ => false,
        }
    }

    /// Start with the largest set so adding singletons shares its tree.
    pub fn join_all(sets: &[Self]) -> Self {
        if sets.is_empty() || sets.iter().any(Self::is_all) {
            // The empty union has no ValueSet representation. Callers only
            // invoke this for an occupied head.
            assert!(!sets.is_empty(), "join_all needs a nonempty list");
            return Self::ALL;
        }
        let largest = sets
            .iter()
            .enumerate()
            .max_by_key(|(_, s)| s.0.as_ref().unwrap().0.items.len)
            .unwrap()
            .0;
        let mut out = sets[largest].clone();
        for (i, s) in sets.iter().enumerate() {
            if i != largest {
                out = out.union(s);
            }
        }
        out
    }
}

pub(crate) fn int_text(negative: bool, value: &BigUint) -> String {
    if negative && value.bits() != 0 {
        format!("-{value}")
    } else {
        value.to_string()
    }
}

// A B-tree of minimum degree 8: each non-root node has 7..15 keys.
// Arc::make_mut clones only a bounded number of keys per visited node.
const DEGREE: usize = 8;
const MAX: usize = 2 * DEGREE - 1;

#[derive(Clone, Debug, Default)]
struct Set {
    root: Option<Arc<Node>>,
    len: usize,
    hash: u64,
}

#[derive(Clone, Debug, Default)]
struct Node {
    keys: Vec<Arc<str>>,
    children: Vec<Arc<Node>>,
}

#[derive(Clone, Copy)]
enum Op {
    Union,
    Meet,
    Minus,
}

fn item_hash(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

impl Set {
    fn from_items(items: impl IntoIterator<Item = Arc<str>>) -> Self {
        let mut keys: Vec<_> = items.into_iter().collect();
        keys.sort_unstable();
        keys.dedup();
        Self::from_sorted(keys)
    }

    /// Build a balanced tree in linear time after a sorted merge.
    fn from_sorted(keys: Vec<Arc<str>>) -> Self {
        let len = keys.len();
        if len == 0 {
            return Self::default();
        }
        let hash = keys.iter().fold(0u64, |h, v| h.wrapping_add(item_hash(v)));
        let mut height = 0;
        let mut capacity = MAX;
        while capacity < len {
            height += 1;
            capacity = capacity.saturating_mul(2 * DEGREE).saturating_add(MAX);
        }
        Self {
            root: Some(Node::from_sorted(&mut keys.into_iter(), len, height, true)),
            len,
            hash,
        }
    }

    fn iter(&self) -> Iter<'_> {
        let mut it = Iter { stack: Vec::new() };
        if let Some(root) = &self.root {
            it.left(root);
        }
        it
    }

    fn contains(&self, key: &str) -> bool {
        let mut node = self.root.as_deref();
        while let Some(n) = node {
            match n.keys.binary_search_by(|v| v.as_ref().cmp(key)) {
                Ok(_) => return true,
                Err(i) => node = n.children.get(i).map(Arc::as_ref),
            }
        }
        false
    }

    fn insert(&mut self, key: Arc<str>) {
        if self.contains(&key) {
            return;
        }
        self.len += 1;
        self.hash = self.hash.wrapping_add(item_hash(&key));
        let root = self.root.get_or_insert_with(Arc::default);
        if root.keys.len() == MAX {
            let mut next = Node {
                keys: Vec::new(),
                children: vec![root.clone()],
            };
            next.split(0);
            *root = Arc::new(next);
        }
        Arc::make_mut(root).insert(key);
    }

    fn remove(&mut self, key: &str) {
        if !self.contains(key) {
            return;
        }
        self.len -= 1;
        self.hash = self.hash.wrapping_sub(item_hash(key));
        let root = self.root.as_mut().unwrap();
        Arc::make_mut(root).remove(key);
        if root.keys.is_empty() {
            self.root = root.children.first().cloned();
        }
    }

    fn combine(&self, other: &Self, op: Op) -> Self {
        let (small, large) = if self.len <= other.len {
            (self, other)
        } else {
            (other, self)
        };
        if small.len.saturating_mul(8) < large.len {
            return match op {
                Op::Union => {
                    let mut out = large.clone();
                    for key in small.iter() {
                        out.insert(key.clone());
                    }
                    out
                }
                Op::Meet => {
                    let mut out = small.clone();
                    for key in small.iter() {
                        if !large.contains(key) {
                            out.remove(key);
                        }
                    }
                    out
                }
                Op::Minus => {
                    let mut out = self.clone();
                    if self.len <= other.len {
                        for key in self.iter() {
                            if other.contains(key) {
                                out.remove(key);
                            }
                        }
                    } else {
                        for key in other.iter() {
                            out.remove(key);
                        }
                    }
                    out
                }
            };
        }
        let mut a = self.iter().peekable();
        let mut b = other.iter().peekable();
        let mut keys = Vec::new();
        while let (Some(x), Some(y)) = (a.peek(), b.peek()) {
            match x.cmp(y) {
                std::cmp::Ordering::Less => {
                    let x = a.next().unwrap();
                    if !matches!(op, Op::Meet) {
                        keys.push(x.clone());
                    }
                }
                std::cmp::Ordering::Equal => {
                    let x = a.next().unwrap();
                    b.next();
                    if !matches!(op, Op::Minus) {
                        keys.push(x.clone());
                    }
                }
                std::cmp::Ordering::Greater => {
                    let y = b.next().unwrap();
                    if matches!(op, Op::Union) {
                        keys.push(y.clone());
                    }
                }
            }
        }
        if !matches!(op, Op::Meet) {
            keys.extend(a.cloned());
        }
        if matches!(op, Op::Union) {
            keys.extend(b.cloned());
        }
        Self::from_sorted(keys)
    }
}

impl Node {
    fn from_sorted(
        keys: &mut impl Iterator<Item = Arc<str>>,
        len: usize,
        height: u32,
        root: bool,
    ) -> Arc<Self> {
        if height == 0 {
            return Arc::new(Self {
                keys: keys.take(len).collect(),
                children: Vec::new(),
            });
        }
        let min_child = DEGREE.pow(height) - 1;
        let max_child = (2 * DEGREE).pow(height) - 1;
        let count = (if root { 1 } else { DEGREE - 1 }..=MAX)
            .find(|&k| len >= k && (k + 1) * min_child <= len - k && len - k <= (k + 1) * max_child)
            .unwrap();
        let base = (len - count) / (count + 1);
        let extra = (len - count) % (count + 1);
        let mut out = Self::default();
        for i in 0..=count {
            out.children.push(Self::from_sorted(
                keys,
                base + usize::from(i < extra),
                height - 1,
                false,
            ));
            if i < count {
                out.keys.push(keys.next().unwrap());
            }
        }
        Arc::new(out)
    }

    fn split(&mut self, i: usize) {
        let child = Arc::make_mut(&mut self.children[i]);
        let right_keys = child.keys.split_off(DEGREE);
        let middle = child.keys.pop().unwrap();
        let right_children = if child.children.is_empty() {
            Vec::new()
        } else {
            child.children.split_off(DEGREE)
        };
        self.keys.insert(i, middle);
        self.children.insert(
            i + 1,
            Arc::new(Self {
                keys: right_keys,
                children: right_children,
            }),
        );
    }

    fn insert(&mut self, key: Arc<str>) {
        let mut i = self.keys.binary_search(&key).unwrap_err();
        if self.children.is_empty() {
            self.keys.insert(i, key);
            return;
        }
        if self.children[i].keys.len() == MAX {
            self.split(i);
            if key > self.keys[i] {
                i += 1;
            }
        }
        Arc::make_mut(&mut self.children[i]).insert(key);
    }

    fn edge(&self, last: bool) -> Arc<str> {
        let mut node = self;
        while !node.children.is_empty() {
            node = if last {
                node.children.last().unwrap()
            } else {
                &node.children[0]
            };
        }
        if last {
            node.keys.last().unwrap().clone()
        } else {
            node.keys[0].clone()
        }
    }

    fn merge(&mut self, i: usize) {
        let middle = self.keys.remove(i);
        let right = self.children.remove(i + 1);
        let left = Arc::make_mut(&mut self.children[i]);
        left.keys.push(middle);
        left.keys.extend(right.keys.iter().cloned());
        left.children.extend(right.children.iter().cloned());
    }

    fn remove(&mut self, key: &str) {
        match self.keys.binary_search_by(|v| v.as_ref().cmp(key)) {
            Ok(i) if self.children.is_empty() => {
                self.keys.remove(i);
            }
            Ok(i) => {
                if self.children[i].keys.len() >= DEGREE {
                    let previous = self.children[i].edge(true);
                    Arc::make_mut(&mut self.children[i]).remove(&previous);
                    self.keys[i] = previous;
                } else if self.children[i + 1].keys.len() >= DEGREE {
                    let next = self.children[i + 1].edge(false);
                    Arc::make_mut(&mut self.children[i + 1]).remove(&next);
                    self.keys[i] = next;
                } else {
                    self.merge(i);
                    Arc::make_mut(&mut self.children[i]).remove(key);
                }
            }
            Err(mut i) => {
                if self.children[i].keys.len() == DEGREE - 1 {
                    if i > 0 && self.children[i - 1].keys.len() >= DEGREE {
                        let sibling = Arc::make_mut(&mut self.children[i - 1]);
                        let middle =
                            std::mem::replace(&mut self.keys[i - 1], sibling.keys.pop().unwrap());
                        let child = sibling.children.pop();
                        let target = Arc::make_mut(&mut self.children[i]);
                        target.keys.insert(0, middle);
                        if let Some(child) = child {
                            target.children.insert(0, child);
                        }
                    } else if i + 1 < self.children.len()
                        && self.children[i + 1].keys.len() >= DEGREE
                    {
                        let sibling = Arc::make_mut(&mut self.children[i + 1]);
                        let middle = std::mem::replace(&mut self.keys[i], sibling.keys.remove(0));
                        let child = if sibling.children.is_empty() {
                            None
                        } else {
                            Some(sibling.children.remove(0))
                        };
                        let target = Arc::make_mut(&mut self.children[i]);
                        target.keys.push(middle);
                        if let Some(child) = child {
                            target.children.push(child);
                        }
                    } else {
                        if i == self.keys.len() {
                            i -= 1;
                        }
                        self.merge(i);
                    }
                }
                Arc::make_mut(&mut self.children[i]).remove(key);
            }
        }
    }
}

struct Iter<'a> {
    stack: Vec<(&'a Node, usize)>,
}

impl<'a> Iter<'a> {
    fn left(&mut self, mut node: &'a Node) {
        loop {
            self.stack.push((node, 0));
            let Some(child) = node.children.first() else {
                break;
            };
            node = child;
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Arc<str>;

    fn next(&mut self) -> Option<Self::Item> {
        let (node, i) = self.stack.pop()?;
        let key = &node.keys[i];
        if i + 1 < node.keys.len() {
            self.stack.push((node, i + 1));
        }
        if let Some(child) = node.children.get(i + 1) {
            self.left(child);
        }
        Some(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn keys(mask: u8) -> Vec<Arc<str>> {
        (0..4)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| format!("v{i}").into())
            .collect()
    }

    fn model(set: &Option<ValueSet>) -> u8 {
        // Bit 4 is ★, all unlisted values of the infinite domain.
        (0..5).fold(0, |mask, i| {
            mask | if set.as_ref().is_some_and(|s| s.contains(&format!("v{i}"))) {
                1 << i
            } else {
                0
            }
        })
    }

    #[test]
    fn exhaustive_infinite_domain_model() {
        // literal-types §10: F(X), E(X), All, and an outside value ★.
        let sets: Vec<_> = (0..16)
            .flat_map(|mask| {
                [
                    ValueSet::only(keys(mask)),
                    Some(ValueSet::except(keys(mask))),
                ]
            })
            .chain([Some(ValueSet::ALL)])
            .collect();
        for a in &sets {
            for b in &sets {
                let (x, y) = (model(a), model(b));
                let union = match (a, b) {
                    (Some(a), Some(b)) => Some(a.union(b)),
                    (a, b) => a.clone().or_else(|| b.clone()),
                };
                assert_eq!(model(&union), x | y);
                let meet = a.as_ref().and_then(|a| a.meet(b.as_ref()?));
                assert_eq!(model(&meet), x & y);
                let minus = match (a, b) {
                    (Some(a), Some(b)) => a.minus(b),
                    (a, _) => a.clone(),
                };
                assert_eq!(model(&minus), x & !y);
                if let (Some(a), Some(b)) = (a, b) {
                    assert_eq!(a.subset(b), x & !y == 0);
                    assert_eq!(a.disjoint(b), x & y == 0);
                    assert_eq!(
                        model(&Some(ValueSet::join_all(&[a.clone(), b.clone()]))),
                        x | y
                    );
                }
            }
            if let Some(a) = a {
                assert_eq!(model(&a.complement()), 31 ^ model(&Some(a.clone())));
            }
        }
    }

    #[test]
    fn canonical_sets_and_hashes() {
        assert_eq!(
            std::mem::size_of::<ValueSet>(),
            std::mem::size_of::<usize>()
        );
        assert_eq!(ValueSet::only([]), None);
        assert_eq!(ValueSet::except([]), ValueSet::ALL);
        let a = ValueSet::only(["b", "a", "a"].map(Arc::from)).unwrap();
        let b = ValueSet::only(["a", "b"].map(Arc::from)).unwrap();
        assert_eq!(a, b);
        let hash = |v: &ValueSet| {
            let mut h = DefaultHasher::new();
            v.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(&a), hash(&b));
        let mut collision = Set::from_items([Arc::from("different")]);
        collision.hash = a.0.as_ref().unwrap().0.items.hash;
        collision.len = a.0.as_ref().unwrap().0.items.len;
        assert_ne!(a, ValueSet::from_set(false, collision).unwrap());
        assert_eq!(a.items().map(AsRef::as_ref).collect::<Vec<_>>(), ["a", "b"]);
        assert!(a.is_finite());
        assert!(!ValueSet::except([Arc::from("a")]).is_finite());
        assert_eq!(TestValues::All, TestValues::All);
        assert_ne!(
            TestValues::Only(vec![LitValue::Int(1.into())]),
            TestValues::Except(vec![LitValue::Str("1".into())])
        );
    }

    #[test]
    fn canonical_literal_values() {
        // literal-types decision 12: decimal numbers and decoded scalars.
        let hex = BigUint::parse_bytes(b"10", 16).unwrap();
        assert_eq!(int_text(false, &hex), "16");
        assert_eq!(int_text(true, &BigUint::from(0u8)), "0");
        assert_eq!(int_text(true, &hex), "-16");
        let (m, diags) =
            ilang_syntax::parse_module("a = \"\\u{e9}\"\nb = \"é\"\nc = \"e\\u{301}\"\n");
        assert!(diags.is_empty(), "{diags:?}");
        let strings: Vec<_> = m
            .items
            .iter()
            .map(|item| {
                let ilang_syntax::ast::ItemKind::Binding(b) = &item.kind else {
                    panic!()
                };
                let ilang_syntax::ast::ExprKind::String(s) = &b.value.kind else {
                    panic!()
                };
                ValueSet::only([Arc::from(s.as_str())]).unwrap()
            })
            .collect();
        assert_eq!(strings[0], strings[1]);
        assert_ne!(strings[0], strings[2]);
    }

    fn check_tree(set: &Set, model: &BTreeSet<Arc<str>>) {
        fn check(node: &Node, root: bool) -> (usize, usize) {
            assert!(!node.keys.is_empty());
            assert!(node.keys.len() <= MAX);
            assert!(root || node.keys.len() >= DEGREE - 1);
            assert!(node.keys.windows(2).all(|w| w[0] < w[1]));
            if node.children.is_empty() {
                return (0, node.keys.len());
            }
            assert_eq!(node.children.len(), node.keys.len() + 1);
            let children: Vec<_> = node.children.iter().map(|c| check(c, false)).collect();
            assert!(children.iter().all(|c| c.0 == children[0].0));
            (
                children[0].0 + 1,
                node.keys.len() + children.iter().map(|c| c.1).sum::<usize>(),
            )
        }
        assert_eq!(
            set.iter().collect::<Vec<_>>(),
            model.iter().collect::<Vec<_>>()
        );
        assert_eq!(set.len, model.len());
        assert_eq!(
            set.hash,
            model.iter().fold(0u64, |h, v| h.wrapping_add(item_hash(v)))
        );
        if let Some(root) = &set.root {
            assert_eq!(check(root, true).1, set.len);
        } else {
            assert_eq!(set.len, 0);
        }
    }

    #[test]
    fn persistent_tree_insert_remove_and_bulk_build() {
        let mut set = Set::default();
        let mut model = BTreeSet::new();
        let mut snapshots = Vec::new();
        // Deterministic permutations exercise insertion, borrowing and merging
        // at both sides of an internal node, not just the append path.
        for i in 0..4096 {
            let key: Arc<str> = format!("{:04}", (i * 109) % 4096).into();
            set.insert(key.clone());
            model.insert(key);
            if i % 127 == 0 {
                check_tree(&set, &model);
                snapshots.push((set.clone(), model.clone()));
            }
        }
        for n in 1..=1024 {
            let small: BTreeSet<_> = model.iter().take(n).cloned().collect();
            check_tree(&Set::from_sorted(small.iter().cloned().collect()), &small);
        }
        for n in [1025, 4095, 4096] {
            let subset: BTreeSet<_> = model.iter().take(n).cloned().collect();
            check_tree(&Set::from_sorted(subset.iter().cloned().collect()), &subset);
        }
        for i in 0..4096 {
            let key = format!("{:04}", (i * 53) % 4096);
            set.remove(&key);
            model.remove(key.as_str());
            if i % 127 == 0 || i == 4095 {
                check_tree(&set, &model);
            }
        }
        for (set, model) in snapshots {
            check_tree(&set, &model);
        }
    }

    #[test]
    fn algebra_on_large_shared_sets() {
        let a = ValueSet::only((0..1000).map(|n| Arc::from(n.to_string()))).unwrap();
        let b = ValueSet::only((500..1500).map(|n| Arc::from(n.to_string()))).unwrap();
        let one = ValueSet::only([Arc::from("1001")]).unwrap();
        assert_eq!(a.union(&b).items().count(), 1500);
        assert_eq!(a.meet(&b).unwrap().items().count(), 500);
        assert_eq!(a.minus(&b).unwrap().items().count(), 500);
        assert_eq!(a.union(&one).items().count(), 1001);
        assert_eq!(one.meet(&a), None);
        assert_eq!(a.minus(&one), Some(a.clone()));
        assert_eq!(one.minus(&a), Some(one.clone()));
        assert_eq!(a.meet(&a.union(&one)), Some(a));
    }
}
