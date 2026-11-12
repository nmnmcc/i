//! Scheme bundles retain bounds and shared variables.
use super::{
    Head, Ty, V, Variance, codec, prove,
    infer::Scheme,
    lits::ValueSet,
    solve::{Mode, Solver},
};
use crate::hir::TypeRef;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

/// A complete, share-preserving interface scheme. Nominal identities are
/// supplied by its consumer so package paths remain stable across programs.
#[derive(Clone, Debug)]
pub struct SchemeArtifact {
    pub(super) catalog: Arc<SchemeCatalog>,
    pub(super) scheme: Scheme,
}

#[derive(Debug)]
pub(super) struct SchemeCatalog {
    pub(super) solver: Mutex<Solver>,
    pub(super) schemes: Vec<Scheme>,
    pub(super) externals: OnceLock<FxHashMap<V, usize>>,
}

impl SchemeCatalog {
    fn external_ids(&self) -> &FxHashMap<V, usize> {
        self.externals.get_or_init(|| {
            let solver = self.solver.lock().expect("scheme catalog");
            let mut ids = FxHashMap::default();
            for scheme in &self.schemes {
                let (vars, _, _) = scheme_roots(&solver, scheme);
                for v in vars {
                    if solver.vars[v.0 as usize].level <= scheme.level {
                        let next = ids.len();
                        ids.entry(v).or_insert(next);
                    }
                }
            }
            ids
        })
    }
}

impl SchemeArtifact {
    /// Identities in the enclosing interface shared with other schemes.
    pub fn external_ids_used(&self) -> Vec<usize> {
        let ids = self.catalog.external_ids();
        let solver = self.catalog.solver.lock().expect("scheme snapshot");
        let (vars, _, _) = scheme_roots(&solver, &self.scheme);
        let mut result: Vec<_> = vars
            .into_iter()
            .filter(|v| solver.vars[v.0 as usize].level <= self.scheme.level)
            .map(|v| ids[&v])
            .collect();
        result.sort_unstable();
        result.dedup();
        result
    }
    /// Whether the scheme includes retained obligations whose meaning is
    /// not captured by its displayed type alone.
    pub fn needs_semantic_comparison(&self) -> bool {
        let solver = self.catalog.solver.lock().expect("scheme snapshot");
        let (vars, _, _) = scheme_roots(&solver, &self.scheme);
        vars.iter().any(|v| {
            solver.vars[v.0 as usize].difference || !solver.vars[v.0 as usize].handled.is_empty()
        })
    }
    pub fn text_with(&self, type_name: &mut impl FnMut(TypeRef) -> String) -> String {
        let external_ids = self.catalog.external_ids();
        encode(
            &self.catalog.solver.lock().expect("scheme snapshot"),
            &self.scheme,
            type_name,
            external_ids,
        )
    }

    pub fn from_bundle(
        text: &str,
        type_ref: &mut impl FnMut(&str) -> Option<TypeRef>,
    ) -> Result<Self, String> {
        SchemeEnvironment::default().read(text, type_ref)
    }
}

/// Reads schemes belonging to one interface while preserving nongeneralized
/// variables shared by several exports.
pub struct SchemeEnvironment {
    solver: Solver,
    externals: FxHashMap<usize, V>,
}

impl Default for SchemeEnvironment {
    fn default() -> Self {
        Self {
            solver: Solver::new(HashMap::new(), Vec::new()),
            externals: FxHashMap::default(),
        }
    }
}

