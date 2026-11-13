//! Ordinary scheme instantiation at the target-plan boundary.
use super::{display, infer::Scheme, scheme_wire::SchemeCatalog, *};
use crate::hir;
use std::sync::{Arc, Mutex, OnceLock};

fn concrete(t: &ShownType) -> Result<Ty, String> {
    Ok(match t {
        ShownType::Named(r, xs) => {
            Ty::named(*r, xs.iter().map(concrete).collect::<Result<_, _>>()?)
        }
        ShownType::Never => Ty::Never,
        ShownType::Lit(s) => {
            let b = if s.starts_with('"') {
                hir::BuiltinType::String
            } else {
                hir::BuiltinType::Int
            };
            let value = if b == hir::BuiltinType::String {
                let lexed = ilang_syntax::lex(s);
                assert!(lexed.diagnostics.is_empty(), "checked literal must lex");
                let ilang_syntax::token::TokenKind::Str(value) = &lexed.tokens[0].kind else {
                    panic!("checked string literal must be a string token");
                };
                value.clone()
            } else {
                s.clone()
            };
            Ty::lit(
                b,
                lits::ValueSet::only([value.into()]).expect("one literal"),
            )
        }
        ShownType::Tuple(xs) => Ty::tuple(xs.iter().map(concrete).collect::<Result<_, _>>()?),
        ShownType::Record(keys, xs) => {
            let positions = keys.iter().filter(|k| k.is_none()).count() as u32;
            Ty::data(
                ShapeId::new(positions, keys.iter().flatten().cloned()),
                xs.iter().map(concrete).collect::<Result<_, _>>()?,
            )
        }
        ShownType::Union(xs) => Ty::Union(xs.iter().map(concrete).collect::<Result<_, _>>()?),
        ShownType::Inter(xs) => Ty::Inter(xs.iter().map(concrete).collect::<Result<_, _>>()?),
        ShownType::Diff(a, xs) => Ty::Diff(
            Box::new(concrete(a)?),
            xs.iter().map(concrete).collect::<Result<_, _>>()?,
        ),
        ShownType::Fn(f) if f.group => Ty::Fn(Box::new(FnTy {
            param: concrete(&f.params[0].1)?,
            labels: Vec::new(),
            ret: concrete(&f.ret)?,
            err: f
                .err
                .as_ref()
                .map(concrete)
                .transpose()?
                .unwrap_or(Ty::Never),
            env: f
                .env
                .as_ref()
                .map(concrete)
                .transpose()?
                .unwrap_or(Ty::Never),
        })),
        ShownType::Fn(f) => Ty::Fn(Box::new(FnTy {
            param: Ty::tuple(
                f.params
                    .iter()
                    .map(|(_, t)| concrete(t))
                    .collect::<Result<_, _>>()?,
            ),
            labels: f.params.iter().map(|(l, _)| l.clone()).collect(),
            ret: concrete(&f.ret)?,
            err: f
                .err
                .as_ref()
                .map(concrete)
                .transpose()?
                .unwrap_or(Ty::Never),
            env: f
                .env
                .as_ref()
                .map(concrete)
                .transpose()?
                .unwrap_or(Ty::Never),
        })),
        _ => return Err("provider type must be closed (no holes or free variables)".into()),
    })
}

impl Typed {
    /// Instantiate an ordinary checked factory scheme at one service instance.
    /// Uses the same retained constraints and subtype solver as source calls.
    pub(crate) fn provider_factory(
        &self,
        id: DefId,
        service: &ShownType,
        args: &[ShownType],
    ) -> Result<ShownType, String> {
        let artifact = self
            .provider_scheme_artifacts
            .get(&id)
            .ok_or("factory has no checked scheme")?;
        let mut s = artifact
            .catalog
            .solver
            .lock()
            .expect("provider scheme snapshot")
            .scheme_snapshot();
        s.level = 1;
        let ty = s.freshen(
            &artifact.scheme.ty,
            artifact.scheme.level,
            &mut HashMap::new(),
        );
        let ty = display::both(&ty);
        let Ty::Fn(f) = ty else {
            return Err("factory must be an ordinary function".into());
        };
        if f.arity() != Some(args.len()) {
            return Err("factory argument count does not match generated arguments".into());
        }
        let service = concrete(service)?;
        s.constrain(&f.ret, &service);
        // Fix generalized arguments that also determine captured services.
        // A closed narrower return (including Data(Never) from a stopping
        // factory) is already a valid subtype and must not be widened.
        if let (Ty::Con(head, _, actual), Ty::Con(wanted, _, args)) = (&f.ret, &service)
            && head == wanted
        {
            for (actual, wanted) in actual.iter().zip(args) {
                let mut vars = Vec::new();
                display::ty_vars(actual, &mut vars);
                if !vars.is_empty() {
                    s.constrain(wanted, actual);
                }
            }
        }
        if !s.failures.is_empty() {
            return Err("factory must return its service".into());
        }
        s.constrain(&f.err, &Ty::Never);
        if !s.failures.is_empty() {
            return Err("factory has uncaught typed errors".into());
        }
        for (param, arg) in f.parts().expect("checked arity").iter().zip(args) {
            s.constrain(&concrete(arg)?, param);
        }
        if !s.failures.is_empty() {
            return Err("factory argument types do not match generated arguments".into());
        }
        display::scheme(
            &s,
            &f.env,
            display::Opts {
                mono: true,
                written: false,
            },
        )
        .map_err(|()| "factory environment cannot be instantiated".into())
    }

