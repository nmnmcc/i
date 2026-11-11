//! Read-only structural proofs and Elixir-style negative product elimination.
//!
//! All recursive questions are suspended on a work stack. A recursive active
//! question is unknown; it is never an empty-set or coinductive proof.

use super::bdd::{Bdd, Kernel, Op};
use super::{Demand, Id, Label, Node, Pool, ResourceLimit};
use crate::hir::TypeRef;
use crate::types::{
    Head, Ty, V, Variance,
    lits::ValueSet,
    solve::{Mode, Solver},
};
use rustc_hash::{FxHashMap as Map, FxHashSet as Set};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Proven,
    Refuted,
    Unknown,
    ResourceLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Empty {
    Yes,
    No,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Cell {
    absent: bool,
    pos: Vec<Id>,
    neg: Vec<Id>,
}

impl Cell {
    fn free() -> Self {
        Self {
            absent: true,
            pos: vec![],
            neg: vec![],
        }
    }
    fn required(t: Id) -> Self {
        Self {
            absent: false,
            pos: vec![t],
            neg: vec![],
        }
    }
    fn normalized(mut self) -> Self {
        self.pos.sort_unstable();
        self.pos.dedup();
        self.neg.sort_unstable();
        self.neg.dedup();
        self
    }
    fn meet(&self, b: &Self) -> Self {
        Self {
            absent: self.absent && b.absent,
            pos: self.pos.iter().chain(&b.pos).copied().collect(),
            neg: self.neg.iter().chain(&b.neg).copied().collect(),
        }
        .normalized()
    }
    // A \ (present & positives & !negatives). Each outcome is one field
    // predicate, not a product of choices over all record fields.
    fn difference(&self, b: &Self, index: usize) -> Self {
        if index == 0 {
            return Self {
                absent: self.absent && !b.absent,
                pos: vec![Id::NEVER],
                neg: vec![],
            };
        }
        let mut c = self.clone();
        c.absent = false;
        if let Some(t) = b.pos.get(index - 1) {
            c.neg.push(*t);
        } else {
            c.pos.push(b.neg[index - 1 - b.pos.len()]);
        }
        c.normalized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hir::{BuiltinType, TypeRef},
        types::lits::LitValue,
    };

    fn int() -> Ty {
        Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![])
    }
    fn string() -> Ty {
        Ty::named(TypeRef::Builtin(BuiltinType::String), vec![])
    }
    fn lit(n: i32) -> Ty {
        Ty::literal(&LitValue::Int(n.into()))
    }
    fn solver() -> Solver {
        Solver::new(Default::default(), vec![])
    }
    fn record(s: &mut Solver, key: &str, ty: Ty) -> Ty {
        s.record(vec![(Label::name(key), ty)])
    }
    fn verdict(s: &Solver, a: &Ty, b: &Ty) -> Verdict {
        let snapshot = s.vars.clone();
        let mut work = vec![];
        let mut verdicts = vec![];
        for cache in [false, true] {
            let mut demand = Demand::new(Demand::LIMIT, cache);
            verdicts.push(prove_with_cache(s, a, b, &mut demand, cache));
            work.push(demand.work());
        }
        assert_eq!(snapshot, s.vars);
        assert_eq!(verdicts[0], verdicts[1]);
        assert_eq!(work[0], work[1]);
        verdicts[0]
    }

    #[test]
    fn scalar_residual_and_wrap_union_are_exact_without_new_bounds() {
        let mut s = solver();
        let both = s.union(vec![int(), string()]);
        let src = record(&mut s, "a", both);
        let ai = record(&mut s, "a", int());
        let az = record(&mut s, "a", string());
        let target = s.union(vec![ai, az]);
        assert_eq!(verdict(&s, &src, &target), Verdict::Proven);
        s.constrain(&src, &target);
        assert!(s.failures.is_empty());
        let src = record(&mut s, "a", int());
        let one = lit(1);
        let other = int().with_values(int().values().minus(one.values()));
        let a = record(&mut s, "a", one);
        let b = record(&mut s, "a", other);
        let target = s.union(vec![a, b]);
        assert_eq!(verdict(&s, &src, &target), Verdict::Proven);
    }

    #[test]
    fn missing_presence_and_nested_tuple_partition_keep_witnesses() {
        let mut s = solver();
        let all = s.record(vec![]);
        let a = record(&mut s, "a", int());
        let b = record(&mut s, "b", string());
        let target = s.union(vec![a, b]);
        assert_eq!(verdict(&s, &all, &target), Verdict::Refuted);
        let pair = Ty::tuple(vec![s.union(vec![int(), string()]), int()]);
        let src = record(&mut s, "data", pair);
        let a = record(&mut s, "data", Ty::tuple(vec![int(), int()]));
        let b = record(&mut s, "data", Ty::tuple(vec![string(), int()]));
        let target = s.union(vec![a, b]);
        assert_eq!(verdict(&s, &src, &target), Verdict::Proven);
    }

    #[test]
    fn established_upper_bounds_prove_coverage_but_lower_bounds_do_not() {
        let mut s = solver();
        let v = s.fresh();
        let src = record(&mut s, "a", v.clone());
        let a = record(&mut s, "a", int());
        let b = record(&mut s, "a", string());
        let target = s.union(vec![a, b]);
        assert_eq!(verdict(&s, &src, &target), Verdict::Unknown);
        s.constrain(&int(), &v);
        assert_eq!(verdict(&s, &src, &target), Verdict::Unknown);
        let upper = s.union(vec![int(), string()]);
        s.constrain(&v, &upper);
        assert_eq!(verdict(&s, &src, &target), Verdict::Proven);
    }

    #[test]
    fn whole_row_upper_bounds_support_coverage_through_forgetting() {
        let mut s = solver();
        let ai = record(&mut s, "a", int());
        let az = record(&mut s, "a", string());
        let rows = s.union(vec![ai, az]);
        let input = s.fresh();
        s.constrain(&input, &rows);
        let changed = s.without(&input, vec![Label::name("b")]);
        let field = record(&mut s, "b", int());
        let changed = s.inter(vec![changed, field.clone()]);
        let ai = s.record(vec![(Label::name("a"), int()), (Label::name("b"), int())]);
        let az = s.record(vec![
            (Label::name("a"), string()),
            (Label::name("b"), int()),
        ]);
        let target = s.union(vec![ai, az]);
        assert_eq!(verdict(&s, &changed, &target), Verdict::Proven);
        let lower_only = s.fresh();
        s.constrain(&rows, &lower_only);
        let changed = s.without(&lower_only, vec![Label::name("b")]);
        let changed = s.inter(vec![changed, field]);
        assert_eq!(verdict(&s, &changed, &target), Verdict::Unknown);
    }

    #[test]
    fn forgetting_preserves_a_common_source_witness() {
        let mut s = solver();
        let v = record(&mut s, "a", Ty::tuple(vec![Ty::Never]));
        assert_eq!(v, Ty::Never);
        let bottom = s.without(&v, vec![Label::name("a")]);
        assert_eq!(verdict(&s, &bottom, &Ty::Never), Verdict::Proven);
        let one = record(&mut s, "a", lit(1));
        let two = record(&mut s, "a", lit(2));
        let impossible = s.records.inter(&[one, two]);
        let forgotten = s.without(&impossible, vec![Label::name("a")]);
        assert_eq!(verdict(&s, &forgotten, &Ty::Never), Verdict::Proven);
    }

    #[test]
    fn a_function_witness_does_not_change_the_fixed_e4_predicate() {
        let mut s = solver();
        let function = Ty::Fn(Box::new(crate::types::FnTy {
            params: vec![],
            ret: Ty::Never,
            err: Ty::Never,
            env: Ty::Never,
        }));
        let row = record(&mut s, "cb", function);
        let forgotten = s.without(&row, vec![Label::name("cb")]);
        assert!(matches!(&forgotten, Ty::Graph(g) if matches!(g.node(), Node::Without(..))));
        assert_eq!(verdict(&s, &forgotten, &Ty::Never), Verdict::Refuted);
        assert!(matches!(&forgotten, Ty::Graph(g) if matches!(g.node(), Node::Without(..))));
    }

    #[test]
    fn exhaustion_is_not_refutation_and_retry_uses_a_fresh_budget() {
        let mut s = solver();
        let a = record(&mut s, "a", int());
        let b = record(&mut s, "b", string());
        let src = s.record(vec![]);
        let target = s.union(vec![a, b]);
        let mut demand = Demand::new(1, true);
        assert_eq!(
            prove(&s, &src, &target, &mut demand),
            Verdict::ResourceLimit
        );
        assert_eq!(verdict(&s, &src, &target), Verdict::Refuted);
    }

    #[test]
    fn differential_inclusions_and_negative_projections_against_the_accepted_model() {
        let oracle = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/t3-record-proof-oracle.py");
        let output = std::process::Command::new("python3")
            .arg(oracle)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        let mut s = solver();
        let leaves = [lit(1), lit(2), Ty::literal(&LitValue::Str("x".into()))];
        let fields: Vec<_> = (0..8)
            .map(|bits| {
                s.union(
                    leaves
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| bits & (1 << i) != 0)
                        .map(|(_, t)| t.clone())
                        .collect(),
                )
            })
            .collect();
        let mut roots = vec![];
        for a in 0..8 {
            for b in 0..8 {
                let fs = [("a", a), ("b", b)]
                    .into_iter()
                    .filter(|(_, bits)| *bits != 0)
                    .map(|(key, bits)| (Label::name(key), fields[bits].clone()))
                    .collect();
                roots.push(s.record(fs));
            }
        }
        let mut inclusions = 0;
        for line in text.lines().filter(|line| line.starts_with("inclusion ")) {
            let parts: Vec<_> = line.split_whitespace().collect();
            let i: usize = parts[1].parse().unwrap();
            let j: usize = parts[2].parse().unwrap();
            let mask: u64 = parts[3].parse().unwrap();
            let target = s.records.union(&[roots[i].clone(), roots[j].clone()]);
            for (k, source) in roots.iter().enumerate() {
                let expected = if mask & (1 << k) != 0 {
                    Verdict::Proven
                } else {
                    Verdict::Refuted
                };
                let mut demand = Demand::new(Demand::LIMIT, true);
                assert_eq!(
                    prove(&s, source, &target, &mut demand),
                    expected,
                    "{line}, source {k}"
                );
                inclusions += 1;
            }
        }
        assert_eq!(inclusions, 133_120);
        let mut demand = Demand::new(Demand::LIMIT, true);
        let mut e = Engine::new(&s, &mut demand, true);
        let mut bdds = vec![];
        for root in &roots {
            let id = e.pool.import(root);
            let Answer::Diagram(Some(bdd)) = e.run(Goal::Force(id)).unwrap() else {
                panic!("closed force")
            };
            bdds.push(bdd);
        }
        let mut projections = 0;
        let mut values = Map::from_iter([(Id::NEVER, 0u8), (Id::TOP, 7u8)]);
        let mut masks: Vec<u16> = Vec::new();
        for line in text.lines().filter(|line| line.starts_with("projection ")) {
            let parts: Vec<_> = line.split_whitespace().collect();
            let i: usize = parts[1].parse().unwrap();
            let j: usize = parts[2].parse().unwrap();
            let labels: usize = parts[3].parse().unwrap();
            let expected: u16 = parts[4].parse().unwrap();
            *e.demand = Demand::new(Demand::LIMIT, true);
            e.scope = e.demand.new_scope();
            e.complete.clear();
            e.goals.clear();
            let root = e
                .kernel
                .apply(Op::Difference, bdds[i], bdds[j], e.demand)
                .unwrap();
            let free = e
                .product(vec![])
                .unwrap_or_else(|_| panic!("product budget"));
            let keys = ["a", "b"]
                .into_iter()
                .enumerate()
                .filter(|(i, _)| labels & (1 << i) != 0)
                .map(|(_, key)| Label::name(key))
                .collect();
            let Answer::Diagram(Some(projected)) =
                e.run(Goal::Walk(root, free, vec![], Some(keys))).unwrap()
            else {
                panic!("closed projection: {line}")
            };
            for fields in &e.products[masks.len()..] {
                for (_, cell) in fields {
                    for id in cell.pos.iter().chain(&cell.neg) {
                        value_mask(&e, &mut values, *id);
                    }
                }
                let mut mask = 0;
                for (a, x) in [1u8, 2, 4, 8].into_iter().enumerate() {
                    for (b, y) in [1u8, 2, 4, 8].into_iter().enumerate() {
                        if fields.iter().all(|(label, cell)| {
                            let value = if label == &Label::name("a") { x } else { y };
                            if value == 8 {
                                cell.absent
                            } else {
                                cell.pos.iter().all(|t| values[t] & value != 0)
                                    && cell.neg.iter().all(|t| values[t] & value == 0)
                            }
                        }) {
                            mask |= 1 << (4 * a + b);
                        }
                    }
                }
                masks.push(mask);
            }
            let actual = (0..16).fold(0u16, |mask, row| {
                mask | (u16::from(
                    e.kernel
                        .truth(projected, |atom| masks[atom as usize] & (1 << row) != 0),
                ) << row)
            });
            assert_eq!(actual, expected, "{line}");
            projections += 1;
        }
        assert_eq!(projections, 16_384);
    }

    fn value_mask(e: &Engine<'_>, done: &mut Map<Id, u8>, root: Id) {
        let mut pending = vec![(root, false)];
        while let Some((id, finish)) = pending.pop() {
            if done.contains_key(&id) {
                continue;
            }
            let node = e.node(id);
            if !finish {
                pending.push((id, true));
                pending.extend(node.children().into_iter().map(|id| (id, false)));
                continue;
            }
            let mask = match node {
                Node::Con(Head::Named(TypeRef::Builtin(BuiltinType::Int)), vs, _) => {
                    u8::from(vs.contains("1")) | (u8::from(vs.contains("2")) << 1)
                }
                Node::Con(Head::Named(TypeRef::Builtin(BuiltinType::String)), vs, _) => {
                    u8::from(vs.contains("x")) << 2
                }
                Node::Union(a, b) => done[&a] | done[&b],
                Node::Inter(a, b) => done[&a] & done[&b],
                _ => panic!("unexpected model leaf {node:?}"),
            };
            done.insert(id, mask);
        }
    }
}

