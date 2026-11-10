//! Lazy Boolean decisions, following Elixir descr.ex's bdd_* equations.
//! A decision denotes (atom & yes) | either | (!atom & no).
//! Complement here is relative to the record domain, not written subtraction.

use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct Bdd(u32);

impl Bdd {
    pub const EMPTY: Self = Self(0);
    pub const ALL: Self = Self(1);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Op {
    Union,
    Inter,
    Difference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Goal(Op, Bdd, Bdd);

impl Goal {
    fn new(op: Op, a: Bdd, b: Bdd) -> Self {
        if op != Op::Difference && a > b {
            Self(op, b, a)
        } else {
            Self(op, a, b)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Decision {
    pub atom: u32,
    pub yes: Bdd,
    pub either: Bdd,
    pub no: Bdd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ResourceLimit;

/// One operation owns this budget, including every recursive suboperation.
/// Memoization does not refund or skip a logical state. Completed results are
/// scoped to this demand; no mutable-bound answer survives into another one.
pub(super) struct Demand {
    limit: usize,
    states: FxHashSet<Goal>,
    completed: Option<FxHashMap<Goal, Bdd>>,
}

impl Demand {
    pub const LIMIT: usize = 65_536;

    pub fn new(limit: usize, cache: bool) -> Self {
        Self {
            limit,
            states: FxHashSet::default(),
            completed: cache.then(FxHashMap::default),
        }
    }

    fn charge(&mut self, goal: Goal) -> Result<(), ResourceLimit> {
        if !self.states.contains(&goal) {
            if self.states.len() == self.limit {
                return Err(ResourceLimit);
            }
            self.states.insert(goal);
        }
        Ok(())
    }

    pub fn work(&self) -> usize {
        self.states.len()
    }
}

#[derive(Default)]
pub(super) struct Kernel {
    // Handles 0 and 1 are terminals; decisions start at 2.
    nodes: Vec<Decision>,
    unique: FxHashMap<Decision, Bdd>,
}

// These recipes have bounded depth (one apply expansion), independently of
// decision depth. Both their evaluation and graph traversal use work stacks.
enum Recipe {
    Value(Bdd),
    Apply(Op, Box<Recipe>, Box<Recipe>),
    Node(u32, Box<[Recipe; 3]>),
}

impl Recipe {
    fn value(t: Bdd) -> Self {
        Self::Value(t)
    }

    fn op(op: Op, a: Self, b: Self) -> Self {
        Self::Apply(op, Box::new(a), Box::new(b))
    }

    fn pair(op: Op, a: Bdd, b: Bdd) -> Self {
        Self::op(op, Self::value(a), Self::value(b))
    }

    fn node(atom: u32, yes: Self, either: Self, no: Self) -> Self {
        Self::Node(atom, Box::new([yes, either, no]))
    }
}

enum Task {
    Goal(Goal),
    Recipe(Recipe),
    Apply(Op),
    Node(u32),
    Save(Goal),
}

impl Kernel {
    pub fn decision(&self, t: Bdd) -> Option<Decision> {
        t.0.checked_sub(2).map(|i| self.nodes[i as usize])
    }

    fn intern(&mut self, node: Decision) -> Bdd {
        if let Some(t) = self.unique.get(&node) {
            return *t;
        }
        let t = Bdd(u32::try_from(self.nodes.len() + 2).expect("decision handle overflow"));
        self.nodes.push(node);
        self.unique.insert(node, t);
        t
    }

    pub fn atom(&mut self, atom: u32) -> Bdd {
        self.intern(Decision {
            atom,
            yes: Bdd::ALL,
            either: Bdd::EMPTY,
            no: Bdd::EMPTY,
        })
    }

    pub fn apply(
        &mut self,
        op: Op,
        a: Bdd,
        b: Bdd,
        demand: &mut Demand,
    ) -> Result<Bdd, ResourceLimit> {
        let mut tasks = vec![Task::Goal(Goal::new(op, a, b))];
        let mut values = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Goal(goal @ Goal(op, a, b)) => {
                    demand.charge(goal)?;
                    if let Some(t) = demand.completed.as_ref().and_then(|m| m.get(&goal)) {
                        values.push(*t);
                        continue;
                    }
                    tasks.push(Task::Save(goal));
                    let recipe = self.recipe(op, a, b);
                    tasks.push(Task::Recipe(recipe));
                }
                Task::Recipe(recipe) => match recipe {
                    Recipe::Value(t) => values.push(t),
                    Recipe::Apply(op, a, b) => {
                        tasks.push(Task::Apply(op));
                        tasks.push(Task::Recipe(*b));
                        tasks.push(Task::Recipe(*a));
                    }
                    Recipe::Node(atom, parts) => {
                        let [yes, either, no] = *parts;
                        tasks.push(Task::Node(atom));
                        tasks.push(Task::Recipe(no));
                        tasks.push(Task::Recipe(either));
                        tasks.push(Task::Recipe(yes));
                    }
                },
                Task::Apply(op) => {
                    let b = values.pop().expect("right operand");
                    let a = values.pop().expect("left operand");
                    tasks.push(Task::Goal(Goal::new(op, a, b)));
                }
                Task::Node(atom) => {
                    let no = values.pop().expect("dual child");
                    let either = values.pop().expect("union child");
                    let yes = values.pop().expect("constrained child");
                    if either == Bdd::ALL {
                        values.push(Bdd::ALL);
                    } else if yes == no {
                        tasks.push(Task::Goal(Goal::new(Op::Union, yes, either)));
                    } else {
                        values.push(self.intern(Decision {
                            atom,
                            yes,
                            either,
                            no,
                        }));
                    }
                }
                Task::Save(goal) => {
                    if let Some(memo) = &mut demand.completed {
                        memo.insert(goal, *values.last().expect("completed apply"));
                    }
                }
            }
        }
        debug_assert_eq!(values.len(), 1);
        Ok(values.pop().expect("root result"))
    }

    fn recipe(&self, op: Op, a: Bdd, b: Bdd) -> Recipe {
        use Recipe as R;
        match op {
            Op::Union => {
                if a == b || b == Bdd::EMPTY {
                    return R::value(a);
                }
                if a == Bdd::EMPTY {
                    return R::value(b);
                }
                if a == Bdd::ALL || b == Bdd::ALL {
                    return R::value(Bdd::ALL);
                }
            }
            Op::Inter => {
                if a == b || b == Bdd::ALL {
                    return R::value(a);
                }
                if a == Bdd::ALL {
                    return R::value(b);
                }
                if a == Bdd::EMPTY || b == Bdd::EMPTY {
                    return R::value(Bdd::EMPTY);
                }
            }
            Op::Difference => {
                if a == Bdd::EMPTY || a == b || b == Bdd::ALL {
                    return R::value(Bdd::EMPTY);
                }
                if b == Bdd::EMPTY {
                    return R::value(a);
                }
                if a == Bdd::ALL {
                    let d = self.decision(b).expect("nonterminal complement");
                    let inner = R::node(
                        d.atom,
                        R::pair(Op::Difference, Bdd::ALL, d.yes),
                        R::value(Bdd::EMPTY),
                        R::pair(Op::Difference, Bdd::ALL, d.no),
                    );
                    return R::op(
                        Op::Inter,
                        inner,
                        R::pair(Op::Difference, Bdd::ALL, d.either),
                    );
                }
            }
        }
        let x = self.decision(a).expect("nonterminal left operand");
        let y = self.decision(b).expect("nonterminal right operand");
        if x.atom == y.atom {
            if op == Op::Union || (op == Op::Inter && x.either == y.either) {
                return R::node(
                    x.atom,
                    R::pair(op, x.yes, y.yes),
                    if op == Op::Union {
                        R::pair(op, x.either, y.either)
                    } else {
                        R::value(x.either)
                    },
                    R::pair(op, x.no, y.no),
                );
            }
            return R::node(
                x.atom,
                R::op(
                    op,
                    R::pair(Op::Union, x.yes, x.either),
                    R::pair(Op::Union, y.yes, y.either),
                ),
                R::value(Bdd::EMPTY),
                R::op(
                    op,
                    R::pair(Op::Union, x.no, x.either),
                    R::pair(Op::Union, y.no, y.either),
                ),
            );
        }
        if op == Op::Union {
            let (first, other) = if x.atom < y.atom { (x, b) } else { (y, a) };
            return R::node(
                first.atom,
                R::value(first.yes),
                R::pair(op, first.either, other),
                R::value(first.no),
            );
        }
        if x.atom < y.atom {
            return R::node(
                x.atom,
                R::pair(op, x.yes, b),
                R::pair(op, x.either, b),
                R::pair(op, x.no, b),
            );
        }
        if op == Op::Inter {
            return R::node(
                y.atom,
                R::pair(op, a, y.yes),
                R::pair(op, a, y.either),
                R::pair(op, a, y.no),
            );
        }
        // S minus (p&C | U | !p&D): remove U once, then split on p.
        R::node(
            y.atom,
            R::op(op, R::pair(op, a, y.either), R::value(y.yes)),
            R::value(Bdd::EMPTY),
            R::op(op, R::pair(op, a, y.either), R::value(y.no)),
        )
    }

    pub fn truth(&self, root: Bdd, assignment: impl Fn(u32) -> bool) -> bool {
        let mut pending = vec![root];
        let mut seen = FxHashSet::default();
        while let Some(t) = pending.pop() {
            if t == Bdd::ALL {
                return true;
            }
            if seen.insert(t)
                && let Some(d) = self.decision(t)
            {
                pending.push(d.either);
                pending.push(if assignment(d.atom) { d.yes } else { d.no });
            }
        }
        false
    }

    /// Rebuild changed predicates through apply, rather than retaining an old
    /// decision order after replacement (records §21.2; Elixir bdd_map differs).
    pub fn substitute(
        &mut self,
        root: Bdd,
        replacements: &FxHashMap<u32, Bdd>,
        demand: &mut Demand,
    ) -> Result<Bdd, ResourceLimit> {
        let mut done = FxHashMap::from_iter([(Bdd::EMPTY, Bdd::EMPTY), (Bdd::ALL, Bdd::ALL)]);
        let mut pending = vec![(root, false)];
        while let Some((t, finish)) = pending.pop() {
            if done.contains_key(&t) {
                continue;
            }
            let d = self.decision(t).expect("nonterminal substitution");
            if !finish {
                pending.push((t, true));
                pending.extend([d.yes, d.either, d.no].map(|child| (child, false)));
                continue;
            }
            let atom = replacements
                .get(&d.atom)
                .copied()
                .unwrap_or_else(|| self.atom(d.atom));
            let positive = self.apply(Op::Inter, atom, done[&d.yes], demand)?;
            let negative_atom = self.apply(Op::Difference, Bdd::ALL, atom, demand)?;
            let negative = self.apply(Op::Inter, negative_atom, done[&d.no], demand)?;
            let arms = self.apply(Op::Union, positive, negative, demand)?;
            let out = self.apply(Op::Union, arms, done[&d.either], demand)?;
            done.insert(t, out);
        }
        Ok(done[&root])
    }

    pub fn reachable(&self, root: Bdd) -> usize {
        let mut pending = vec![root];
        let mut seen = FxHashSet::default();
        while let Some(t) = pending.pop() {
            if let Some(d) = self.decision(t)
                && seen.insert(t)
            {
                pending.extend([d.yes, d.either, d.no]);
            }
        }
        seen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::Path, process::Command};

    fn mask(k: &Kernel, root: Bdd) -> u8 {
        (0..8).fold(0, |bits, n| {
            bits | (u8::from(k.truth(root, |atom| n & (1 << (2 - atom)) != 0)) << n)
        })
    }

    #[test]
    fn differential_boolean_operations_against_the_accepted_model() {
        let oracle =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/t3-record-bdd-oracle.py");
        let output = Command::new("python3").arg(oracle).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let protocol = String::from_utf8(output.stdout).unwrap();
        for cache in [false, true] {
            let mut k = Kernel::default();
            let mut forms = vec![Bdd::EMPTY, Bdd::ALL, k.atom(0), k.atom(1), k.atom(2)];
            let mut checks = 0;
            for line in protocol.lines() {
                let words: Vec<_> = line.split_whitespace().collect();
                assert_eq!(words.len(), 5);
                let op = match words[1] {
                    "or" => Op::Union,
                    "and" => Op::Inter,
                    "diff" => Op::Difference,
                    _ => panic!("unknown oracle operation"),
                };
                let a: usize = words[2].parse().unwrap();
                let b: usize = words[3].parse().unwrap();
                let expected: u8 = words[4].parse().unwrap();
                let mut demand = Demand::new(Demand::LIMIT, cache);
                let root = k.apply(op, forms[a], forms[b], &mut demand).unwrap();
                assert_eq!(mask(&k, root), expected, "with cache {cache}, case {line}");
                if words[0] == "build" {
                    forms.push(root);
                } else {
                    assert_eq!(words[0], "check");
                    checks += 1;
                }
            }
            assert_eq!(forms.len(), 80);
            assert_eq!(checks, 19_200);
        }
    }

    #[test]
    fn demand_accounting_and_resource_retry_do_not_change_truth() {
        let mut k = Kernel::default();
        let a = k.atom(0);
        let b = k.atom(1);
        let mut limited = Demand::new(1, true);
        assert_eq!(k.apply(Op::Inter, a, b, &mut limited), Err(ResourceLimit));
        let mut work = Vec::new();
        for cache in [false, true] {
            let mut demand = Demand::new(Demand::LIMIT, cache);
            let root = k.apply(Op::Inter, a, b, &mut demand).unwrap();
            assert_eq!(mask(&k, root), 0b1100_0000);
            let before = demand.work();
            assert_eq!(k.apply(Op::Inter, a, b, &mut demand).unwrap(), root);
            assert_eq!(demand.work(), before);
            work.push(before);
        }
        assert_eq!(work[0], work[1]);
        let mut retry = Demand::new(Demand::LIMIT, true);
        let not_a = k.apply(Op::Difference, Bdd::ALL, a, &mut retry).unwrap();
        assert_eq!(mask(&k, not_a), 0b0000_1111);
    }

    #[test]
    fn demanded_fan_keeps_the_lazy_union_branch() {
        let mut k = Kernel::default();
        let mut demand = Demand::new(Demand::LIMIT, true);
        let mut fan = Bdd::ALL;
        for n in 0..20 {
            let a = k.atom(2 * n);
            let b = k.atom(2 * n + 1);
            let choice = k.apply(Op::Union, a, b, &mut demand).unwrap();
            fan = k.apply(Op::Inter, fan, choice, &mut demand).unwrap();
        }
        assert_eq!(k.reachable(fan), 40);
        assert!(k.truth(fan, |_| true));
        assert!(!k.truth(fan, |_| false));
    }

    #[test]
    fn substitution_rebuilds_atom_order_and_allows_empty_or_compound_predicates() {
        let mut k = Kernel::default();
        let [a, b, c] = [k.atom(0), k.atom(1), k.atom(2)];
        let mut demand = Demand::new(Demand::LIMIT, true);
        let union = k.apply(Op::Union, a, b, &mut demand).unwrap();
        let root = k.apply(Op::Difference, union, c, &mut demand).unwrap();
        let replacements = FxHashMap::from_iter([(0, c), (1, a), (2, b)]);
        let renamed = k.substitute(root, &replacements, &mut demand).unwrap();
        for n in 0..8 {
            let values = [n & 4 != 0, n & 2 != 0, n & 1 != 0];
            assert_eq!(
                k.truth(renamed, |atom| values[atom as usize]),
                (values[2] || values[0]) && !values[1]
            );
        }
        for replacement in [Bdd::EMPTY, Bdd::ALL, b, c, union] {
            for op in [Op::Union, Op::Inter, Op::Difference] {
                let root = k.apply(op, a, b, &mut demand).unwrap();
                let map = FxHashMap::from_iter([(0, replacement)]);
                let changed = k.substitute(root, &map, &mut demand).unwrap();
                let rebuilt = k.apply(op, replacement, b, &mut demand).unwrap();
                assert_eq!(mask(&k, changed), mask(&k, rebuilt));
            }
        }
    }

    #[test]
    fn deep_decisions_use_an_explicit_work_stack() {
        let mut k = Kernel::default();
        let mut root = Bdd::ALL;
        for atom in (0..20_000).rev() {
            root = k.intern(Decision {
                atom,
                yes: root,
                either: Bdd::EMPTY,
                no: Bdd::EMPTY,
            });
        }
        let atom = k.atom(0);
        let mut demand = Demand::new(Demand::LIMIT, true);
        let difference = k.apply(Op::Difference, root, atom, &mut demand).unwrap();
        assert_eq!(difference, Bdd::EMPTY);
        let mut demand = Demand::new(Demand::LIMIT, true);
        let complement = k
            .apply(Op::Difference, Bdd::ALL, root, &mut demand)
            .unwrap();
        assert!(!k.truth(complement, |_| true));
        assert!(k.truth(complement, |_| false));
        assert!(k.truth(root, |_| true));
        assert_eq!(k.reachable(root), 20_000);
    }
}