    /// Check the entire generalized structured adapter type, independently of
    /// any particular entry result, error or environment.
    pub(crate) fn provider_adapter(
        &self,
        p: &Program,
        id: DefId,
        supplies: &[ShownType],
        requires: &[ShownType],
    ) -> Result<(), String> {
        let actual = self
            .provider_scheme_artifacts
            .get(&id)
            .ok_or("adapter has no checked scheme")?;
        let mut s = actual
            .catalog
            .solver
            .lock()
            .expect("adapter scheme snapshot")
            .scheme_snapshot();
        s.level = 1;
        let a = s.fresh();
        let e = s.fresh();
        let n = s.fresh();
        s.mark_mode(&e, solve::Mode::Row);
        s.mark_mode(&n, solve::Mode::Env);
        let supplies = supplies
            .iter()
            .map(concrete)
            .collect::<Result<Vec<_>, _>>()?;
        let requires = requires
            .iter()
            .map(concrete)
            .collect::<Result<Vec<_>, _>>()?;
        let required = s.env_union(requires.clone());
        let supplied_only = s.env_union(supplies.clone());
        let rest = s.env_diff(n.clone(), supplies.clone());
        let env = s.env_union(requires.into_iter().chain([rest]).collect());
        let supplied = s.env_union(supplies);
        let available = s.env_union(vec![env.clone(), supplied]);
        let body_env = s.inter(vec![n, available]);
        let body = Ty::Fn(Box::new(FnTy::new(vec![], a.clone(), e.clone(), body_env)));
        let scheme = Scheme {
            level: 0,
            ty: Ty::Fn(Box::new(FnTy::new(
                vec![(Some("body".into()), body)],
                a,
                e,
                env,
            ))),
        };
        let expected = SchemeArtifact {
            scheme: scheme.clone(),
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(s),
                schemes: vec![scheme],
                externals: OnceLock::new(),
            }),
        };
        // Difference removes a set of service constructors. Normalize both
        // schemes before comparison so manifest list order cannot affect the
        // generalized proof (the source scheme retains its written order).
        // For nullary services, N is a subset of (N - S) | S. Source
        // inference may retain that admission intersection or simplify it away.
        // Normalize the identity on both sides before comparing the complete
        // generalized schemes; parameterized services keep their constraints.
        fn view(t: &Ty) -> Ty {
            t.clone()
        }
        fn map_nodes(ty: &Ty, env: bool, f: &mut dyn FnMut(Ty, bool) -> Ty) -> Ty {
            let all = |xs: &[Ty], env: bool, f: &mut dyn FnMut(Ty, bool) -> Ty| -> Vec<Ty> {
                xs.iter().map(|x| map_nodes(x, env, f)).collect()
            };
            let shallow = match ty {
                Ty::Con(h, vs, args) => Ty::Con(*h, vs.clone(), all(args, false, f)),
                Ty::Fn(fun) => Ty::Fn(Box::new(FnTy {
                    param: map_nodes(&fun.param, false, f),
                    labels: fun.labels.clone(),
                    ret: map_nodes(&fun.ret, false, f),
                    err: map_nodes(&fun.err, false, f),
                    env: map_nodes(&fun.env, true, f),
                })),
                Ty::Union(xs) => Ty::Union(all(xs, env, f)),
                Ty::Inter(xs) => Ty::Inter(all(xs, env, f)),
                Ty::Diff(b, hs) => Ty::Diff(Box::new(map_nodes(b, env, f)), all(hs, false, f)),
                Ty::Both(a, b) => Ty::Both(
                    Box::new(map_nodes(a, env, f)),
                    Box::new(map_nodes(b, env, f)),
                ),
                t => t.clone(),
            };
            f(shallow, env)
        }
        fn parts(t: &Ty, intersection: bool) -> Vec<Ty> {
            let mut pending = vec![t.clone()];
            let mut result = Vec::new();
            while let Some(t) = pending.pop() {
                match view(&t) {
                    Ty::Inter(xs) if intersection => pending.extend(xs),
                    Ty::Union(xs) if !intersection => pending.extend(xs),
                    t => result.push(t),
                }
            }
            result
        }
        fn admission(t: Ty) -> Ty {
            let Ty::Inter(_) = t else {
                return t;
            };
            let xs = parts(&t, true);
            let keep: Vec<_> = xs.iter().filter(|candidate| {
                let union = parts(candidate, false);
                !xs.iter().any(|base| union.iter().any(|member| {
                    let Ty::Diff(from, removed) = member else { return false; };
                    view(from) == *base && removed.iter().all(|head| {
                        matches!(view(head), Ty::Con(Head::Named(TypeRef::Custom(_)), _, ref args) if args.is_empty())
                            && union.iter().any(|m| *m == view(head))
                    })
                }))
            }).cloned().collect();
            match keep.as_slice() {
                [one] => one.clone(),
                _ => Ty::Inter(keep),
            }
        }
        fn ordered_differences(p: &Program, ty: &Ty) -> Ty {
            map_nodes(ty, false, &mut |ty, env| match ty {
                Ty::Diff(base, mut removed) => {
                    removed.sort_by_cached_key(|ty| match ty.head() {
                        Some(Head::Named(TypeRef::Custom(id))) => {
                            format!("{}.{}", p.module(p.ty(id).module).path, p.ty(id).name)
                        }
                        head => format!("{head:?}"),
                    });
                    Ty::Diff(base, removed)
                }
                ty if env => admission(ty),
                ty => ty,
            })
        }
        let mut actual = actual.clone();
        actual.scheme.ty = ordered_differences(p, &actual.scheme.ty);
        let mut expected = expected;
        expected.scheme.ty = ordered_differences(p, &expected.scheme.ty);
        let name = |r: TypeRef| format!("{r:?}");
        match actual.compare(&expected, &mut |r| name(r), &mut |r| name(r))? {
            SchemeComparison::Equivalent(_) => Ok(()),
            other => {
                // The solver's own subtype check both ways, each side
                // instantiated fresh, and a supplied service given to the
                // body must not reach the adapter's own environment beyond
                // what it requires.
                let mut t = expected
                    .catalog
                    .solver
                    .lock()
                    .expect("adapter scheme snapshot")
                    .clone();
                t.level = 1;
                let a = t.freshen(
                    &actual.scheme.ty,
                    actual.scheme.level,
                    &mut Default::default(),
                );
                t.constrain(&a, &expected.scheme.ty);
                let e = t.freshen(&expected.scheme.ty, 0, &mut Default::default());
                t.constrain(&e, &actual.scheme.ty);
                let leak = t.freshen(
                    &actual.scheme.ty,
                    actual.scheme.level,
                    &mut Default::default(),
                );
                match &leak {
                    Ty::Fn(f) => match f.parts() {
                        Some([Ty::Fn(body)]) => {
                            t.constrain_env(&supplied_only, &body.env);
                            t.constrain_env(&f.env, &required);
                        }
                        _ => t.constrain(&leak, &Ty::Never),
                    },
                    _ => t.constrain(&leak, &Ty::Never),
                }
                if t.failures.is_empty() {
                    Ok(())
                } else {
                    Err(format!(
                        "adapter must have the generalized structured body/result/error/environment type ({other:?})"
                    ))
                }
            }
        }
    }
}