type Product = Vec<(Label, Cell)>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Goal {
    Rel(Id, Id, Mode),
    Enclose(Id, bool),
    Empty(Vec<Id>, Vec<Id>),
    Force(Id),
    Walk(Bdd, u32, Vec<u32>, Option<Vec<Label>>),
    Line(u32, Vec<u32>, Option<Vec<Label>>),
}

#[derive(Clone, Copy, Debug)]
enum Answer {
    Rel(bool),
    Empty(Empty),
    Diagram(Option<Bdd>),
    Type(Id),
}

impl Goal {
    fn unknown(&self) -> Answer {
        match self {
            Self::Rel(..) => Answer::Rel(false),
            Self::Enclose(_, upper) => Answer::Type(if *upper { Id::TOP } else { Id::NEVER }),
            Self::Empty(..) | Self::Walk(_, _, _, None) | Self::Line(_, _, None) => {
                Answer::Empty(Empty::Unknown)
            }
            _ => Answer::Diagram(None),
        }
    }
}

enum Pause {
    Need(Goal),
    Limit,
}
impl From<ResourceLimit> for Pause {
    fn from(_: ResourceLimit) -> Self {
        Self::Limit
    }
}
type Step = Result<Answer, Pause>;

#[derive(Default)]
struct Frame {
    deps: Map<Goal, Answer>,
    tainted: bool,
}

