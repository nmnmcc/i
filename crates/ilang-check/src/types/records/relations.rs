//! Rule-level probes through the production constraint entry point.

use super::{Label, LabelParam};
use crate::{
    hir::{BindingRef, BuiltinType, ModuleId, TypeId, TypeRef},
    types::{
        Ty, V,
        solve::{FailKind, Solver},
    },
};
use std::collections::HashMap;

fn solver() -> Solver {
    Solver::new(HashMap::new(), Vec::new())
}
fn int() -> Ty {
    Ty::named(TypeRef::Builtin(BuiltinType::Int), Vec::new())
}
fn string() -> Ty {
    Ty::named(TypeRef::Builtin(BuiltinType::String), Vec::new())
}
fn key() -> Label {
    Label::Param(LabelParam {
        owner: BindingRef {
            module: ModuleId(0),
            index: 0,
        }.into(),
        index: 0,
        level: 1,
        name: "K".into(),
    })
}

#[test]
fn width_depth_and_a_wrong_field_use_production_constraints() {
    let mut s = solver();
    let child = s.record(vec![
        (Label::name("a"), int()),
        (Label::name("b"), string()),
    ]);
    let src = s.record(vec![(Label::name("row"), child)]);
    let wanted = s.record(vec![(Label::name("a"), int())]);
    let dst = s.record(vec![(Label::name("row"), wanted)]);
    s.constrain(&src, &dst);
    assert!(s.failures.is_empty());
    let bad_child = s.record(vec![(Label::name("a"), string())]);
    let bad = s.record(vec![(Label::name("row"), bad_child)]);
    s.constrain(&src, &bad);
    assert!(!s.failures.is_empty());
}

#[test]
fn split_rec_constraints_and_intersection_members_do_not_make_a_cycle() {
    let mut s = solver();
    let r = s.fresh();
    let a = s.record(vec![(Label::name("a"), int())]);
    let src = s.inter(vec![r.clone(), a]);
    let dst = s.record(vec![
        (Label::name("a"), int()),
        (Label::name("b"), string()),
    ]);
    s.constrain(&src, &dst);
    assert!(s.failures.is_empty());
    s.constrain(&src, &r);
    assert!(s.failures.is_empty());
    let Ty::Var(v) = r else { unreachable!() };
    assert!(s.vars[v.0 as usize].lower.is_empty());
}

#[test]
fn forgetting_fails_at_the_read_even_with_an_unconstrained_base() {
    let mut s = solver();
    let base = s.fresh();
    let lost = s.without(&base, vec![Label::name("a")]);
    let wanted = s.record(vec![(Label::name("a"), int())]);
    s.constrain(&lost, &wanted);
    assert_eq!(s.failures[0].kind, FailKind::NoField);
    let Ty::Var(v) = base else { unreachable!() };
    assert!(s.vars[v.0 as usize].upper.is_empty());

    s.failures.clear();
    let uncertain = s.without(&base, vec![key()]);
    let new = s.record(vec![(key(), string())]);
    let updated = s.inter(vec![uncertain, new]);
    s.constrain(&updated, &wanted);
    assert_eq!(s.failures[0].kind, FailKind::LabelMayBe);
    assert!(s.vars[v.0 as usize].upper.is_empty());
}

#[test]
fn update_overrides_the_old_type_and_empty_record_still_restricts_the_head() {
    let mut s = solver();
    let old = s.record(vec![(Label::name("a"), int())]);
    let forgotten = s.without(&old, vec![Label::name("a")]);
    let new = s.record(vec![(Label::name("a"), string())]);
    let updated = s.inter(vec![forgotten, new.clone()]);
    s.constrain(&updated, &new);
    assert!(s.failures.is_empty());
    let all = s.record(Vec::new());
    let r = s.fresh();
    let input = s.inter(vec![r.clone(), all]);
    assert!(matches!(&input, Ty::Graph(g) if g.record_only()));
    s.constrain(&int(), &input);
    assert!(!s.failures.is_empty());
}

#[test]
fn record_lower_bounds_do_not_join_invariant_field_arguments() {
    let mut s = solver();
    let a = Ty::named(TypeRef::Custom(TypeId(0)), vec![int()]);
    let b = Ty::named(TypeRef::Custom(TypeId(0)), vec![string()]);
    let a = s.record(vec![(Label::name("cell"), a)]);
    let b = s.record(vec![(Label::name("cell"), b)]);
    let target = s.fresh();
    s.constrain(&a, &target);
    s.constrain(&b, &target);
    assert!(s.failures.is_empty());
    let Ty::Var(v) = target else { unreachable!() };
    assert_eq!(s.vars[v.0 as usize].lower.len(), 2);
}