impl Typed {
    pub(crate) fn provider_factory_schema(
        &self,
        id: DefId,
        constructor: hir::TypeId,
        arity: usize,
        args: &[ilang_package::providers::Argument],
        p: &hir::Program,
    ) -> Result<(), String> {
        use ilang_package::providers::Argument;
        let artifact = self
            .provider_scheme_artifacts
            .get(&id)
            .ok_or("factory has no checked scheme")?;
        let mut s = artifact
            .catalog
            .solver
            .lock()
            .expect("provider schema snapshot")
            .scheme_snapshot();
        s.level = 1;
        let ty = s.freshen(
            &artifact.scheme.ty,
            artifact.scheme.level,
            &mut HashMap::new(),
        );
        let Ty::Fn(f) = display::both(&ty) else {
            return Err("factory must be an ordinary function".into());
        };
        if f.arity() != Some(args.len()) {
            return Err("factory argument count does not match generated arguments".into());
        }
        let params = (0..arity).map(|_| s.fresh_input()).collect();
        let service = Ty::named(TypeRef::Custom(constructor), params);
        s.constrain(&f.ret, &service);
        if !s.failures.is_empty() {
            return Err("factory must return its service".into());
        }
        s.constrain(&f.err, &Ty::Never);
        if !s.failures.is_empty() {
            return Err("factory has uncaught typed errors".into());
        }
        for (param, arg) in f.parts().expect("checked arity").iter().zip(args) {
            let actual = match arg {
                Argument::Seed => Ty::named(
                    TypeRef::Builtin(hir::BuiltinType::List),
                    vec![service.clone()],
                ),
                Argument::EntryRootKey => {
                    let nil = p
                        .prelude
                        .and_then(|m| p.module(m).types.iter().find(|id| p.ty(**id).name == "Nil"))
                        .expect("prelude Nil");
                    Ty::named(
                        TypeRef::Builtin(hir::BuiltinType::Key),
                        vec![Ty::named(TypeRef::Custom(*nil), vec![])],
                    )
                }
            };
            s.constrain(&actual, param);
        }
        if !s.failures.is_empty() {
            return Err("factory argument types do not match generated arguments".into());
        }
        Ok(())
    }
}