struct Engine<'a> {
    solver: &'a Solver,
    pool: Pool,
    demand: &'a mut Demand,
    scope: u64,
    kernel: Kernel,
    products: Vec<Product>,
    product_ids: Map<Product, u32>,
    goals: Map<Goal, u64>,
    complete: Map<Goal, Answer>,
    active: Set<Goal>,
    cache: bool,
    labels: Map<Label, u64>,
}

pub(crate) fn prove(s: &Solver, lhs: &Ty, rhs: &Ty, demand: &mut Demand) -> Verdict {
    prove_mode(s, lhs, rhs, Mode::Value, demand)
}

pub(crate) fn prove_mode(
    s: &Solver,
    lhs: &Ty,
    rhs: &Ty,
    mode: Mode,
    demand: &mut Demand,
) -> Verdict {
    prove_with_cache_mode(s, lhs, rhs, mode, demand, true)
}

#[cfg(test)]
fn prove_with_cache(s: &Solver, lhs: &Ty, rhs: &Ty, demand: &mut Demand, cache: bool) -> Verdict {
    prove_with_cache_mode(s, lhs, rhs, Mode::Value, demand, cache)
}

fn prove_with_cache_mode(
    s: &Solver,
    lhs: &Ty,
    rhs: &Ty,
    mode: Mode,
    demand: &mut Demand,
    cache: bool,
) -> Verdict {
    let pool = s.records.clone();
    let a = pool.import(lhs);
    let b = pool.import(rhs);
    let mut e = Engine::new(s, demand, cache);
    match e.run(Goal::Rel(a, b, mode)) {
        Ok(Answer::Rel(true)) => Verdict::Proven,
        Err(_) => Verdict::ResourceLimit,
        _ if mode == Mode::Value => e.coverage(a, b).unwrap_or(Verdict::ResourceLimit),
        _ => Verdict::Unknown,
    }
}

