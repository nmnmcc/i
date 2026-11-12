use super::*;
use crate::hir::BuiltinType;
use crate::hir::TypeId;
use crate::types::FnTy;
use std::collections::HashMap;
fn encode_test(s: &Solver, scheme: &Scheme, names: &mut impl FnMut(TypeRef) -> String) -> String {
    let (vars, _, _) = scheme_roots(s, scheme);
    let ids = vars
        .into_iter()
        .filter(|v| s.vars[v.0 as usize].level <= scheme.level)
        .enumerate()
        .map(|(i, v)| (v, i))
        .collect();
    encode(s, scheme, names, &ids)
}

#[test]
fn nominal_identity_variance_and_row_modes_survive_transfer() {
    let id = TypeId(7);
    let mut solver = Solver::new(HashMap::new(), Vec::new());
    solver.variance.insert(id, vec![Variance::Inv]);
    solver.row_params.insert(id, vec![Mode::Env]);
    let ty = Ty::named(
        TypeRef::Custom(id),
        vec![Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![])],
    );
    let scheme = Scheme {
        level: 0,
        ty,
        key_slots: vec![],
    };
    let text = encode_test(&solver, &scheme, &mut |t| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => "pkg:api.Box".into(),
    });
    let mut destination = Solver::new(HashMap::new(), Vec::new());
    decode(
        &text,
        &mut destination,
        0,
        &mut |name| match name {
            "Int" => Some(TypeRef::Builtin(BuiltinType::Int)),
            "pkg:api.Box" => Some(TypeRef::Custom(TypeId(99))),
            _ => None,
        },
        &mut |_| unreachable!(),
        &mut |_, _| unreachable!(),
    )
    .unwrap();
    assert_eq!(destination.variance[&TypeId(99)], [Variance::Inv]);
    assert_eq!(destination.row_params[&TypeId(99)], [Mode::Env]);
}

#[test]
fn decoded_hidden_handler_obligations_reject_different_service_forms() {
    let service = TypeId(17);
    let mut solver = Solver::new(HashMap::new(), Vec::new());
    solver.variance.insert(service, vec![Variance::Co]);
    let row = solver.fresh_at(1);
    let state = &mut solver.vars[row.0 as usize];
    state.row = Mode::Env;
    state.difference = true;
    state.handled.push(Ty::named(
        TypeRef::Custom(service),
        vec![Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![])],
    ));
    solver.has_handlers = true;
    let scheme = Scheme {
        level: 0,
        ty: Ty::Var(row),
        key_slots: vec![],
    };
    let text = encode_test(&solver, &scheme, &mut |t| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => "pkg:Svc".into(),
    });
    let mut restored = Solver::new(HashMap::new(), Vec::new());
    let decoded = decode(
        &text,
        &mut restored,
        0,
        &mut |name| match name {
            "Int" => Some(TypeRef::Builtin(BuiltinType::Int)),
            "pkg:Svc" => Some(TypeRef::Custom(TypeId(99))),
            _ => None,
        },
        &mut |_| unreachable!(),
        &mut |_, _| None,
    )
    .unwrap();
    let good = Ty::named(
        TypeRef::Custom(TypeId(99)),
        vec![Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![])],
    );
    let bad = Ty::named(
        TypeRef::Custom(TypeId(99)),
        vec![Ty::named(TypeRef::Builtin(BuiltinType::String), vec![])],
    );
    let mut accepted = restored.clone();
    accepted.constrain_env(&good, &decoded.ty);
    assert!(accepted.failures.is_empty());
    restored.constrain_env(&bad, &decoded.ty);
    assert!(!restored.failures.is_empty());
}

#[test]
fn scheme_bounds_follow_long_shared_variable_chains() {
    let mut solver = Solver::new(HashMap::new(), Vec::new());
    let vars: Vec<_> = (0..1000).map(|_| solver.fresh_at(1)).collect();
    for pair in vars.windows(2) {
        solver.vars[pair[0].0 as usize].from.push(pair[1]);
        solver.vars[pair[1].0 as usize].into.push(pair[0]);
    }
    let scheme = Scheme {
        level: 0,
        ty: Ty::Var(vars[0]),
        key_slots: vec![],
    };
    let encoded = encode_test(&solver, &scheme, &mut |_| unreachable!());
    let bundle: Bundle = serde_json::from_str(&encoded).unwrap();
    assert_eq!(bundle.variables.len(), vars.len());
    let mut decoded = Solver::new(HashMap::new(), Vec::new());
    decode(
        &encoded,
        &mut decoded,
        0,
        &mut |_| None,
        &mut |_| unreachable!(),
        &mut |_, _| None,
    )
    .unwrap();
    assert_eq!(decoded.vars.len(), vars.len());
    assert_eq!(decoded.vars[0].from, vec![V(1)]);
    assert_eq!(decoded.vars[999].into, vec![V(998)]);
}