#[test]
fn rigid_label_cannot_reach_an_outer_variable() {
    let mut s = solver();
    let outer = s.fresh();
    let value = s.record(vec![(key(), int())]);
    s.constrain(&value, &outer);
    assert_eq!(s.failures[0].kind, FailKind::LabelEscapes);
    let Ty::Var(V(index)) = outer else {
        unreachable!()
    };
    assert!(s.vars[index as usize].lower.is_empty());
}

#[test]
fn common_field_metadata_never_replaces_an_exact_record_union() {
    let mut s = solver();
    let left = s.record(vec![
        (Label::name("a"), int()),
        (Label::name("left"), int()),
    ]);
    let right = s.record(vec![
        (Label::name("a"), string()),
        (Label::name("right"), int()),
    ]);
    let union = s.union(vec![left.clone(), right]);
    let Ty::Graph(g) = &union else {
        panic!("exact record union")
    };
    let summary = g.known_summary().unwrap();
    let Ty::Graph(summary) = &summary else {
        panic!("record summary")
    };
    assert_eq!(summary.fields().unwrap().len(), 1);
    assert_ne!(&union, &Ty::Graph(summary.clone()));
    let mut demand = super::Demand::new(super::Demand::LIMIT, true);
    assert_eq!(
        super::prove(&s, &union, &left, &mut demand),
        super::Verdict::Refuted
    );
    let known = crate::types::known::members(&union);
    assert_eq!(known, vec![Ty::Graph(summary.clone())]);
}

#[test]
fn production_union_obligations_cannot_choose_bounds_through_an_intersection() {
    let mut s = solver();
    let input = s.fresh();
    let forgotten = s.without(&input, vec![Label::name("g")]);
    let field = s.record(vec![(Label::name("g"), int())]);
    let changed = s.inter(vec![forgotten, field]);
    let a = s.record(vec![(Label::name("a"), int()), (Label::name("g"), int())]);
    let b = s.record(vec![
        (Label::name("b"), string()),
        (Label::name("g"), int()),
    ]);
    let target = s.union(vec![a, b]);
    let before = s.vars.clone();
    s.constrain(&changed, &target);
    assert_eq!(s.vars, before);
    assert_eq!(s.failures[0].kind, FailKind::UnionProof);
}

#[test]
fn a_whole_target_variable_keeps_the_exact_record_union_bound() {
    let mut s = solver();
    let a = s.record(vec![(Label::name("a"), int())]);
    let b = s.record(vec![(Label::name("b"), string())]);
    let source = s.union(vec![a, b]);
    let output = s.fresh();
    s.constrain(&source, &output);
    let Ty::Var(v) = output else { unreachable!() };
    assert!(s.failures.is_empty());
    assert_eq!(s.vars[v.0 as usize].lower, vec![source]);
}

#[test]
fn production_branch_and_fan_graphs_construct_check_and_display_under_one_second() {
    let start = std::time::Instant::now();
    let mut s = solver();
    let all = s.record(vec![]);
    let mut branch = all.clone();
    for n in 0..20 {
        let mut parts = Vec::new();
        for label in [format!("a{n}"), format!("b{n}")] {
            let label = Label::name(label);
            let base = s.without(&branch, vec![label.clone()]);
            let field = s.record(vec![(
                label,
                Ty::literal(&crate::types::LitValue::Int(1.into())),
            )]);
            parts.push(s.inter(vec![base, field]));
        }
        branch = s.union(parts);
    }
    s.constrain(&branch, &all);
    let Ty::Graph(g) = &branch else {
        unreachable!()
    };
    assert!(g.nodes().len() < 200);
    let shown = crate::types::display::show(&s, &branch);
    assert!(shown.len() < 30_000);
    let parameter = s.fresh();
    let mut fan = all.clone();
    for n in 0..20 {
        let a = s.record(vec![(Label::name(format!("a{n}")), parameter.clone())]);
        let b = s.record(vec![(Label::name(format!("b{n}")), int())]);
        let choice = s.union(vec![a, b]);
        fan = s.inter(vec![fan, choice]);
    }
    s.constrain(&fan, &all);
    let Ty::Graph(g) = &fan else { unreachable!() };
    assert!(g.nodes().len() < 200);
    let never = g.map_vars(&mut |_| Ty::Never);
    let Ty::Graph(never) = never else {
        unreachable!()
    };
    assert_eq!(never.fields().unwrap().len(), 20);
    assert!(s.failures.is_empty());
    assert!(start.elapsed().as_secs_f64() < 1.0, "{:?}", start.elapsed());
}