impl SchemeEnvironment {
    pub fn read(
        &mut self,
        text: &str,
        type_ref: &mut impl FnMut(&str) -> Option<TypeRef>,
    ) -> Result<SchemeArtifact, String> {
        let wire: Bundle = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let mut externals = self.externals.clone();
        let scheme = decode(
            text,
            &mut self.solver,
            0,
            type_ref,
            &mut |index, solver| {
                let id = wire.variables.get(index)?.external?;
                Some(*externals.entry(id).or_insert_with(|| solver.fresh_at(0)))
            },
        )?;
        self.externals = externals;
        let ids: FxHashMap<V, usize> = self.externals.iter().map(|(id, v)| (*v, *id)).collect();
        Ok(SchemeArtifact {
            catalog: Arc::new(SchemeCatalog {
                solver: Mutex::new(self.solver.scheme_snapshot()),
                schemes: vec![scheme.clone()],
                externals: OnceLock::from(ids),
            }),
            scheme,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct Bundle {
    version: u32,
    graph: String,
    variables: Vec<Variable>,
    nominals: Vec<Nominal>,
}

#[derive(Serialize, Deserialize)]
struct Nominal {
    name: String,
    variance: Option<Vec<u8>>,
    rows: Option<Vec<u8>>,
}

fn mode_code(mode: Mode) -> u8 {
    match mode {
        Mode::Value => 0,
        Mode::Row => 1,
        Mode::Env => 2,
    }
}

fn read_mode(code: u8) -> Result<Mode, String> {
    match code {
        0 => Ok(Mode::Value),
        1 => Ok(Mode::Row),
        2 => Ok(Mode::Env),
        _ => Err("invalid row mode".into()),
    }
}

fn variance_code(variance: Variance) -> u8 {
    match variance {
        Variance::Bi => 0,
        Variance::Co => 1,
        Variance::Contra => 2,
        Variance::Inv => 3,
    }
}

fn read_variance(code: u8) -> Result<Variance, String> {
    match code {
        0 => Ok(Variance::Bi),
        1 => Ok(Variance::Co),
        2 => Ok(Variance::Contra),
        3 => Ok(Variance::Inv),
        _ => Err("invalid variance".into()),
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Variable {
    depth: Option<u32>,
    external: Option<usize>,
    name: Option<String>,
    mode: u8,
    difference: bool,
    input: bool,
    lower: usize,
    upper: usize,
    handled: usize,
    from: Vec<usize>,
    into: Vec<usize>,
}

fn scheme_roots(s: &Solver, scheme: &Scheme) -> (Vec<V>, FxHashMap<V, usize>, Vec<Ty>) {
    let mut vars = Vec::new();
    let mut index = FxHashMap::default();
    let mut roots = vec![scheme.ty.clone()];
    let mut cursor = 0;
    let mut processed = 0;
    while cursor < roots.len() {
        let mut found = Vec::new();
        super::display::ty_vars(&roots[cursor], &mut found);
        for v in found {
            if let std::collections::hash_map::Entry::Vacant(entry) = index.entry(v) {
                entry.insert(vars.len());
                vars.push(v);
            }
        }
        cursor += 1;
        // Visit each discovered variable, its edges and its bound roots once.
        while processed < vars.len() {
            let state = &s.vars[vars[processed].0 as usize];
            processed += 1;
            for v in state.from.iter().chain(&state.into) {
                if let std::collections::hash_map::Entry::Vacant(entry) = index.entry(*v) {
                    entry.insert(vars.len());
                    vars.push(*v);
                }
            }
            roots.push(Ty::tuple(state.lower.clone()));
            roots.push(Ty::tuple(state.upper.clone()));
            roots.push(Ty::tuple(state.handled.clone()));
        }
    }
    (vars, index, roots)
}

/// Encode all roots together so a binder, node, or constraint shared between
/// the visible type and its bounds has one identity in the bundle.
pub(super) fn encode(
    s: &Solver,
    scheme: &Scheme,
    type_name: &mut impl FnMut(TypeRef) -> String,
    external_ids: &FxHashMap<V, usize>,
) -> String {
    let (vars, index, roots) = scheme_roots(s, scheme);
    let mut metadata = Vec::new();
    for v in &vars {
        let st = &s.vars[v.0 as usize];
        metadata.push(Variable {
            depth: (st.level > scheme.level).then(|| st.level - scheme.level),
            external: (st.level <= scheme.level).then(|| external_ids[v]),
            name: st.name.clone(),
            mode: mode_code(st.row),
            difference: st.difference,
            input: st.input,
            lower: st.lower.len(),
            upper: st.upper.len(),
            handled: st.handled.len(),
            from: st.from.iter().map(|v| index[v]).collect(),
            into: st.into.iter().map(|v| index[v]).collect(),
        });
    }
    let envelope = Ty::tuple(roots);
    let mut nominal_ids = Vec::new();
    named_types(&envelope, &mut nominal_ids);
    let mut nominals: Vec<_> = nominal_ids
        .into_iter()
        .map(|id| Nominal {
            name: type_name(TypeRef::Custom(id)),
            variance: s
                .variance
                .get(&id)
                .map(|vs| vs.iter().copied().map(variance_code).collect()),
            rows: s
                .row_params
                .get(&id)
                .map(|ms| ms.iter().copied().map(mode_code).collect()),
        })
        .collect();
    nominals.sort_by(|a, b| a.name.cmp(&b.name));
    let wire = codec::encode(&envelope, type_name, &mut |v| index[&v].to_string());
    serde_json::to_string(&Bundle {
        version: 3,
        graph: wire,
        variables: metadata,
        nominals,
    })
    .expect("scheme serialization")
}

/// The named types in `t`, with repeats.
fn named_refs(t: &Ty, out: &mut Vec<TypeRef>) {
    map_types(t, &mut |t| {
        if let Ty::Con(Head::Named(r), _, _) = &t {
            out.push(*r);
        }
        t
    });
}

/// The custom types named in `t`, each once, in the order found.
fn named_types(t: &Ty, out: &mut Vec<crate::hir::TypeId>) {
    match t {
        Ty::Con(h, _, xs) => {
            if let Head::Named(TypeRef::Custom(id)) = h
                && !out.contains(id)
            {
                out.push(*id);
            }
            xs.iter().for_each(|x| named_types(x, out));
        }
        Ty::Union(xs) | Ty::Inter(xs) => xs.iter().for_each(|x| named_types(x, out)),
        Ty::Fn(f) => {
            named_types(&f.param, out);
            [&f.ret, &f.err, &f.env]
                .into_iter()
                .for_each(|x| named_types(x, out));
        }
        Ty::Diff(a, hs) => {
            named_types(a, out);
            hs.iter().for_each(|x| named_types(x, out));
        }
        Ty::Both(a, b) => {
            named_types(a, out);
            named_types(b, out);
        }
        Ty::Var(_) | Ty::Never | Ty::Top => {}
    }
}

/// Restore a scheme with one allocation map for the visible type, its bounds,
/// and its labels. The external-variable callback owns cross-scheme sharing.
pub(super) fn decode(
    text: &str,
    solver: &mut Solver,
    base: u32,
    type_ref: &mut impl FnMut(&str) -> Option<TypeRef>,
    external: &mut impl FnMut(usize, &mut Solver) -> Option<V>,
) -> Result<Scheme, String> {
    let bundle: Bundle = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if bundle.version != 3 {
        return Err("invalid scheme version".into());
    }
    let mut external_ids = std::collections::HashSet::new();
    for variable in &bundle.variables {
        match (variable.depth, variable.external) {
            (Some(depth), None) if depth > 0 => (),
            (None, Some(id)) if external_ids.insert(id) => (),
            _ => return Err("invalid scheme variable scope".into()),
        }
    }
    let mut s = solver.clone();
    for nominal in &bundle.nominals {
        let Some(TypeRef::Custom(id)) = type_ref(&nominal.name) else {
            return Err("unknown nominal identity".into());
        };
        if let Some(values) = &nominal.variance {
            let values = values
                .iter()
                .copied()
                .map(read_variance)
                .collect::<Result<Vec<_>, _>>()?;
            if s.variance.get(&id).is_some_and(|prior| prior != &values) {
                return Err("nominal variance differs".into());
            }
            s.variance.insert(id, values);
        }
        if let Some(values) = &nominal.rows {
            let values = values
                .iter()
                .copied()
                .map(read_mode)
                .collect::<Result<Vec<_>, _>>()?;
            if s.row_params.get(&id).is_some_and(|prior| prior != &values) {
                return Err("nominal row modes differ".into());
            }
            s.row_params.insert(id, values);
        }
    }
    let mut vars = Vec::with_capacity(bundle.variables.len());
    for (i, var) in bundle.variables.iter().enumerate() {
        let v = match var.depth {
            Some(d) if d > 0 => s.fresh_at(base.checked_add(d).ok_or("scheme level overflow")?),
            Some(_) => return Err("invalid generalized variable level".into()),
            None => external(i, &mut s).ok_or("unknown external variable")?,
        };
        if vars.contains(&v) || v.0 as usize >= s.vars.len() {
            return Err("invalid scheme variable identity".into());
        }
        vars.push(v);
    }
    let graph = codec::decode(&bundle.graph, type_ref, &mut |name| {
        name.parse::<usize>()
            .ok()
            .and_then(|i| vars.get(i).copied())
    })?;
    let Ty::Con(Head::Shape(_), _, roots) = graph else {
        return Err("scheme graph is not a tuple".into());
    };
    if roots.len() != 1 + vars.len() * 3 {
        return Err("wrong number of scheme roots".into());
    }
    let mut bounds = Vec::with_capacity(vars.len());
    for (i, meta) in bundle.variables.iter().enumerate() {
        let mut parts = Vec::new();
        for (j, count) in [meta.lower, meta.upper, meta.handled]
            .into_iter()
            .enumerate()
        {
            let Ty::Con(Head::Shape(_), values, items) = &roots[1 + i * 3 + j] else {
                return Err("scheme bounds are not a tuple".into());
            };
            if *values != ValueSet::ALL || items.len() != count {
                return Err("invalid scheme bounds".into());
            }
            parts.push(items.clone());
        }
        bounds.push(parts);
    }
    for (i, meta) in bundle.variables.iter().enumerate() {
        let mode = read_mode(meta.mode)?;
        let edges = |ids: &[usize]| {
            ids.iter()
                .map(|id| vars.get(*id).copied().ok_or("unknown variable edge"))
                .collect::<Result<Vec<_>, _>>()
        };
        let from = edges(&meta.from)?;
        let into = edges(&meta.into)?;
        let st = &mut s.vars[vars[i].0 as usize];
        st.name = meta.name.clone();
        st.row = mode;
        st.difference = meta.difference;
        st.input = meta.input;
        st.lower = bounds[i][0].clone();
        st.upper = bounds[i][1].clone();
        st.handled = bounds[i][2].clone();
        st.from = from;
        st.into = into;
        if meta.external.is_some()
            && (vars[i].0 as usize) < solver.vars.len()
            && *st != solver.vars[vars[i].0 as usize]
        {
            return Err("shared external variable has inconsistent constraints".into());
        }
        if !st.handled.is_empty() {
            s.has_handlers = true;
        }
    }
    let ty = roots[0].clone();
    *solver = s;
    Ok(Scheme { level: base, ty })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::BuiltinType;
    use crate::hir::TypeId;
    use crate::types::FnTy;
    use std::collections::HashMap;
    fn encode_test(
        s: &Solver,
        scheme: &Scheme,
        names: &mut impl FnMut(TypeRef) -> String,
    ) -> String {
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
        decode(
            &encoded,
            &mut decoded,
            0,
            &mut |_| None,
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
            let root = Ty::Fn(Box::new(FnTy::new(vec![], Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]), Ty::Never, row)));
            let scheme = Scheme {
                level: 0,
                ty: root,
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
            let root = Ty::Fn(Box::new(FnTy::new(vec![], Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]), Ty::Never, Ty::Var(row))));
            let scheme = Scheme {
                level: 0,
                ty: root,
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
            let body = Ty::Fn(Box::new(FnTy::new(vec![], result.clone(), Ty::Never, Ty::Var(row))));
            let root = Ty::Fn(Box::new(FnTy::new(vec![(Some("body".into()), body)], result, Ty::Never, Ty::Var(row))));
            let scheme = Scheme {
                level: 0,
                ty: root,
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


}

/// An alias template with formal positions retained for package comparison.
#[derive(Clone, Debug)]
pub struct AliasArtifact {
    template: super::alias::Template,
    solver: Arc<Mutex<Solver>>,
    modes: Vec<Mode>,
}

impl AliasArtifact {
    pub(super) fn new(
        template: super::alias::Template,
        solver: &Solver,
        id: crate::hir::TypeId,
    ) -> Self {
        let mut snapshot = Solver::new(solver.variance.clone(), Vec::new());
        snapshot.row_params = solver.row_params.clone();
        let modes = solver
            .row_params
            .get(&id)
            .cloned()
            .unwrap_or_else(|| vec![Mode::Value; template.params]);
        Self {
            template,
            solver: Arc::new(Mutex::new(snapshot)),
            modes,
        }
    }

    /// Compare denoted sets with the same rigid formal positions. The name
    /// callbacks identify nominal types independently of each program's IDs.
    /// `None` means the representation can be reported as changed without
    /// claiming that the denoted sets differ.
    pub fn equivalent(
        &self,
        other: &Self,
        left_name: &mut impl FnMut(TypeRef) -> String,
        right_name: &mut impl FnMut(TypeRef) -> String,
    ) -> Result<Option<bool>, String> {
        if self.template.params != other.template.params
            || self.template.slots != other.template.slots
            || self.modes != other.modes
        {
            return Ok(Some(false));
        }
        let left = self.solver.lock().expect("alias snapshot").clone();
        let right = other.solver.lock().expect("alias snapshot").clone();
        let mut solver = Solver::new(HashMap::new(), Vec::new());
        for _ in 0..self.template.slots {
            let v = solver.fresh_at(0);
            solver.vars[v.0 as usize].input = true;
        }
        let mut identities = HashMap::<String, TypeRef>::new();
        let mut next_id = 0u32;
        let mut import = |artifact: &AliasArtifact,
                          source: &Solver,
                          name: &mut dyn FnMut(TypeRef) -> String|
         -> Result<Ty, String> {
            let mut refs = Vec::new();
            named_refs(&artifact.template.ty, &mut refs);
            let mut names = HashMap::new();
            for ty in refs {
                let key = match ty {
                    TypeRef::Builtin(_) => format!("builtin:{}", name(ty)),
                    TypeRef::Custom(_) => format!("nominal:{}", name(ty)),
                };
                let mapped = match ty {
                    TypeRef::Builtin(_) => ty,
                    TypeRef::Custom(old) => {
                        let mapped = *identities.entry(key.clone()).or_insert_with(|| {
                            let id = crate::hir::TypeId(next_id);
                            next_id += 1;
                            TypeRef::Custom(id)
                        });
                        let TypeRef::Custom(id) = mapped else {
                            unreachable!()
                        };
                        if let Some(vs) = source.variance.get(&old) {
                            if solver.variance.get(&id).is_some_and(|prior| prior != vs) {
                                return Err("nominal variance differs".into());
                            }
                            solver.variance.insert(id, vs.clone());
                        }
                        if let Some(ms) = source.row_params.get(&old) {
                            if solver.row_params.get(&id).is_some_and(|prior| prior != ms) {
                                return Err("nominal row modes differ".into());
                            }
                            solver.row_params.insert(id, ms.clone());
                        }
                        mapped
                    }
                };
                if names
                    .insert(key, mapped)
                    .is_some_and(|prior| prior != mapped)
                {
                    return Err("duplicate nominal identity".into());
                }
            }
            Ok(map_types(&artifact.template.ty, &mut |t| match t {
                Ty::Con(Head::Named(ty), values, args) => {
                    let key = match ty {
                        TypeRef::Builtin(_) => format!("builtin:{}", name(ty)),
                        TypeRef::Custom(_) => format!("nominal:{}", name(ty)),
                    };
                    Ty::Con(Head::Named(names[&key]), values, args)
                }
                t => t,
            }))
        };
        let a = import(self, &left, left_name)?;
        let b = import(other, &right, right_name)?;
        // The subtype relation intentionally ignores argument labels, while
        // the package interface must preserve named-call availability. Wrap
        // each function in a disjoint tagged wrapper encoding its labels.
        // The ordinary proof then compares both the function type
        // and its call protocol, including inside records and unions.
        let mut tags = HashMap::<Vec<Option<String>>, crate::hir::TypeId>::new();
        let mut tag = |ty: Ty, _: bool| {
            let Ty::Fn(function) = &ty else {
                return ty;
            };
            let labels = function.labels.clone();
            let id = *tags.entry(labels).or_insert_with(|| {
                let id = crate::hir::TypeId(next_id);
                next_id += 1;
                solver.variance.insert(id, vec![Variance::Co]);
                solver.comparison_wrappers.insert(id);
                id
            });
            Ty::named(TypeRef::Custom(id), vec![ty])
        };
        let a = map_types(&a, &mut |t| tag(t, false));
        let b = map_types(&b, &mut |t| tag(t, false));
        if a == b {
            return Ok(Some(true));
        }
        let prove = |a: &Ty, b: &Ty| {
            prove::prove(&solver, a, b, &mut prove::Demand::new(prove::Demand::LIMIT))
        };
        match (prove(&a, &b), prove(&b, &a)) {
            (prove::Verdict::Proven, prove::Verdict::Proven) => Ok(Some(true)),
            (prove::Verdict::Refuted, _) | (_, prove::Verdict::Refuted) => Ok(Some(false)),
            (prove::Verdict::ResourceLimit, _) | (_, prove::Verdict::ResourceLimit) => {
                Err("alias equivalence exceeded the proof limit".into())
            }
            _ if self.modes.iter().all(|m| *m == Mode::Value)
                && self.template.params == self.template.slots =>
            {
                let int = Ty::named(TypeRef::Builtin(crate::hir::BuiltinType::Int), Vec::new());
                let string = Ty::named(
                    TypeRef::Builtin(crate::hir::BuiltinType::String),
                    Vec::new(),
                );
                for selected in 0..self.template.params {
                    let replace = |v: V| {
                        if v.0 as usize == selected {
                            int.clone()
                        } else {
                            string.clone()
                        }
                    };
                    let a = super::alias::map_vars(&a, &mut replace.clone());
                    let b = super::alias::map_vars(&b, &mut replace.clone());
                    if matches!(prove(&a, &b), prove::Verdict::Refuted)
                        || matches!(prove(&b, &a), prove::Verdict::Refuted)
                    {
                        return Ok(Some(false));
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

/// The result of a read-only interface comparison. An unfinished proof is
/// distinct from a counterexample and from exhausting its work budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemeComparison {
    Equivalent(Vec<Vec<(usize, usize)>>),
    Different,
    Unknown,
    ResourceLimit,
}

fn remap_scheme_type(ty: &Ty, vars: &[usize]) -> Ty {
    super::alias::map_vars(ty, &mut |v| Ty::Var(V(vars[v.0 as usize] as u32)))
}

/// `ty` with each type changed by `f`, from the leaves up.
fn map_types(ty: &Ty, f: &mut impl FnMut(Ty) -> Ty) -> Ty {
    let shallow = match ty {
        Ty::Con(h, vs, args) => Ty::Con(*h, vs.clone(), args.iter().map(|a| map_types(a, f)).collect()),
        Ty::Fn(fun) => Ty::Fn(Box::new(super::FnTy {
            param: map_types(&fun.param, f),
            labels: fun.labels.clone(),
            ret: map_types(&fun.ret, f),
            err: map_types(&fun.err, f),
            env: map_types(&fun.env, f),
        })),
        Ty::Union(xs) => Ty::Union(xs.iter().map(|x| map_types(x, f)).collect()),
        Ty::Inter(xs) => Ty::Inter(xs.iter().map(|x| map_types(x, f)).collect()),
        Ty::Diff(b, hs) => Ty::Diff(
            Box::new(map_types(b, f)),
            hs.iter().map(|x| map_types(x, f)).collect(),
        ),
        Ty::Both(a, b) => Ty::Both(Box::new(map_types(a, f)), Box::new(map_types(b, f))),
        t => t.clone(),
    };
    f(shallow)
}

/// The variables of `ty`.
fn variables(ty: &Ty) -> Vec<V> {
    let mut out = Vec::new();
    super::display::ty_vars(ty, &mut out);
    out
}

fn tag_scheme_type(
    ty: &Ty,
    tags: &mut HashMap<Vec<Option<String>>, crate::hir::TypeId>,
    next: &mut u32,
) -> Ty {
    map_types(ty, &mut |ty| {
        let Ty::Fn(f) = &ty else {
            return ty;
        };
        let labels = f.labels.clone();
        let id = *tags.entry(labels).or_insert_with(|| {
            let id = crate::hir::TypeId(*next);
            *next += 1;
            id
        });
        Ty::named(TypeRef::Custom(id), vec![ty])
    })
}

fn tag_scheme_solver(
    solver: &mut Solver,
    root: &Ty,
    tags: &mut HashMap<Vec<Option<String>>, crate::hir::TypeId>,
    next: &mut u32,
) -> Ty {
    let root = tag_scheme_type(root, tags, next);
    for state in &mut solver.vars {
        for ty in state
            .lower
            .iter_mut()
            .chain(&mut state.upper)
            .chain(&mut state.handled)
        {
            *ty = tag_scheme_type(ty, tags, next);
        }
    }
    root
}

fn proves(
    solver: &Solver,
    left: &Ty,
    right: &Ty,
    mode: Mode,
    demand: &mut prove::Demand,
) -> Result<bool, ()> {
    match prove::prove_mode(solver, left, right, mode, demand) {
        prove::Verdict::Proven => Ok(true),
        prove::Verdict::ResourceLimit => Err(()),
        prove::Verdict::Refuted | prove::Verdict::Unknown => Ok(false),
    }
}

// `subject` is interpreted only under its own constraints. Every condition of
// `target` must follow without adding a bound to either solver.
struct SchemeMapping<'a> {
    variables: &'a [usize],
}

fn entails_mapped(
    subject: &Solver,
    subject_root: &Ty,
    target: &Solver,
    target_root: &Ty,
    mapped: SchemeMapping<'_>,
    demand: &mut prove::Demand,
) -> Result<bool, ()> {
    let SchemeMapping { variables: mapping } = mapped;
    for (i, wanted) in target.vars.iter().enumerate() {
        let actual = &subject.vars[mapping[i]];
        if actual.row != wanted.row
            || actual.difference != wanted.difference
            || actual.input != wanted.input
        {
            return Ok(false);
        }
        let variable = Ty::Var(V(mapping[i] as u32));
        let mode = actual.row;
        for lower in &wanted.lower {
            let lower = remap_scheme_type(lower, mapping);
            if !proves(subject, &lower, &variable, mode, demand)? {
                return Ok(false);
            }
        }
        for upper in &wanted.upper {
            let upper = remap_scheme_type(upper, mapping);
            if !proves(subject, &variable, &upper, mode, demand)? {
                return Ok(false);
            }
        }
        for v in &wanted.from {
            if !proves(
                subject,
                &Ty::Var(V(mapping[v.0 as usize] as u32)),
                &variable,
                mode,
                demand,
            )? {
                return Ok(false);
            }
        }
        for v in &wanted.into {
            if !proves(
                subject,
                &variable,
                &Ty::Var(V(mapping[v.0 as usize] as u32)),
                mode,
                demand,
            )? {
                return Ok(false);
            }
        }
        for handled in &wanted.handled {
            let handled = remap_scheme_type(handled, mapping);
            let mut found = false;
            for own in &actual.handled {
                if proves(subject, own, &handled, Mode::Env, demand)?
                    && proves(subject, &handled, own, Mode::Env, demand)?
                {
                    found = true;
                    break;
                }
            }
            if !found {
                return Ok(false);
            }
        }
    }
    let target_root = remap_scheme_type(target_root, mapping);
    proves(subject, subject_root, &target_root, Mode::Value, demand)
}

/// Whether `v` occurs in `ty` in an output place and in an input place.
fn sites(ty: &Ty, v: V) -> (bool, bool) {
    fn go(ty: &Ty, v: V, pos: bool, out: &mut (bool, bool)) {
        match ty {
            Ty::Var(w) if *w == v => {
                if pos {
                    out.0 = true;
                } else {
                    out.1 = true;
                }
            }
            Ty::Con(_, _, xs) | Ty::Union(xs) | Ty::Inter(xs) => {
                xs.iter().for_each(|x| go(x, v, pos, out))
            }
            Ty::Fn(f) => {
                go(&f.param, v, !pos, out);
                [&f.ret, &f.err, &f.env]
                    .into_iter()
                    .for_each(|x| go(x, v, pos, out));
            }
            Ty::Diff(a, hs) => {
                go(a, v, pos, out);
                hs.iter().for_each(|x| go(x, v, pos, out));
            }
            Ty::Both(a, b) => {
                go(a, v, true, out);
                go(b, v, false, out);
            }
            _ => {}
        }
    }
    let mut out = (false, false);
    go(ty, v, true, &mut out);
    out
}

// A closed service form supplies an explicit counterexample when the only
// remaining quantified row is otherwise unconstrained.
fn handler_witness(source: &Solver, source_root: &Ty, target: &Solver, target_root: &Ty) -> bool {
    if source.vars.len() != 1 || target.vars.len() != 1 {
        return false;
    }
    let (positive, negative) = sites(source_root, V(0));
    if !positive || !negative {
        return false;
    }
    let mut demand = prove::Demand::new(prove::Demand::LIMIT);
    if !matches!(
        proves(source, source_root, target_root, Mode::Value, &mut demand),
        Ok(true)
    ) || !matches!(
        proves(target, target_root, source_root, Mode::Value, &mut demand),
        Ok(true)
    ) {
        return false;
    }
    let a = &source.vars[0];
    let b = &target.vars[0];
    if a.row != Mode::Env
        || b.row != Mode::Env
        || !a.lower.is_empty()
        || !a.upper.is_empty()
        || !a.from.is_empty()
        || !a.into.is_empty()
        || !b.lower.is_empty()
        || !b.upper.is_empty()
        || !b.from.is_empty()
        || !b.into.is_empty()
    {
        return false;
    }
    for form in &a.handled {
        if !variables(form).is_empty() {
            continue;
        }
        if !matches!(form.value_head(), Some(Head::Named(TypeRef::Custom(_)))) {
            continue;
        }
        let mut accepts = source.clone();
        accepts.constrain_env(form, &Ty::Var(V(0)));
        let mut rejects = target.clone();
        rejects.constrain_env(form, &Ty::Var(V(0)));
        if accepts.failures.is_empty() && definite_failure(&rejects) {
            return true;
        }
    }
    false
}

fn definite_failure(solver: &Solver) -> bool {
    use super::solve::FailKind;
    !solver.failures.is_empty()
        && solver.failures.iter().all(|f| {
            matches!(
                f.kind,
                FailKind::Mismatch
                    | FailKind::Values
                    | FailKind::Invariant
                    | FailKind::EnvForms
            )
        })
}

impl SchemeArtifact {
    /// Compare complete schemes under their own retained assumptions.
    pub fn compare(
        &self,
        other: &Self,
        left_name: &mut impl FnMut(TypeRef) -> String,
        right_name: &mut impl FnMut(TypeRef) -> String,
    ) -> Result<SchemeComparison, String> {
        self.compare_impl(other, left_name, right_name, false)
    }

    fn compare_impl(
        &self,
        other: &Self,
        left_name: &mut impl FnMut(TypeRef) -> String,
        right_name: &mut impl FnMut(TypeRef) -> String,
        closed_environment: bool,
    ) -> Result<SchemeComparison, String> {
        let mut identities = HashMap::<String, TypeRef>::new();
        let mut next = 0u32;
        let mut identify = |t: TypeRef, name: &mut dyn FnMut(TypeRef) -> String| {
            let key = match t {
                TypeRef::Builtin(_) => format!("builtin:{}", name(t)),
                TypeRef::Custom(_) => format!("nominal:{}", name(t)),
            };
            let canonical = match t {
                TypeRef::Builtin(_) => t,
                TypeRef::Custom(_) => *identities.entry(key.clone()).or_insert_with(|| {
                    let id = TypeRef::Custom(crate::hir::TypeId(next));
                    next += 1;
                    id
                }),
            };
            if let Some(old) = identities.insert(key.clone(), canonical) {
                assert_eq!(old, canonical, "type identity collision");
            }
            key
        };
        let left_wire = self.text_with(&mut |t| identify(t, left_name));
        let right_wire = other.text_with(&mut |t| identify(t, right_name));
        drop(identify);
        let mut left_bundle: Bundle =
            serde_json::from_str(&left_wire).map_err(|e| e.to_string())?;
        let mut right_bundle: Bundle =
            serde_json::from_str(&right_wire).map_err(|e| e.to_string())?;
        let mut left_env = SchemeEnvironment::default();
        let mut right_env = SchemeEnvironment::default();
        let left = left_env.read(&left_wire, &mut |name| identities.get(name).copied())?;
        let right = right_env.read(&right_wire, &mut |name| identities.get(name).copied())?;
        let mut left_solver = left.catalog.solver.lock().expect("scheme snapshot").clone();
        let mut right_solver = right
            .catalog
            .solver
            .lock()
            .expect("scheme snapshot")
            .clone();
        let mut left_type = left.scheme.ty.clone();
        let mut right_type = right.scheme.ty.clone();
        substitute_exact_variables(
            &mut left_solver,
            &mut left_type,
            &mut left_bundle.variables,
            closed_environment,
        );
        substitute_exact_variables(
            &mut right_solver,
            &mut right_type,
            &mut right_bundle.variables,
            closed_environment,
        );
        collapse_equal_variables(
            &mut left_solver,
            &mut left_type,
            &mut left_bundle.variables,
            closed_environment,
        );
        collapse_equal_variables(
            &mut right_solver,
            &mut right_type,
            &mut right_bundle.variables,
            closed_environment,
        );
        project_hidden(
            &mut left_solver,
            &mut left_type,
            &mut left_bundle.variables,
            closed_environment,
        );
        project_hidden(
            &mut right_solver,
            &mut right_type,
            &mut right_bundle.variables,
            closed_environment,
        );
        for (id, values) in right_solver.variance.clone() {
            if left_solver
                .variance
                .get(&id)
                .is_some_and(|other| other != &values)
            {
                return Ok(SchemeComparison::Different);
            }
            left_solver.variance.insert(id, values);
        }
        for (id, values) in left_solver.variance.clone() {
            right_solver.variance.insert(id, values);
        }
        for (id, values) in right_solver.row_params.clone() {
            if left_solver
                .row_params
                .get(&id)
                .is_some_and(|other| other != &values)
            {
                return Ok(SchemeComparison::Different);
            }
            left_solver.row_params.insert(id, values);
        }
        for (id, values) in left_solver.row_params.clone() {
            right_solver.row_params.insert(id, values);
        }
        let raw_left = left_solver.scheme_snapshot();
        let raw_right = right_solver.scheme_snapshot();
        let tag_base = next;
        let cover = || {
            let mut demand = prove::Demand::new(prove::Demand::LIMIT);
            let left = CoverSide {
                solver: &raw_left,
                root: &left_type,
                metadata: &left_bundle.variables,
            };
            let right = CoverSide {
                solver: &raw_right,
                root: &right_type,
                metadata: &right_bundle.variables,
            };
            if !find_cover(left, right, tag_base, &mut demand)? {
                return Ok(false);
            }
            find_cover(right, left, tag_base, &mut demand)
        };
        let mut tags = HashMap::new();
        let left_root = tag_scheme_solver(&mut left_solver, &left_type, &mut tags, &mut next);
        let right_root = tag_scheme_solver(&mut right_solver, &right_type, &mut tags, &mut next);
        for id in tags.values() {
            left_solver.variance.insert(*id, vec![Variance::Co]);
            right_solver.variance.insert(*id, vec![Variance::Co]);
            left_solver.comparison_wrappers.insert(*id);
            right_solver.comparison_wrappers.insert(*id);
        }
        if left_bundle.variables.len() != right_bundle.variables.len() {
            return Ok(match cover() {
                Ok(true) => SchemeComparison::Equivalent(vec![vec![]]),
                Err(()) => SchemeComparison::ResourceLimit,
                Ok(false) => SchemeComparison::Unknown,
            });
        }
        let count = left_bundle.variables.len();
        let mut candidates = Vec::with_capacity(count);
        for b in &right_bundle.variables {
            let mut matches = Vec::new();
            for (i, a) in left_bundle.variables.iter().enumerate() {
                if a.depth == b.depth
                    && a.external.is_some() == b.external.is_some()
                    && a.mode == b.mode
                    && a.difference == b.difference
                    && a.input == b.input
                {
                    matches.push(i);
                }
            }
            candidates.push(matches);
        }
        let mut mapping = vec![usize::MAX; count];
        let mut chosen = vec![false; count];
        let mut next_candidate = vec![0usize; count];
        let mut depth = 0usize;
        let mut solutions = Vec::new();
        let mut demand = prove::Demand::new(prove::Demand::LIMIT);
        loop {
            if demand.charge().is_err() {
                return Ok(SchemeComparison::ResourceLimit);
            }
            if depth == count {
                let mut reverse = vec![0usize; count];
                for (i, &j) in mapping.iter().enumerate() {
                    reverse[j] = i;
                }
                let first = entails_mapped(
                    &left_solver,
                    &left_root,
                    &right_solver,
                    &right_root,
                    SchemeMapping {
                        variables: &mapping,
                    },
                    &mut demand,
                );
                let second = if matches!(first, Ok(true)) {
                    entails_mapped(
                        &right_solver,
                        &right_root,
                        &left_solver,
                        &left_root,
                        SchemeMapping {
                            variables: &reverse,
                        },
                        &mut demand,
                    )
                } else {
                    Ok(false)
                };
                match (first, second) {
                    (Err(_), _) | (_, Err(_)) => return Ok(SchemeComparison::ResourceLimit),
                    (Ok(true), Ok(true)) => {
                        let mut pairs: Vec<_> = right_bundle
                            .variables
                            .iter()
                            .enumerate()
                            .filter_map(|(i, var)| {
                                Some((left_bundle.variables[mapping[i]].external?, var.external?))
                            })
                            .collect();
                        pairs.sort_unstable();
                        pairs.dedup();
                        if pairs.is_empty() || closed_environment {
                            return Ok(SchemeComparison::Equivalent(vec![pairs]));
                        }
                        if !solutions.contains(&pairs) {
                            solutions.push(pairs);
                        }
                    }
                    _ if count == 1
                        && (handler_witness(
                            &left_solver,
                            &left_root,
                            &right_solver,
                            &right_root,
                        ) || handler_witness(
                            &right_solver,
                            &right_root,
                            &left_solver,
                            &left_root,
                        )) =>
                    {
                        return Ok(SchemeComparison::Different);
                    }
                    _ => (),
                }
                if depth == 0 {
                    break;
                }
                depth -= 1;
                chosen[mapping[depth]] = false;
                continue;
            }
            let mut found = None;
            while next_candidate[depth] < candidates[depth].len() {
                let candidate = candidates[depth][next_candidate[depth]];
                next_candidate[depth] += 1;
                if !chosen[candidate] {
                    found = Some(candidate);
                    break;
                }
            }
            if let Some(candidate) = found {
                mapping[depth] = candidate;
                chosen[candidate] = true;
                depth += 1;
                if depth < count {
                    next_candidate[depth] = 0;
                }
            } else {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                chosen[mapping[depth]] = false;
            }
        }
        if solutions.is_empty() {
            Ok(match cover() {
                Ok(true) => SchemeComparison::Equivalent(vec![vec![]]),
                Err(()) => SchemeComparison::ResourceLimit,
                Ok(false) => SchemeComparison::Unknown,
            })
        } else {
            Ok(SchemeComparison::Equivalent(solutions))
        }
    }
}

/// Eliminate an invisible, ordinary existential variable by composing its
/// lower and upper constraints. Only direct variable edges are projected;
/// structural occurrences remain in the scheme and need the general solver.
fn project_hidden(
    solver: &mut Solver,
    root: &mut Ty,
    metadata: &mut Vec<Variable>,
    closed_environment: bool,
) {
    loop {
        let visible = variables(root);
        let candidate = (0..solver.vars.len()).find(|&i| {
            let v = V(i as u32);
            let st = &solver.vars[i];
            if visible.contains(&v)
                || (metadata[i].depth.is_none() && !closed_environment)
                || st.input
                || st.row != Mode::Value
                || st.difference
                || !st.handled.is_empty()
            {
                return false;
            }
            for state in &solver.vars {
                for ty in state.lower.iter().chain(&state.upper).chain(&state.handled) {
                    if variables(ty).contains(&v)
                        && !matches!(ty, Ty::Var(w) if *w == v)
                    {
                        return false;
                    }
                }
            }
            let lower = st
                .lower
                .iter()
                .cloned()
                .chain(st.from.iter().copied().map(Ty::Var))
                .filter(|t| *t != Ty::Var(v))
                .collect::<Vec<_>>();
            let upper = st
                .upper
                .iter()
                .cloned()
                .chain(st.into.iter().copied().map(Ty::Var))
                .filter(|t| *t != Ty::Var(v))
                .collect::<Vec<_>>();
            lower.iter().all(|l| {
                upper
                    .iter()
                    .all(|u| matches!(l, Ty::Var(_)) || matches!(u, Ty::Var(_)))
            })
        });
        let Some(i) = candidate else { break };
        let v = V(i as u32);
        let state = solver.vars[i].clone();
        let lower = state
            .lower
            .into_iter()
            .chain(state.from.into_iter().map(Ty::Var))
            .filter(|t| *t != Ty::Var(v))
            .collect::<Vec<_>>();
        let upper = state
            .upper
            .into_iter()
            .chain(state.into.into_iter().map(Ty::Var))
            .filter(|t| *t != Ty::Var(v))
            .collect::<Vec<_>>();
        for state in &mut solver.vars {
            state.lower.retain(|t| *t != Ty::Var(v));
            state.upper.retain(|t| *t != Ty::Var(v));
            state.from.retain(|w| *w != v);
            state.into.retain(|w| *w != v);
        }
        for l in &lower {
            for u in &upper {
                solver.constrain(l, u);
            }
        }
        assert!(
            solver.failures.is_empty(),
            "valid scheme projection must preserve consistent bounds"
        );
        solver.vars.remove(i);
        metadata.remove(i);
        let remap = |var: V| {
            assert_ne!(var, v, "projected variable remains in scheme");
            Ty::Var(V(if var.0 > v.0 { var.0 - 1 } else { var.0 }))
        };
        *root = super::alias::map_vars(root, &mut remap.clone());
        for state in &mut solver.vars {
            for ty in state
                .lower
                .iter_mut()
                .chain(&mut state.upper)
                .chain(&mut state.handled)
            {
                *ty = super::alias::map_vars(ty, &mut remap.clone());
            }
            for var in state.from.iter_mut().chain(&mut state.into) {
                assert_ne!(*var, v, "projected edge remains");
                if var.0 > v.0 {
                    var.0 -= 1;
                }
            }
        }
        *solver = solver.scheme_snapshot();
    }
}

fn aggregate_schemes(schemes: &[SchemeArtifact]) -> Result<SchemeArtifact, String> {
    let mut identities = HashMap::<String, TypeRef>::new();
    let mut wires = Vec::with_capacity(schemes.len());
    for artifact in schemes {
        wires.push(artifact.text_with(&mut |t| {
            let name = format!("{t:?}");
            if let Some(old) = identities.insert(name.clone(), t) {
                assert_eq!(old, t, "interface type identity collision");
            }
            name
        }));
    }
    let mut environment = SchemeEnvironment::default();
    let mut roots = Vec::with_capacity(schemes.len());
    for wire in &wires {
        let artifact = environment.read(wire, &mut |name| identities.get(name).copied())?;
        roots.push(artifact.scheme.ty);
    }
    let scheme = Scheme {
        level: 0,
        ty: Ty::tuple(roots),
    };
    let ids: FxHashMap<V, usize> = environment
        .externals
        .iter()
        .map(|(id, v)| (*v, *id))
        .collect();
    Ok(SchemeArtifact {
        catalog: Arc::new(SchemeCatalog {
            solver: Mutex::new(environment.solver.scheme_snapshot()),
            schemes: vec![scheme.clone()],
            externals: OnceLock::from(ids),
        }),
        scheme,
    })
}

/// Compare a closed group together so shared nongeneralized variables cannot
/// be independently renamed in separate exported schemes.
pub fn compare_scheme_group(
    left: &[SchemeArtifact],
    right: &[SchemeArtifact],
    left_name: &mut impl FnMut(TypeRef) -> String,
    right_name: &mut impl FnMut(TypeRef) -> String,
) -> Result<SchemeComparison, String> {
    if left.len() != right.len() {
        return Ok(SchemeComparison::Different);
    }
    let a = aggregate_schemes(left)?;
    let b = aggregate_schemes(right)?;
    a.compare_impl(&b, left_name, right_name, true)
}

/// Contract mutually constrained variables while preserving every bound and
/// obligation. Only variables with compatible scope and row identities can be
/// identified; external identities are contracted only in a closed group.
fn collapse_equal_variables(
    solver: &mut Solver,
    root: &mut Ty,
    metadata: &mut Vec<Variable>,
    closed: bool,
) {
    let n = solver.vars.len();
    let mut edges = vec![Vec::new(); n];
    let mut reverse = vec![Vec::new(); n];
    let mut add = |a: usize, b: usize| {
        if a != b {
            edges[a].push(b);
            reverse[b].push(a);
        }
    };
    for (i, state) in solver.vars.iter().enumerate() {
        for ty in &state.upper {
            if let Ty::Var(v) = ty {
                add(i, v.0 as usize);
            }
        }
        for ty in &state.lower {
            if let Ty::Var(v) = ty {
                add(v.0 as usize, i);
            }
        }
        for v in &state.into {
            add(i, v.0 as usize);
        }
        for v in &state.from {
            add(v.0 as usize, i);
        }
    }
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);
    for start in 0..n {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut stack = vec![(start, 0usize)];
        while let Some((node, at)) = stack.last_mut() {
            if *at == edges[*node].len() {
                order.push(*node);
                stack.pop();
                continue;
            }
            let child = edges[*node][*at];
            *at += 1;
            if !visited[child] {
                visited[child] = true;
                stack.push((child, 0));
            }
        }
    }
    visited.fill(false);
    let mut representative: Vec<_> = (0..n).collect();
    for start in order.into_iter().rev() {
        if visited[start] {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![start];
        visited[start] = true;
        while let Some(node) = stack.pop() {
            component.push(node);
            for &child in &reverse[node] {
                if !visited[child] {
                    visited[child] = true;
                    stack.push(child);
                }
            }
        }
        if component.len() < 2 {
            continue;
        }
        let first = component[0];
        if component.iter().any(|&i| {
            solver.vars[i].row != solver.vars[first].row
                || metadata[i].depth != metadata[first].depth
                || (metadata[i].external.is_some() && !closed)
        }) {
            continue;
        }
        let rep = *component.iter().min().expect("nonempty component");
        for i in component {
            representative[i] = rep;
        }
    }
    if representative.iter().enumerate().all(|(i, rep)| i == *rep) {
        return;
    }
    let mut dense = vec![0usize; n];
    let mut kept = Vec::new();
    for i in 0..n {
        if representative[i] == i {
            dense[i] = kept.len();
            kept.push(i);
        }
    }
    for i in 0..n {
        dense[i] = dense[representative[i]];
    }
    let remap = |v: V| Ty::Var(V(dense[v.0 as usize] as u32));
    *root = super::alias::map_vars(root, &mut remap.clone());
    let mut merged = kept
        .iter()
        .map(|&i| {
            let mut state = solver.vars[i].clone();
            state.lower.clear();
            state.upper.clear();
            state.handled.clear();
            state.from.clear();
            state.into.clear();
            state.input = false;
            state.difference = false;
            state
        })
        .collect::<Vec<_>>();
    for (i, state) in solver.vars.iter().enumerate() {
        let index = dense[i];
        let out = &mut merged[index];
        out.input |= state.input;
        out.difference |= state.difference;
        for ty in &state.lower {
            let ty = super::alias::map_vars(ty, &mut remap.clone());
            if ty != Ty::Var(V(index as u32)) && !out.lower.contains(&ty) {
                out.lower.push(ty);
            }
        }
        for ty in &state.upper {
            let ty = super::alias::map_vars(ty, &mut remap.clone());
            if ty != Ty::Var(V(index as u32)) && !out.upper.contains(&ty) {
                out.upper.push(ty);
            }
        }
        for ty in &state.handled {
            let ty = super::alias::map_vars(ty, &mut remap.clone());
            if !out.handled.contains(&ty) {
                out.handled.push(ty);
            }
        }
        for &v in &state.from {
            let v = V(dense[v.0 as usize] as u32);
            if v.0 as usize != index && !out.from.contains(&v) {
                out.from.push(v);
            }
        }
        for &v in &state.into {
            let v = V(dense[v.0 as usize] as u32);
            if v.0 as usize != index && !out.into.contains(&v) {
                out.into.push(v);
            }
        }
    }
    let mut new_meta: Vec<_> = kept.iter().map(|&i| metadata[i].clone()).collect();
    for (i, state) in merged.iter().enumerate() {
        new_meta[i].input = state.input;
        new_meta[i].difference = state.difference;
    }
    solver.vars = merged;
    *metadata = new_meta;
}

/// Replace a variable fixed to one exact structural type. Its other bounds
/// are imposed on that type before the binder is removed.
fn substitute_exact_variables(
    solver: &mut Solver,
    root: &mut Ty,
    metadata: &mut Vec<Variable>,
    closed: bool,
) {
    loop {
        let candidate = solver.vars.iter().enumerate().find_map(|(i, state)| {
            if (metadata[i].depth.is_none() && !closed)
                || state.input
                || state.row != Mode::Value
                || state.difference
                || !state.handled.is_empty()
            {
                return None;
            }
            let v = V(i as u32);
            state
                .lower
                .iter()
                .find(|ty| {
                    !matches!(ty, Ty::Var(_))
                        && state.upper.contains(ty)
                        && !variables(ty).contains(&v)
                })
                .cloned()
                .map(|ty| (i, ty))
        });
        let Some((i, replacement)) = candidate else {
            break;
        };
        let v = V(i as u32);
        let state = solver.vars[i].clone();
        let substitute = |ty: &Ty| {
            super::alias::map_vars(ty, &mut |w| {
                if w == v {
                    replacement.clone()
                } else {
                    Ty::Var(w)
                }
            })
        };
        *root = substitute(root);
        for current in &mut solver.vars {
            for ty in current
                .lower
                .iter_mut()
                .chain(&mut current.upper)
                .chain(&mut current.handled)
            {
                *ty = substitute(ty);
            }
            current.from.retain(|w| *w != v);
            current.into.retain(|w| *w != v);
        }
        for lower in state.lower.iter().filter(|t| **t != Ty::Var(v)) {
            solver.constrain(&substitute(lower), &replacement);
        }
        for upper in state.upper.iter().filter(|t| **t != Ty::Var(v)) {
            solver.constrain(&replacement, &substitute(upper));
        }
        for from in state.from.iter().filter(|w| **w != v) {
            solver.constrain(&Ty::Var(*from), &replacement);
        }
        for into in state.into.iter().filter(|w| **w != v) {
            solver.constrain(&replacement, &Ty::Var(*into));
        }
        assert!(
            solver.failures.is_empty(),
            "valid exact substitution must preserve consistent bounds"
        );
        solver.vars.remove(i);
        metadata.remove(i);
        let renumber = |w: V| {
            assert_ne!(w, v, "substituted binder remains");
            Ty::Var(V(if w.0 > v.0 { w.0 - 1 } else { w.0 }))
        };
        *root = super::alias::map_vars(root, &mut renumber.clone());
        for current in &mut solver.vars {
            for ty in current
                .lower
                .iter_mut()
                .chain(&mut current.upper)
                .chain(&mut current.handled)
            {
                *ty = super::alias::map_vars(ty, &mut renumber.clone());
            }
            for var in current.from.iter_mut().chain(&mut current.into) {
                assert_ne!(*var, v, "substituted edge remains");
                if var.0 > v.0 {
                    var.0 -= 1;
                }
            }
        }
        *solver = solver.scheme_snapshot();
    }
}


/// Every part of `ty`, `ty` included.
fn subterms(ty: &Ty, out: &mut Vec<Ty>) {
    map_types(ty, &mut |t| {
        if !out.contains(&t) {
            out.push(t.clone());
        }
        t
    });
}

#[derive(Clone, Copy)]
struct CoverSide<'a> {
    solver: &'a Solver,
    root: &'a Ty,
    metadata: &'a [Variable],
}

/// Whether some instantiation of the variables of `cover` with parts of
/// `subject` meets the bounds of `cover` and makes its root a subtype of
/// the root of `subject`.
fn find_cover(
    cover: CoverSide<'_>,
    subject: CoverSide<'_>,
    tag_base: u32,
    demand: &mut prove::Demand,
) -> Result<bool, ()> {
    let CoverSide {
        solver: cover,
        root: cover_root,
        metadata: cover_meta,
    } = cover;
    let CoverSide {
        solver: subject,
        root: subject_root,
        metadata: subject_meta,
    } = subject;
    if cover_meta
        .iter()
        .chain(subject_meta)
        .any(|v| v.depth.is_none() || (v.difference && v.handled == 0))
    {
        return Ok(false);
    }
    let mut candidates = vec![Ty::Never, Ty::Top];
    for ty in std::iter::once(subject_root).chain(
        subject
            .vars
            .iter()
            .flat_map(|s| s.lower.iter().chain(&s.upper)),
    ) {
        subterms(ty, &mut candidates);
    }
    // Closed pieces of the covering scheme and Boolean combinations of
    // subject atoms are valid instantiation expressions even when they do
    // not occur verbatim in the subject root.
    let mut closed = Vec::new();
    for ty in std::iter::once(cover_root).chain(
        cover
            .vars
            .iter()
            .flat_map(|s| s.lower.iter().chain(&s.upper)),
    ) {
        subterms(ty, &mut closed);
    }
    for candidate in closed {
        if variables(&candidate).is_empty() && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
    let atoms: Vec<_> = candidates
        .iter()
        .filter(|ty| match ty {
            Ty::Var(_) => true,
            Ty::Con(Head::Named(_), _, args) => args.is_empty(),
            _ => false,
        })
        .cloned()
        .collect();
    for i in 0..atoms.len() {
        for j in i + 1..atoms.len() {
            demand.charge()?;
            for candidate in [
                Ty::Union(vec![atoms[i].clone(), atoms[j].clone()]),
                Ty::Inter(vec![atoms[i].clone(), atoms[j].clone()]),
            ] {
                if !candidates.contains(&candidate) {
                    candidates.push(candidate);
                }
            }
        }
    }
    let subject_vars: Vec<_> = atoms
        .iter()
        .filter(|ty| matches!(ty, Ty::Var(_)))
        .cloned()
        .collect();
    if subject_vars.len() > 2 {
        for candidate in [Ty::Union(subject_vars.clone()), Ty::Inter(subject_vars)] {
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
    }
    let mut choices = Vec::with_capacity(cover.vars.len());
    for state in &cover.vars {
        if state.row != Mode::Value {
            let options = candidates
                .iter()
                .filter(|ty| row_candidate(subject, ty, state.row))
                .cloned()
                .collect::<Vec<_>>();
            if options.is_empty() {
                return Ok(false);
            }
            choices.push(options);
        } else {
            choices.push(candidates.clone());
        }
    }
    let count = cover.vars.len();
    let mut positions = vec![0usize; count];
    let mut depth = 0usize;
    loop {
        demand.charge()?;
        if depth == count {
            let substitute = |ty: &Ty| {
                super::alias::map_vars(ty, &mut |v| {
                    choices[v.0 as usize][positions[v.0 as usize]].clone()
                })
            };
            let mut valid = true;
            'vars: for (i, state) in cover.vars.iter().enumerate() {
                let value = &choices[i][positions[i]];
                for lower in &state.lower {
                    if !proves(subject, &substitute(lower), value, state.row, demand)? {
                        valid = false;
                        break 'vars;
                    }
                }
                for upper in &state.upper {
                    if !proves(subject, value, &substitute(upper), state.row, demand)? {
                        valid = false;
                        break 'vars;
                    }
                }
                for from in &state.from {
                    let from = &choices[from.0 as usize][positions[from.0 as usize]];
                    if !proves(subject, from, value, state.row, demand)? {
                        valid = false;
                        break 'vars;
                    }
                }
                for into in &state.into {
                    let into = &choices[into.0 as usize][positions[into.0 as usize]];
                    if !proves(subject, value, into, state.row, demand)? {
                        valid = false;
                        break 'vars;
                    }
                }
                for handler in &state.handled {
                    if !handler_entailed(subject, value, &substitute(handler), state.row, demand)? {
                        valid = false;
                        break 'vars;
                    }
                }
            }
            if valid {
                let target = substitute(cover_root);
                let mut solver = subject.scheme_snapshot();
                let mut tags = HashMap::new();
                let mut next = tag_base;
                let left = tag_scheme_type(&target, &mut tags, &mut next);
                let right = tag_scheme_type(subject_root, &mut tags, &mut next);
                for id in tags.values() {
                    solver.variance.insert(*id, vec![Variance::Co]);
                    solver.comparison_wrappers.insert(*id);
                }
                if proves(&solver, &left, &right, Mode::Value, demand)? {
                    return Ok(true);
                }
            }
            if depth == 0 {
                return Ok(false);
            }
            depth -= 1;
            positions[depth] += 1;
            continue;
        }
        if positions[depth] >= choices[depth].len() {
            positions[depth] = 0;
            if depth == 0 {
                return Ok(false);
            }
            depth -= 1;
            positions[depth] += 1;
        } else {
            depth += 1;
            if depth < count {
                positions[depth] = 0;
            }
        }
    }
}

/// A row candidate: only `Never`, row variables of this mode, and their
/// unions and intersections.
fn row_candidate(solver: &Solver, ty: &Ty, mode: Mode) -> bool {
    match ty {
        Ty::Never => true,
        Ty::Var(v) => solver.vars[v.0 as usize].row == mode,
        Ty::Union(xs) | Ty::Inter(xs) => xs.iter().all(|x| row_candidate(solver, x, mode)),
        _ => false,
    }
}

/// Prove a retained same-form handler for a candidate row without installing
/// it into the subject. Every union branch must retain the obligation.
fn handler_entailed(
    subject: &Solver,
    value: &Ty,
    handler: &Ty,
    mode: Mode,
    demand: &mut prove::Demand,
) -> Result<bool, ()> {
    Ok(match value {
        Ty::Never => true,
        Ty::Var(v) => {
            let state = &subject.vars[v.0 as usize];
            let mut found = false;
            for own in &state.handled {
                if proves(subject, own, handler, mode, demand)?
                    && proves(subject, handler, own, mode, demand)?
                {
                    found = true;
                    break;
                }
            }
            found
        }
        Ty::Con(..) | Ty::Fn(_) if value.value_head() != handler.value_head() => true,
        Ty::Con(..) | Ty::Fn(_) => {
            proves(subject, value, handler, mode, demand)?
                && proves(subject, handler, value, mode, demand)?
        }
        Ty::Union(xs) => {
            for x in xs {
                if !handler_entailed(subject, x, handler, mode, demand)? {
                    return Ok(false);
                }
            }
            true
        }
        Ty::Inter(xs) => {
            for x in xs {
                if handler_entailed(subject, x, handler, mode, demand)? {
                    return Ok(true);
                }
            }
            false
        }
        Ty::Diff(base, _) => handler_entailed(subject, base, handler, mode, demand)?,
        _ => false,
    })
}