#[test]
fn exported_schemes_keep_one_shared_nongeneralized_variable() {
    let mut solver = Solver::new(HashMap::new(), Vec::new());
    let shared = solver.fresh_at(0);
    solver.vars[shared.0 as usize].input = true;
    solver.vars[shared.0 as usize]
        .upper
        .push(Ty::named(TypeRef::Builtin(BuiltinType::String), vec![]));
    let first = Scheme {
        level: 0,
        ty: Ty::Var(shared),
        key_slots: vec![],
    };
    let second = Scheme {
        level: 0,
        ty: Ty::tuple(vec![Ty::Var(shared)]),
        key_slots: vec![],
    };
    let catalog = Arc::new(SchemeCatalog {
        solver: Mutex::new(solver),
        schemes: vec![first.clone(), second.clone()],
        externals: OnceLock::new(),
    });
    let a = SchemeArtifact {
        catalog: catalog.clone(),
        scheme: first,
    }
    .text_with(&mut |t| match t {
        TypeRef::Builtin(b) => b.name().into(),
        _ => unreachable!(),
    });
    let b = SchemeArtifact {
        catalog,
        scheme: second,
    }
    .text_with(&mut |t| match t {
        TypeRef::Builtin(b) => b.name().into(),
        _ => unreachable!(),
    });
    let mut env = SchemeEnvironment::default();
    let resolver =
        &mut |name: &str| (name == "String").then_some(TypeRef::Builtin(BuiltinType::String));
    let one = env.read(&a, resolver).unwrap();
    let two = env.read(&b, resolver).unwrap();
    assert_eq!(env.externals.len(), 1);
    assert_eq!(
        scheme_roots(&one.catalog.solver.lock().unwrap(), &one.scheme).0,
        [V(0)]
    );
    assert_eq!(
        scheme_roots(&two.catalog.solver.lock().unwrap(), &two.scheme).0,
        [V(0)]
    );
    let mut malformed: serde_json::Value = serde_json::from_str(&b).unwrap();
    malformed["variables"][0]["input"] = serde_json::Value::Bool(false);
    let previous = env.solver.vars.clone();
    assert!(env.read(&malformed.to_string(), resolver).is_err());
    assert_eq!(env.solver.vars, previous);
}

#[test]
fn scheme_comparison_proves_alpha_renamed_record_labels_without_mutation() {
    let build = |id: u32, name: &str| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let key = TyVarId(id);
        let var = solver.fresh_at(1);
        solver.vars[var.0 as usize].label_key = Some(key);
        solver.vars[var.0 as usize].name = Some(name.into());
        let record = solver
            .records
            .record(vec![(records::variable_label(key, name, 1), Ty::Var(var))]);
        let ty = Ty::Fn(Box::new(FnTy {
            params: vec![(None, record), (None, Ty::Var(var))],
            ret: Ty::Var(var),
            err: Ty::Never,
            env: Ty::Never,
        }));
        let scheme = Scheme {
            level: 0,
            ty,
            key_slots: vec![(key, 1)],
        };
        let before = solver.vars.clone();
        (
            SchemeArtifact {
                catalog: Arc::new(SchemeCatalog {
                    solver: Mutex::new(solver),
                    schemes: vec![scheme.clone()],
                    externals: OnceLock::new(),
                }),
                scheme,
            },
            before,
        )
    };
    let (a, before_a) = build(7, "K");
    let (b, before_b) = build(91, "Key");
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        a.compare(&b, &mut names, &mut other_names).unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
    assert_eq!(a.catalog.solver.lock().unwrap().vars, before_a);
    assert_eq!(b.catalog.solver.lock().unwrap().vars, before_b);
}