impl Engine<'_> {
    fn coverage(&mut self, a: Id, b: Id) -> Result<Verdict, ResourceLimit> {
        let Answer::Type(may_source) = self.run(Goal::Enclose(a, true))? else {
            unreachable!()
        };
        let Answer::Type(must_target) = self.run(Goal::Enclose(b, false))? else {
            unreachable!()
        };
        if matches!(
            self.run(Goal::Empty(vec![may_source], vec![must_target]))?,
            Answer::Empty(Empty::Yes)
        ) {
            return Ok(Verdict::Proven);
        }
        let Answer::Type(must_source) = self.run(Goal::Enclose(a, false))? else {
            unreachable!()
        };
        let Answer::Type(may_target) = self.run(Goal::Enclose(b, true))? else {
            unreachable!()
        };
        Ok(
            if matches!(
                self.run(Goal::Empty(vec![must_source], vec![may_target]))?,
                Answer::Empty(Empty::No)
            ) {
                Verdict::Refuted
            } else {
                Verdict::Unknown
            },
        )
    }
    fn new<'a>(s: &'a Solver, demand: &'a mut Demand, cache: bool) -> Engine<'a> {
        let scope = demand.new_scope();
        Engine {
            solver: s,
            pool: s.records.clone(),
            demand,
            scope,
            kernel: Kernel::default(),
            products: vec![],
            product_ids: Map::default(),
            goals: Map::default(),
            complete: Map::default(),
            active: Set::default(),
            cache,
            labels: Map::default(),
        }
    }
    fn node(&self, id: Id) -> Node {
        self.pool.node(id)
    }

    fn charge(&mut self, goal: &Goal) -> Result<(), ResourceLimit> {
        if !self.goals.contains_key(goal) {
            let id = self.goals.len() as u64;
            self.demand.charge_state(&[5, self.scope, id])?;
            self.goals.insert(goal.clone(), id);
        }
        Ok(())
    }

    fn run(&mut self, root: Goal) -> Result<Answer, ResourceLimit> {
        self.charge(&root)?;
        let mut stack = vec![(root.clone(), Frame::default())];
        self.active.insert(root);
        while let Some((goal, mut frame)) = stack.pop() {
            match self.compute(&goal, &mut frame) {
                Ok(answer) => {
                    self.active.remove(&goal);
                    if self.cache && !frame.tainted {
                        self.complete.insert(goal.clone(), answer);
                    }
                    if let Some((_, parent)) = stack.last_mut() {
                        parent.deps.insert(goal, answer);
                        parent.tainted |= frame.tainted;
                    } else {
                        return Ok(answer);
                    }
                }
                Err(Pause::Need(child)) => {
                    self.charge(&child)?;
                    self.active.insert(child.clone());
                    stack.push((goal, frame));
                    stack.push((child, Frame::default()));
                }
                Err(Pause::Limit) => return Err(ResourceLimit),
            }
        }
        unreachable!()
    }

    fn need(&self, goal: Goal, f: &mut Frame) -> Step {
        if let Some(answer) = f.deps.get(&goal).or_else(|| self.complete.get(&goal)) {
            return Ok(*answer);
        }
        if self.active.contains(&goal) {
            f.tainted = true;
            return Ok(goal.unknown());
        }
        Err(Pause::Need(goal))
    }

    fn rel(&self, a: Id, b: Id, env: Mode, f: &mut Frame) -> Result<bool, Pause> {
        match self.need(Goal::Rel(a, b, env), f)? {
            Answer::Rel(true) => Ok(true),
            Answer::Rel(false)
                if env == Mode::Value
                    && self.pool.record_domain(a)
                    && self.pool.record_domain(b) =>
            {
                match self.need(Goal::Empty(vec![a], vec![b]), f)? {
                    Answer::Empty(Empty::Yes) => Ok(true),
                    Answer::Empty(_) => Ok(false),
                    _ => unreachable!(),
                }
            }
            Answer::Rel(false) => Ok(false),
            _ => unreachable!(),
        }
    }
    fn empty(&self, mut p: Vec<Id>, mut n: Vec<Id>, f: &mut Frame) -> Result<Empty, Pause> {
        p.sort_unstable();
        p.dedup();
        n.sort_unstable();
        n.dedup();
        match self.need(Goal::Empty(p, n), f)? {
            Answer::Empty(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    fn force(&self, id: Id, f: &mut Frame) -> Result<Option<Bdd>, Pause> {
        match self.need(Goal::Force(id), f)? {
            Answer::Diagram(v) => Ok(v),
            _ => unreachable!(),
        }
    }
    fn bounds(&self, v: V, upper: bool) -> Vec<Id> {
        let state = &self.solver.vars[v.0 as usize];
        let ts = if upper { &state.upper } else { &state.lower };
        let links = if upper { &state.into } else { &state.from };
        ts.iter()
            .map(|t| self.pool.import(t))
            .chain(links.iter().map(|v| self.pool.intern(Node::Var(*v))))
            .collect()
    }
    fn union(&self, ids: Vec<Id>) -> Id {
        self.pool.import(
            &self.pool.union(
                &ids.into_iter()
                    .map(|id| self.pool.ty(id))
                    .collect::<Vec<_>>(),
            ),
        )
    }
    fn inter(&self, ids: Vec<Id>) -> Id {
        self.pool.import(
            &self.pool.inter(
                &ids.into_iter()
                    .map(|id| self.pool.ty(id))
                    .collect::<Vec<_>>(),
            ),
        )
    }
    fn product(&mut self, p: Product) -> Result<u32, Pause> {
        if let Some(id) = self.product_ids.get(&p) {
            self.demand.charge_state(&[6, self.scope, u64::from(*id)])?;
            return Ok(*id);
        }
        let id = self.products.len() as u32;
        self.demand.charge_state(&[6, self.scope, u64::from(id)])?;
        self.product_ids.insert(p.clone(), id);
        self.products.push(p);
        Ok(id)
    }
    fn meet_products(&mut self, a: u32, b: u32) -> Result<u32, Pause> {
        let mut p = self.products[a as usize].clone();
        for (l, c) in &self.products[b as usize] {
            match p.binary_search_by(|(key, _)| key.cmp(l)) {
                Ok(i) => p[i].1 = p[i].1.meet(c),
                Err(i) => p.insert(i, (l.clone(), c.clone())),
            }
        }
        self.product(p)
    }
    fn apply(&mut self, op: Op, a: Bdd, b: Bdd) -> Result<Bdd, Pause> {
        Ok(self.kernel.apply(op, a, b, self.demand)?)
    }

    fn compute(&mut self, g: &Goal, f: &mut Frame) -> Step {
        match g {
            Goal::Rel(a, b, env) => self.relation(*a, *b, *env, f).map(Answer::Rel),
            Goal::Enclose(id, upper) => self.enclose(*id, *upper, f).map(Answer::Type),
            Goal::Empty(p, n) => self.set_empty(p, n, f).map(Answer::Empty),
            Goal::Force(id) => self.force_node(*id, f).map(Answer::Diagram),
            Goal::Walk(bdd, p, n, projection) => self.walk(*bdd, *p, n, projection, f),
            Goal::Line(p, n, projection) => self.line(*p, n, projection, f),
        }
    }

    /// Whole-row bounds give sufficient enclosures for a proof. Field types
    /// stay shared and retain the ordinary relation and its variable identity.
    fn enclose(&mut self, id: Id, upper: bool, f: &mut Frame) -> Result<Id, Pause> {
        let get = |e: &Self, id, f: &mut Frame| -> Result<Id, Pause> {
            match e.need(Goal::Enclose(id, upper), f)? {
                Answer::Type(id) => Ok(id),
                _ => unreachable!(),
            }
        };
        Ok(match self.node(id) {
            Node::Var(v) => {
                let bounds = self.bounds(v, upper);
                let bound = if upper {
                    self.inter(bounds)
                } else {
                    self.union(bounds)
                };
                get(self, bound, f)?
            }
            Node::Union(a, b) => self.union(vec![get(self, a, f)?, get(self, b, f)?]),
            Node::Inter(a, b) => self.inter(vec![get(self, a, f)?, get(self, b, f)?]),
            Node::Without(base, labels) => {
                let base = get(self, base, f)?;
                self.pool
                    .import(&self.pool.without(&self.pool.ty(base), labels))
            }
            Node::Diff(base, heads) => {
                let base = get(self, base, f)?;
                self.pool.intern(Node::Diff(base, heads))
            }
            _ => id,
        })
    }

    fn relation(&mut self, a: Id, b: Id, env: Mode, f: &mut Frame) -> Result<bool, Pause> {
        if a == b || a == Id::NEVER || b == Id::TOP {
            return Ok(true);
        }
        let (x, y) = (self.node(a), self.node(b));
        Ok(match (x, y) {
            (Node::Union(x, y), _) => self.rel(x, b, env, f)? && self.rel(y, b, env, f)?,
            (_, Node::Inter(x, y)) => self.rel(a, x, env, f)? && self.rel(a, y, env, f)?,
            (Node::Inter(x, y), _) => self.rel(x, b, env, f)? || self.rel(y, b, env, f)?,
            (_, Node::Union(x, y)) => self.rel(a, x, env, f)? || self.rel(a, y, env, f)?,
            (Node::Var(v), _) => {
                let bounds = self.bounds(v, true);
                self.rel(self.inter(bounds), b, env, f)?
            }
            (_, Node::Var(v)) => {
                let bounds = self.bounds(v, false);
                self.rel(a, self.union(bounds), env, f)?
            }
            (Node::Con(h, av, aa), Node::Con(k, bv, ba)) if h == k && aa.len() == ba.len() => {
                if (env == Mode::Env && av != bv) || (env != Mode::Env && !av.subset(&bv)) {
                    return Ok(false);
                }
                let variance = self.solver.variances(h, aa.len());
                let rows = self.solver.rows(h, aa.len());
                for (((a, b), v), row) in aa.into_iter().zip(ba).zip(variance).zip(rows) {
                    let (v, row) = if env == Mode::Value {
                        (v, row)
                    } else {
                        (Variance::Inv, Mode::Value)
                    };
                    if matches!(v, Variance::Co | Variance::Inv) && !self.rel(a, b, row, f)? {
                        return Ok(false);
                    }
                    if matches!(v, Variance::Contra | Variance::Inv) && !self.rel(b, a, row, f)? {
                        return Ok(false);
                    }
                }
                true
            }
            (Node::Fn(a), Node::Fn(b))
                if env == Mode::Value && a.params.len() == b.params.len() =>
            {
                for ((_, a), (_, b)) in a.params.into_iter().zip(b.params) {
                    if !self.rel(b, a, Mode::Value, f)? {
                        return Ok(false);
                    }
                }
                self.rel(a.ret, b.ret, Mode::Value, f)?
                    && self.rel(a.err, b.err, Mode::Row, f)?
                    && self.rel(a.env, b.env, Mode::Env, f)?
            }
            (Node::Rec(a), Node::Rec(b)) if env == Mode::Value => {
                for (l, t) in b {
                    let Ok(i) = a.binary_search_by(|(key, _)| key.cmp(&l)) else {
                        return Ok(false);
                    };
                    if !self.rel(a[i].1, t, Mode::Value, f)? {
                        return Ok(false);
                    }
                }
                true
            }
            (Node::Without(x, labels), Node::Rec(fields)) if env == Mode::Value => {
                if fields.is_empty() {
                    true
                } else if fields
                    .iter()
                    .any(|(l, _)| labels.iter().any(|k| k == l || k.may_equal(l)))
                {
                    false
                } else {
                    self.rel(x, b, Mode::Value, f)?
                }
            }
            (Node::Without(x, removed), Node::Without(y, target))
                if removed.iter().all(|l| target.contains(l)) =>
            {
                self.rel(x, y, env, f)?
            }
            (_, Node::Without(y, _)) if env == Mode::Value => {
                self.rel(a, y, Mode::Value, f)? && self.rel(a, Id::RECORD, Mode::Value, f)?
            }
            (Node::Both(x, _), _) => self.rel(x, b, env, f)?,
            (_, Node::Both(_, y)) => self.rel(a, y, env, f)?,
            _ => false,
        })
    }

    fn set_empty(&mut self, p: &[Id], n: &[Id], f: &mut Frame) -> Result<Empty, Pause> {
        if p.contains(&Id::NEVER) || n.contains(&Id::TOP) || p.iter().any(|t| n.contains(t)) {
            return Ok(Empty::Yes);
        }
        for &a in p {
            for &b in n {
                if self.rel(a, b, Mode::Value, f)? {
                    return Ok(Empty::Yes);
                }
            }
        }
        // Keep record Boolean structure in the lazy kernel. Expanding source
        // unions here would turn a conjunction of choices into eager DNF.
        if p.iter().any(|id| self.pool.record_domain(*id))
            && p.iter()
                .all(|id| *id == Id::TOP || self.pool.record_domain(*id))
        {
            let mut source = Bdd::ALL;
            for &id in p {
                let Some(b) = self.force(id, f)? else {
                    return Ok(Empty::Unknown);
                };
                source = self.apply(Op::Inter, source, b)?;
            }
            for &id in n {
                let Some(b) = self.force(id, f)? else {
                    return Ok(Empty::Unknown);
                };
                source = self.apply(Op::Difference, source, b)?;
            }
            let free = self.product(vec![])?;
            return match self.need(Goal::Walk(source, free, vec![], None), f)? {
                Answer::Empty(v) => Ok(v),
                _ => unreachable!(),
            };
        }
        // Expand only the Boolean node being demanded, never the source DAG.
        for (negative, ids) in [(false, p), (true, n)] {
            for (i, &id) in ids.iter().enumerate() {
                let node = self.node(id);
                let (parts, split) = match node {
                    Node::Union(a, b) => (vec![a, b], !negative),
                    Node::Inter(a, b) => (vec![a, b], negative),
                    Node::Top if !negative => (vec![], false),
                    Node::Never if negative => (vec![], false),
                    _ => continue,
                };
                let mut rest = ids.to_vec();
                rest.remove(i);
                if !split {
                    rest.extend(parts);
                    return if negative {
                        self.empty(p.to_vec(), rest, f)
                    } else {
                        self.empty(rest, n.to_vec(), f)
                    };
                }
                let mut result = Empty::Yes;
                for t in parts {
                    let mut part = rest.clone();
                    part.push(t);
                    let e = if negative {
                        self.empty(p.to_vec(), part, f)?
                    } else {
                        self.empty(part, n.to_vec(), f)?
                    };
                    if e == Empty::No {
                        return Ok(e);
                    }
                    if e == Empty::Unknown {
                        result = e;
                    }
                }
                return Ok(result);
            }
        }
        // Bounds are sufficient enclosures, not a guessed equality. Use both
        // directions separately so a missing lower bound cannot be a witness.
        for (negative, ids) in [(false, p), (true, n)] {
            for (i, &id) in ids.iter().enumerate() {
                let Node::Var(v) = self.node(id) else {
                    continue;
                };
                let upper = self.inter(self.bounds(v, true));
                let lower = self.union(self.bounds(v, false));
                let mut may = ids.to_vec();
                may[i] = if negative { lower } else { upper };
                let a = if negative {
                    self.empty(p.to_vec(), may, f)?
                } else {
                    self.empty(may, n.to_vec(), f)?
                };
                if a == Empty::Yes {
                    return Ok(Empty::Yes);
                }
                let mut must = ids.to_vec();
                must[i] = if negative { upper } else { lower };
                let b = if negative {
                    self.empty(p.to_vec(), must, f)?
                } else {
                    self.empty(must, n.to_vec(), f)?
                };
                return Ok(if b == Empty::No {
                    Empty::No
                } else {
                    Empty::Unknown
                });
            }
        }
        let mut head = None;
        for &id in p {
            let h = match self.node(id) {
                Node::Con(h, _, _) => h,
                Node::Fn(_) => Head::Fn,
                Node::Rec(_) | Node::Without(..) => Head::Record,
                _ => return Ok(Empty::Unknown),
            };
            if head.is_some_and(|old| old != h) {
                return Ok(Empty::Yes);
            }
            head = Some(h);
        }
        let Some(head) = head else {
            // A finite list of concrete heads cannot exhaust Top: a tuple
            // arity absent from that list supplies a witness.
            return Ok(
                if n.iter().all(|t| {
                    matches!(
                        self.node(*t),
                        Node::Con(..) | Node::Fn(_) | Node::Rec(_) | Node::Without(..)
                    )
                }) {
                    Empty::No
                } else {
                    Empty::Unknown
                },
            );
        };
        if head == Head::Record {
            let mut source = Bdd::ALL;
            for &id in p {
                let Some(b) = self.force(id, f)? else {
                    return Ok(Empty::Unknown);
                };
                source = self.apply(Op::Inter, source, b)?;
            }
            for &id in n {
                let Some(b) = self.force(id, f)? else {
                    return Ok(Empty::Unknown);
                };
                source = self.apply(Op::Difference, source, b)?;
            }
            let free = self.product(vec![])?;
            return match self.need(Goal::Walk(source, free, vec![], None), f)? {
                Answer::Empty(v) => Ok(v),
                _ => unreachable!(),
            };
        }
        if let Head::Named(TypeRef::Custom(id)) = head
            && self.solver.comparison_wrappers.contains(&id)
        {
            let mut positive = Vec::new();
            let mut negative = Vec::new();
            for &node in p {
                let Node::Con(_, _, args) = self.node(node) else {
                    unreachable!()
                };
                positive.push(args[0]);
            }
            for &node in n {
                match self.node(node) {
                    Node::Con(h, _, args) if h == head => negative.push(args[0]),
                    Node::Con(..) | Node::Fn(_) | Node::Rec(_) | Node::Without(..) => (),
                    _ => return Ok(Empty::Unknown),
                }
            }
            return match self.need(Goal::Empty(positive, negative), f)? {
                Answer::Empty(value) => Ok(value),
                _ => unreachable!(),
            };
        }
        let mut scalar = Some(ValueSet::ALL);
        let mut scalar_head = true;
        for &id in p {
            let Node::Con(_, values, args) = self.node(id) else {
                scalar_head = false;
                break;
            };
            if !args.is_empty() {
                scalar_head = false;
                break;
            }
            scalar = scalar.and_then(|v| v.meet(&values));
        }
        if scalar_head {
            for &id in n {
                match self.node(id) {
                    Node::Con(h, values, _) if h == head => {
                        scalar = scalar.and_then(|v| v.minus(&values))
                    }
                    Node::Con(..) | Node::Fn(_) | Node::Rec(_) | Node::Without(..) => (),
                    _ => return Ok(Empty::Unknown),
                }
            }
            return Ok(if scalar.is_none() {
                Empty::Yes
            } else {
                Empty::No
            });
        }
        if let Head::Tuple(arity) = head {
            // Tuples use the same first-difference product algorithm, with
            // positional labels and no absent coordinate.
            let mut fields = vec![];
            for i in 0..arity {
                let mut cell = Cell::required(Id::TOP);
                for &id in p {
                    let Node::Con(_, _, args) = self.node(id) else {
                        unreachable!()
                    };
                    cell.pos.push(args[i as usize]);
                }
                fields.push((Label::name(format!("{i:010}")), cell.normalized()));
            }
            let product = self.product(fields)?;
            let mut negs = vec![];
            for &id in n {
                match self.node(id) {
                    Node::Con(h, _, args) if h == head => {
                        let fields = args
                            .into_iter()
                            .enumerate()
                            .map(|(i, id)| (Label::name(format!("{i:010}")), Cell::required(id)))
                            .collect();
                        negs.push(self.product(fields)?);
                    }
                    Node::Con(..) | Node::Fn(_) | Node::Rec(_) | Node::Without(..) => (),
                    _ => return Ok(Empty::Unknown),
                }
            }
            return match self.need(Goal::Line(product, negs, None), f)? {
                Answer::Empty(v) => Ok(v),
                _ => unreachable!(),
            };
        }
        if head == Head::Fn {
            // As in Elixir fun_line_empty?'s positive-only case, a function
            // with no terminating result inhabits every positive arrow of
            // its arity. This is a demanded witness, not E4 normalization.
            let mut arity = None;
            for &id in p {
                let Node::Fn(fun) = self.node(id) else {
                    unreachable!()
                };
                if arity.is_some_and(|n| n != fun.params.len()) {
                    return Ok(Empty::Yes);
                }
                arity = Some(fun.params.len());
            }
            if n.iter().all(|id| match self.node(*id) {
                Node::Fn(fun) => Some(fun.params.len()) != arity,
                Node::Con(..) | Node::Rec(_) | Node::Without(..) => true,
                _ => false,
            }) {
                return Ok(Empty::No);
            }
        }
        // Function and nominal Boolean identities are deliberately outside
        // this kernel. Their ordinary structural inclusion was tried above.
        Ok(Empty::Unknown)
    }

    fn force_node(&mut self, id: Id, f: &mut Frame) -> Result<Option<Bdd>, Pause> {
        Ok(Some(match self.node(id) {
            Node::Never | Node::Con(..) | Node::Fn(_) => Bdd::EMPTY,
            Node::Top => Bdd::ALL,
            Node::Rec(fields) => {
                if fields.is_empty() {
                    return Ok(Some(Bdd::ALL));
                }
                if fields.iter().any(|(l, _)| matches!(l, Label::Param(_))) {
                    return Ok(None);
                }
                let product = self.product(
                    fields
                        .into_iter()
                        .map(|(l, t)| (l, Cell::required(t)))
                        .collect(),
                )?;
                self.kernel.atom(product)
            }
            Node::Union(a, b) | Node::Inter(a, b) => {
                let op = if matches!(self.node(id), Node::Union(..)) {
                    Op::Union
                } else {
                    Op::Inter
                };
                let Some(a) = self.force(a, f)? else {
                    return Ok(None);
                };
                let Some(b) = self.force(b, f)? else {
                    return Ok(None);
                };
                self.apply(op, a, b)?
            }
            Node::Without(base, labels) => {
                if labels.iter().any(|l| matches!(l, Label::Param(_))) {
                    return Ok(None);
                }
                let Some(base) = self.force(base, f)? else {
                    return Ok(None);
                };
                let free = self.product(vec![])?;
                match self.need(Goal::Walk(base, free, vec![], Some(labels)), f)? {
                    Answer::Diagram(v) => return Ok(v),
                    _ => unreachable!(),
                }
            }
            Node::Diff(base, heads) => {
                if heads.iter().any(|h| matches!(self.node(*h), Node::Rec(_))) {
                    Bdd::EMPTY
                } else {
                    let Some(base) = self.force(base, f)? else {
                        return Ok(None);
                    };
                    base
                }
            }
            Node::Both(a, b) if a == b => {
                let Some(a) = self.force(a, f)? else {
                    return Ok(None);
                };
                a
            }
            _ => return Ok(None),
        }))
    }

    fn walk(
        &mut self,
        bdd: Bdd,
        p: u32,
        n: &[u32],
        projection: &Option<Vec<Label>>,
        f: &mut Frame,
    ) -> Step {
        if bdd == Bdd::EMPTY {
            return Ok(if projection.is_some() {
                Answer::Diagram(Some(Bdd::EMPTY))
            } else {
                Answer::Empty(Empty::Yes)
            });
        }
        if bdd == Bdd::ALL {
            return self.need(Goal::Line(p, n.to_vec(), projection.clone()), f);
        }
        let d = self.kernel.decision(bdd).expect("decision");
        let mut negative = n.to_vec();
        negative.push(d.atom);
        let positive = self.meet_products(p, d.atom)?;
        let mut empty = Empty::Yes;
        let mut diagram = Bdd::EMPTY;
        let mut unknown = false;
        for (b, p, n) in [
            (d.no, p, negative),
            (d.yes, positive, n.to_vec()),
            (d.either, p, n.to_vec()),
        ] {
            match self.need(Goal::Walk(b, p, n, projection.clone()), f)? {
                Answer::Empty(Empty::No) => return Ok(Answer::Empty(Empty::No)),
                Answer::Empty(Empty::Unknown) => empty = Empty::Unknown,
                Answer::Empty(Empty::Yes) => (),
                Answer::Diagram(Some(b)) => diagram = self.apply(Op::Union, diagram, b)?,
                Answer::Diagram(None) => unknown = true,
                _ => unreachable!(),
            }
        }
        Ok(if projection.is_some() {
            Answer::Diagram((!unknown).then_some(diagram))
        } else {
            Answer::Empty(empty)
        })
    }

    fn line(&mut self, p: u32, n: &[u32], projection: &Option<Vec<Label>>, f: &mut Frame) -> Step {
        let mut unknown = false;
        for (label, cell) in self.products[p as usize].clone() {
            if cell.absent {
                continue;
            }
            match self.empty(cell.pos, cell.neg, f)? {
                Empty::Yes => {
                    return Ok(if projection.is_some() {
                        Answer::Diagram(Some(Bdd::EMPTY))
                    } else {
                        Answer::Empty(Empty::Yes)
                    });
                }
                Empty::Unknown => {
                    unknown |= projection
                        .as_ref()
                        .is_none_or(|labels| labels.contains(&label))
                }
                Empty::No => (),
            }
        }
        let Some((&first, rest)) = n.split_first() else {
            return if let Some(labels) = projection {
                if unknown {
                    Ok(Answer::Diagram(None))
                } else {
                    let mut fields = self.products[p as usize].clone();
                    fields.retain(|(l, _)| !labels.contains(l));
                    let id = self.product(fields)?;
                    Ok(Answer::Diagram(Some(
                        if self.products[id as usize].is_empty() {
                            Bdd::ALL
                        } else {
                            self.kernel.atom(id)
                        },
                    )))
                }
            } else {
                Ok(Answer::Empty(if unknown {
                    Empty::Unknown
                } else {
                    Empty::No
                }))
            };
        };
        let mut prefix = self.products[p as usize].clone();
        let excluded = self.products[first as usize].clone();
        let mut diagram = Bdd::EMPTY;
        let mut any_unknown = false;
        for (label, target) in excluded {
            let next_label = self.labels.len() as u64;
            let label_id = *self.labels.entry(label.clone()).or_insert(next_label);
            let source = match prefix.binary_search_by(|(key, _)| key.cmp(&label)) {
                Ok(i) => prefix[i].1.clone(),
                Err(_) => Cell::free(),
            };
            // Charge each split outcome before allocating its residual product.
            for index in 0..=target.pos.len() + target.neg.len() {
                self.demand.charge_state(&[
                    7,
                    self.scope,
                    u64::from(p),
                    u64::from(first),
                    label_id,
                    index as u64,
                ])?;
                let diff = source.difference(&target, index);
                let mut residual = prefix.clone();
                match residual.binary_search_by(|(key, _)| key.cmp(&label)) {
                    Ok(i) => residual[i].1 = diff,
                    Err(i) => residual.insert(i, (label.clone(), diff)),
                }
                let id = self.product(residual)?;
                match self.need(Goal::Line(id, rest.to_vec(), projection.clone()), f)? {
                    Answer::Empty(Empty::No) => return Ok(Answer::Empty(Empty::No)),
                    Answer::Empty(Empty::Unknown) => any_unknown = true,
                    Answer::Empty(Empty::Yes) => (),
                    Answer::Diagram(Some(b)) => diagram = self.apply(Op::Union, diagram, b)?,
                    Answer::Diagram(None) => any_unknown = true,
                    _ => unreachable!(),
                }
            }
            let meet = source.meet(&target);
            match prefix.binary_search_by(|(key, _)| key.cmp(&label)) {
                Ok(i) => prefix[i].1 = meet,
                Err(i) => prefix.insert(i, (label, meet)),
            }
        }
        Ok(if projection.is_some() {
            Answer::Diagram((!any_unknown).then_some(diagram))
        } else {
            Answer::Empty(if any_unknown {
                Empty::Unknown
            } else {
                Empty::Yes
            })
        })
    }
}
