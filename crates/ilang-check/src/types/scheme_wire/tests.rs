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
    let scheme = Scheme { level: 0, ty };
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
    };
    let encoded = encode_test(&solver, &scheme, &mut |_| unreachable!());
    let bundle: Bundle = serde_json::from_str(&encoded).unwrap();
    assert_eq!(bundle.variables.len(), vars.len());
    let mut decoded = Solver::new(HashMap::new(), Vec::new());
    decode(&encoded, &mut decoded, 0, &mut |_| None, &mut |_, _| None).unwrap();
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
    };
    let second = Scheme {
        level: 0,
        ty: Ty::tuple(vec![Ty::Var(shared)]),
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
        let root = Ty::Fn(Box::new(FnTy::new(
            vec![],
            Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]),
            Ty::Never,
            row,
        )));
        let scheme = Scheme { level: 0, ty: root };
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
        let root = Ty::Fn(Box::new(FnTy::new(
            vec![],
            Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]),
            Ty::Never,
            Ty::Var(row),
        )));
        let scheme = Scheme { level: 0, ty: root };
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
        let body = Ty::Fn(Box::new(FnTy::new(
            vec![],
            result.clone(),
            Ty::Never,
            Ty::Var(row),
        )));
        let root = Ty::Fn(Box::new(FnTy::new(
            vec![(Some("body".into()), body)],
            result,
            Ty::Never,
            Ty::Var(row),
        )));
        let scheme = Scheme { level: 0, ty: root };
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