#[test]
fn scheme_comparison_proves_redundant_bounds() {
    let build = |redundant: bool| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let v = solver.fresh_at(1);
        let integer = Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]);
        let string = Ty::named(TypeRef::Builtin(BuiltinType::String), vec![]);
        solver.vars[v.0 as usize].upper.push(integer.clone());
        if redundant {
            solver.vars[v.0 as usize]
                .upper
                .push(Ty::Union(vec![integer, string]));
        }
        let scheme = Scheme {
            level: 0,
            ty: Ty::Var(v),
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(false)
            .compare(&build(true), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn scheme_comparison_projects_invisible_intermediate_variables() {
    let build = |intermediate: bool| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let a = solver.fresh_at(1);
        let integer = Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]);
        if intermediate {
            let b = solver.fresh_at(1);
            solver.constrain(&Ty::Var(a), &Ty::Var(b));
            solver.constrain(&Ty::Var(b), &integer);
        } else {
            solver.constrain(&Ty::Var(a), &integer);
        }
        assert!(solver.failures.is_empty());
        let scheme = Scheme {
            level: 0,
            ty: Ty::Var(a),
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(false)
            .compare(&build(true), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn scheme_comparison_collapses_mutually_constrained_visible_variables() {
    let build = |extra: bool| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let a = solver.fresh_at(1);
        let b = if extra { solver.fresh_at(1) } else { a };
        if extra {
            solver.constrain(&Ty::Var(a), &Ty::Var(b));
            solver.constrain(&Ty::Var(b), &Ty::Var(a));
        }
        assert!(solver.failures.is_empty());
        let root = solver.records.record(vec![
            (Label::name("left"), Ty::Var(a)),
            (Label::name("right"), Ty::Var(b)),
        ]);
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(false)
            .compare(&build(true), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn scheme_comparison_substitutes_exact_structural_variables() {
    let build = |extra: bool| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let a = solver.fresh_at(1);
        let list = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![Ty::Var(a)]);
        let value = if extra {
            let b = solver.fresh_at(1);
            solver.constrain(&list, &Ty::Var(b));
            solver.constrain(&Ty::Var(b), &list);
            Ty::Var(b)
        } else {
            list
        };
        assert!(solver.failures.is_empty());
        let root = solver.records.record(vec![
            (Label::name("value"), value),
            (Label::name("item"), Ty::Var(a)),
        ]);
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(false)
            .compare(&build(true), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn scheme_comparison_uses_explicit_nonbijective_union_instantiations() {
    let build = |two: bool| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let a = solver.fresh_at(1);
        let value = if two {
            let b = solver.fresh_at(1);
            Ty::Union(vec![Ty::Var(a), Ty::Var(b)])
        } else {
            Ty::Var(a)
        };
        let root = solver.records.record(vec![(Label::name("value"), value)]);
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(false)
            .compare(&build(true), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn scheme_coverage_preserves_keys_while_instantiating_union_variables() {
    let build = |two: bool, id: u32, name: &str| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let key = TyVarId(id);
        let k = solver.fresh_at(1);
        solver.vars[k.0 as usize].label_key = Some(key);
        solver.vars[k.0 as usize].name = Some(name.into());
        let a = solver.fresh_at(1);
        let value = if two {
            let b = solver.fresh_at(1);
            Ty::Union(vec![Ty::Var(a), Ty::Var(b)])
        } else {
            Ty::Var(a)
        };
        let root = solver
            .records
            .record(vec![(records::variable_label(key, name, 1), value)]);
        let root = Ty::Fn(Box::new(FnTy {
            params: vec![(None, root), (None, Ty::Var(k))],
            ret: Ty::Never,
            err: Ty::Never,
            env: Ty::Never,
        }));
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![(key, 1)],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(false, 7, "K")
            .compare(&build(true, 12, "Key"), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn scheme_coverage_preserves_handlers_through_union_row_instantiation() {
    let build = |two: bool| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let service = TypeId(5);
        solver.variance.insert(service, vec![Variance::Co]);
        let handler = Ty::named(
            TypeRef::Custom(service),
            vec![Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![])],
        );
        let a = solver.fresh_at(1);
        solver.vars[a.0 as usize].row = Mode::Env;
        solver.vars[a.0 as usize].difference = true;
        solver.vars[a.0 as usize].handled.push(handler.clone());
        let row = if two {
            let b = solver.fresh_at(1);
            solver.vars[b.0 as usize].row = Mode::Env;
            solver.vars[b.0 as usize].difference = true;
            solver.vars[b.0 as usize].handled.push(handler);
            Ty::Union(vec![Ty::Var(a), Ty::Var(b)])
        } else {
            Ty::Var(a)
        };
        solver.has_handlers = true;
        let root = Ty::Fn(Box::new(FnTy {
            params: vec![],
            ret: Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]),
            err: Ty::Never,
            env: row,
        }));
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => "pkg:Svc".into(),
    };
    let mut other_names = names;
    assert_eq!(
        build(false)
            .compare(&build(true), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn scheme_coverage_finds_a_counterexample_in_a_union_row_handler() {
    let build = |second: BuiltinType| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let service = TypeId(5);
        solver.variance.insert(service, vec![Variance::Co]);
        let mut rows = Vec::new();
        for parameter in [BuiltinType::Int, second] {
            let v = solver.fresh_at(1);
            solver.vars[v.0 as usize].row = Mode::Env;
            solver.vars[v.0 as usize].difference = true;
            solver.vars[v.0 as usize].handled.push(Ty::named(
                TypeRef::Custom(service),
                vec![Ty::named(TypeRef::Builtin(parameter), vec![])],
            ));
            rows.push(Ty::Var(v));
        }
        solver.has_handlers = true;
        let row = Ty::Union(rows);
        let result = Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]);
        let body = Ty::Fn(Box::new(FnTy {
            params: vec![],
            ret: result.clone(),
            err: Ty::Never,
            env: row.clone(),
        }));
        let root = Ty::Fn(Box::new(FnTy {
            params: vec![(Some("body".into()), body)],
            ret: result,
            err: Ty::Never,
            env: row,
        }));
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => "pkg:Svc".into(),
    };
    let mut other_names = names;
    assert_eq!(
        build(BuiltinType::Int)
            .compare(&build(BuiltinType::String), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Different
    );
}

#[test]
fn output_only_environment_can_choose_an_empty_row() {
    let build = |parameter: BuiltinType| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let service = TypeId(5);
        solver.variance.insert(service, vec![Variance::Co]);
        let row = solver.fresh_at(1);
        solver.vars[row.0 as usize].row = Mode::Env;
        solver.vars[row.0 as usize].difference = true;
        solver.vars[row.0 as usize].handled.push(Ty::named(
            TypeRef::Custom(service),
            vec![Ty::named(TypeRef::Builtin(parameter), vec![])],
        ));
        solver.has_handlers = true;
        let root = Ty::Fn(Box::new(FnTy {
            params: vec![],
            ret: Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]),
            err: Ty::Never,
            env: Ty::Var(row),
        }));
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => "pkg:Svc".into(),
    };
    let mut other_names = names;
    assert_eq!(
        build(BuiltinType::Int)
            .compare(&build(BuiltinType::String), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn scheme_comparison_uses_explicit_nonbijective_intersection_instantiations() {
    let build = |two: bool| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let a = solver.fresh_at(1);
        let value = if two {
            let b = solver.fresh_at(1);
            Ty::Inter(vec![Ty::Var(a), Ty::Var(b)])
        } else {
            Ty::Var(a)
        };
        let root = solver.records.record(vec![(Label::name("value"), value)]);
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(false)
            .compare(&build(true), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn scheme_coverage_synthesizes_a_union_of_subject_binders() {
    let build = |two: bool| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let a = solver.fresh_at(1);
        let b = if two { solver.fresh_at(1) } else { a };
        let c = if two { solver.fresh_at(1) } else { a };
        let fun = Ty::Fn(Box::new(FnTy {
            params: vec![
                (Some("a".into()), Ty::Var(a)),
                (Some("b".into()), Ty::Var(b)),
                (Some("c".into()), Ty::Var(c)),
            ],
            ret: Ty::Top,
            err: Ty::Never,
            env: Ty::Never,
        }));
        let root = solver.records.record(vec![(Label::name("f"), fun)]);
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(false)
            .compare(&build(true), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn a_reverse_subtype_failure_is_not_a_coverage_counterexample() {
    let source = Solver::new(HashMap::new(), Vec::new());
    let mut target = Solver::new(HashMap::new(), Vec::new());
    let x = target.fresh_at(1);
    let int = Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]);
    let string = Ty::named(TypeRef::Builtin(BuiltinType::String), vec![]);
    let result = source.records.union(&[int.clone(), string.clone()]);
    let ground = Ty::Fn(Box::new(FnTy {
        params: vec![(Some("x".into()), int), (Some("y".into()), string)],
        ret: source.records.record(vec![(Label::name("value"), result)]),
        err: Ty::Never,
        env: Ty::Never,
    }));
    let generic = Ty::Fn(Box::new(FnTy {
        params: vec![
            (Some("x".into()), Ty::Var(x)),
            (Some("y".into()), Ty::Var(x)),
        ],
        ret: target
            .records
            .record(vec![(Label::name("value"), Ty::Var(x))]),
        err: Ty::Never,
        env: Ty::Never,
    }));
    let mut forward = target.scheme_snapshot();
    forward.constrain(&generic, &ground);
    assert!(forward.failures.is_empty());
    let mut backward = target.scheme_snapshot();
    backward.constrain(&ground, &generic);
    assert!(definite_failure(&backward));
    assert_eq!(
        shared_group_counterexample(&source, &ground, &[], &target, &generic, &[], &[]),
        Some(false)
    );
}

#[test]
fn scheme_coverage_combines_structured_subject_bounds() {
    let subject = Solver::new(HashMap::new(), Vec::new());
    let mut cover = Solver::new(HashMap::new(), Vec::new());
    let x = cover.fresh_at(1);
    let int = Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]);
    let string = Ty::named(TypeRef::Builtin(BuiltinType::String), vec![]);
    let either = subject.records.union(&[int.clone(), string.clone()]);
    let left = subject.records.record(vec![
        (Label::name("a"), int.clone()),
        (Label::name("b"), string.clone()),
    ]);
    let right = subject
        .records
        .record(vec![(Label::name("a"), string), (Label::name("b"), int)]);
    let out_a = subject
        .records
        .record(vec![(Label::name("a"), either.clone())]);
    let out_b = subject.records.record(vec![(Label::name("b"), either)]);
    let fun = |a, b, result| {
        Ty::Fn(Box::new(FnTy {
            params: vec![(Some("x".into()), a), (Some("y".into()), b)],
            ret: result,
            err: Ty::Never,
            env: Ty::Never,
        }))
    };
    let subject_root = fun(left, right, Ty::tuple(vec![out_a, out_b]));
    let cover_root = fun(
        Ty::Var(x),
        Ty::Var(x),
        Ty::tuple(vec![Ty::Var(x), Ty::Var(x)]),
    );
    let meta = Variable {
        depth: Some(1),
        external: None,
        name: None,
        label: None,
        mode: 0,
        difference: false,
        input: true,
        lower: 0,
        upper: 0,
        handled: 0,
        from: vec![],
        into: vec![],
    };
    let mut demand = records::Demand::new(records::Demand::LIMIT, true);
    assert_eq!(
        find_cover(
            CoverSide {
                solver: &cover,
                root: &cover_root,
                metadata: &[meta]
            },
            CoverSide {
                solver: &subject,
                root: &subject_root,
                metadata: &[]
            },
            &[],
            &[],
            100,
            &mut demand
        ),
        Ok(true)
    );
}

#[test]
fn scheme_comparison_aligns_variables_by_use_not_allocation_order() {
    let build = |reversed: bool| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let one = solver.fresh_at(1);
        let two = solver.fresh_at(1);
        let (a, b) = if reversed { (two, one) } else { (one, two) };
        let root = solver.records.record(vec![
            (Label::name("left"), Ty::Var(a)),
            (Label::name("right"), Ty::Var(b)),
        ]);
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(false)
            .compare(&build(true), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
}

#[test]
fn scheme_comparison_keeps_twenty_branch_graphs_shared() {
    let build = |reverse: bool| {
        let solver = Solver::new(HashMap::new(), Vec::new());
        let pool = &solver.records;
        let mut root = pool.record(Vec::new());
        for i in 0..20 {
            let a = pool.record(vec![(
                Label::name(format!("a{i}")),
                Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]),
            )]);
            let b = pool.record(vec![(
                Label::name(format!("b{i}")),
                Ty::named(TypeRef::Builtin(BuiltinType::String), vec![]),
            )]);
            let pair = if reverse { [b, a] } else { [a, b] };
            root = pool.union(&[
                pool.inter(&[root.clone(), pair[0].clone()]),
                pool.inter(&[root, pair[1].clone()]),
            ]);
        }
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let a = build(false);
    let b = build(true);
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    let start = std::time::Instant::now();
    assert_eq!(
        a.compare(&b, &mut names, &mut other_names).unwrap(),
        SchemeComparison::Equivalent(vec![vec![]])
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
}

#[test]
fn scheme_comparison_distinguishes_universal_from_monomorphic_function() {
    let build = |generic: bool| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let arg = if generic {
            Ty::Var(solver.fresh_at(1))
        } else {
            Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![])
        };
        let fun = Ty::Fn(Box::new(FnTy {
            params: vec![(None, arg.clone())],
            ret: arg,
            err: Ty::Never,
            env: Ty::Never,
        }));
        let root = solver.records.record(vec![(Label::name("f"), fun)]);
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(true)
            .compare(&build(false), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Different
    );
}

#[test]
fn scheme_comparison_detects_changed_label_key_positions() {
    let build = |slot| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let key = TyVarId(1);
        let v = solver.fresh_at(1);
        solver.vars[v.0 as usize].label_key = Some(key);
        let record = solver
            .records
            .record(vec![(records::variable_label(key, "K", 1), Ty::Var(v))]);
        let scheme = Scheme {
            level: 0,
            ty: record,
            key_slots: vec![(key, slot)],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => unreachable!(),
    };
    let mut other_names = names;
    assert_eq!(
        build(0)
            .compare(&build(1), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Different
    );
}

#[test]
fn scheme_comparison_detects_hidden_service_form_counterexample() {
    let build = |parameter: BuiltinType| {
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        let service = TypeId(3);
        solver.variance.insert(service, vec![Variance::Co]);
        let row = solver.fresh_at(1);
        solver.vars[row.0 as usize].row = Mode::Env;
        solver.vars[row.0 as usize].difference = true;
        solver.vars[row.0 as usize].handled.push(Ty::named(
            TypeRef::Custom(service),
            vec![Ty::named(TypeRef::Builtin(parameter), vec![])],
        ));
        solver.has_handlers = true;
        let result = Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]);
        let body = Ty::Fn(Box::new(FnTy {
            params: vec![],
            ret: result.clone(),
            err: Ty::Never,
            env: Ty::Var(row),
        }));
        let root = Ty::Fn(Box::new(FnTy {
            params: vec![(Some("body".into()), body)],
            ret: result,
            err: Ty::Never,
            env: Ty::Var(row),
        }));
        let scheme = Scheme {
            level: 0,
            ty: root,
            key_slots: vec![],
        };
        SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(solver),
                schemes: vec![scheme.clone()],
                externals: OnceLock::new(),
            }),
            scheme,
        }
    };
    let mut names = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().into(),
        TypeRef::Custom(_) => "pkg:Svc".into(),
    };
    let mut other_names = names;
    assert_eq!(
        build(BuiltinType::Int)
            .compare(&build(BuiltinType::String), &mut names, &mut other_names)
            .unwrap(),
        SchemeComparison::Different
    );
}

#[test]
fn bounds_rows_and_label_key_share_one_binder_after_round_trip() {
    let mut s = Solver::new(HashMap::new(), Vec::new());
    let var = s.fresh_at(1);
    let other = s.fresh_at(1);
    let key = TyVarId(7);
    let state = &mut s.vars[var.0 as usize];
    state.name = Some("K".into());
    state.label_key = Some(key);
    state.row = Mode::Env;
    state.difference = true;
    state.input = true;
    state
        .upper
        .push(Ty::named(TypeRef::Builtin(BuiltinType::String), vec![]));
    state
        .handled
        .push(Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]));
    state.from.push(other);
    s.vars[other.0 as usize].into.push(var);
    s.vars[other.0 as usize]
        .lower
        .push(Ty::named(TypeRef::Builtin(BuiltinType::String), vec![]));
    let ty = s
        .records
        .record(vec![(records::variable_label(key, "K", 1), Ty::Var(var))]);
    let scheme = Scheme {
        level: 0,
        ty,
        key_slots: vec![(key, 1)],
    };
    let mut type_name = |t: TypeRef| match t {
        TypeRef::Builtin(b) => b.name().to_string(),
        _ => unreachable!(),
    };
    let encoded = encode_test(&s, &scheme, &mut type_name);
    let mut out = Solver::new(HashMap::new(), Vec::new());
    let decoded = decode(
        &encoded,
        &mut out,
        5,
        &mut |name| match name {
            "String" => Some(TypeRef::Builtin(BuiltinType::String)),
            "Int" => Some(TypeRef::Builtin(BuiltinType::Int)),
            _ => None,
        },
        &mut |_| TyVarId(100),
        &mut |_, _| None,
    )
    .unwrap();
    assert_eq!(decoded.key_slots, [(TyVarId(100), 1)]);
    let labels = records::label_params(&decoded.ty);
    let label = labels.iter().next().unwrap();
    assert_eq!(records::label_var(label), Some(TyVarId(100)));
    assert_eq!(label.level, 6);
    let state = &out.vars[0];
    assert_eq!(state.label_key, Some(TyVarId(100)));
    assert_eq!(state.row, Mode::Env);
    assert!(state.difference && state.input);
    assert_eq!(state.upper.len(), 1);
    assert_eq!(state.handled.len(), 1);
    assert!(out.has_handlers);
    assert_eq!(out.vars[0].from, vec![V(1)]);
    assert_eq!(out.vars[1].into, vec![V(0)]);
    assert_eq!(out.vars[1].lower.len(), 1);
    // U6 permits different node numbering for the same graph.
    let again = encode_test(&out, &decoded, &mut type_name);
    let mut original: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    let mut rebuilt: serde_json::Value = serde_json::from_str(&again).unwrap();
    original.as_object_mut().unwrap().remove("graph");
    rebuilt.as_object_mut().unwrap().remove("graph");
    assert_eq!(original, rebuilt);
}

#[test]
fn decoded_label_scheme_can_be_instantiated_and_called() {
    let mut original = Solver::new(HashMap::new(), Vec::new());
    let key = TyVarId(3);
    let key_type = original.fresh_at(1);
    original.vars[key_type.0 as usize].label_key = Some(key);
    original.vars[key_type.0 as usize].name = Some("K".into());
    original.vars[key_type.0 as usize]
        .upper
        .push(Ty::named(TypeRef::Builtin(BuiltinType::String), vec![]));
    let int = Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]);
    let record = original
        .records
        .record(vec![(records::variable_label(key, "K", 1), int.clone())]);
    let ty = Ty::Fn(Box::new(FnTy {
        params: vec![(None, record), (None, Ty::Var(key_type))],
        ret: int.clone(),
        err: Ty::Never,
        env: Ty::Never,
    }));
    let scheme = Scheme {
        level: 0,
        ty,
        key_slots: vec![(key, 1)],
    };
    let encoded = encode_test(&original, &scheme, &mut |t| match t {
        TypeRef::Builtin(b) => b.name().into(),
        _ => unreachable!(),
    });
    let mut restored = Solver::new(HashMap::new(), Vec::new());
    let scheme = decode(
        &encoded,
        &mut restored,
        0,
        &mut |name| match name {
            "Int" => Some(TypeRef::Builtin(BuiltinType::Int)),
            "String" => Some(TypeRef::Builtin(BuiltinType::String)),
            _ => None,
        },
        &mut |_| TyVarId(20),
        &mut |_, _| None,
    )
    .unwrap();
    let instantiated = restored.freshen(&scheme.ty, scheme.level, &mut HashMap::new());
    let label = records::label_params(&instantiated)
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(records::label_var(&label), Some(scheme.key_slots[0].0));
    let actual = restored.records.record(vec![(Label::name("a"), int)]);
    let labels = [(label, Label::name("a"))].into_iter().collect();
    let callable = records::map_labels(&instantiated, &labels);
    let Ty::Graph(graph) = callable else {
        panic!("decoded callable graph");
    };
    let Some(Ty::Fn(function)) = graph.view() else {
        panic!("decoded callable");
    };
    restored.constrain(&actual, &function.params[0].1);
    assert!(restored.failures.is_empty());
    let wrong = restored.records.record(vec![(
        Label::name("b"),
        Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]),
    )]);
    restored.constrain(&wrong, &function.params[0].1);
    assert!(!restored.failures.is_empty());
}
