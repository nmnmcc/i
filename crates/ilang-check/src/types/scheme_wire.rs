//! Shared-graph scheme bundles retain bounds and scoped label identities.
use super::{
    Head, Ty, V, Variance,
    infer::Scheme,
    lits::ValueSet,
    records::{self, Label, Node},
    solve::{Mode, Solver},
};
use crate::hir::{TyVarId, TypeRef};
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
    next_label: u32,
}

impl Default for SchemeEnvironment {
    fn default() -> Self {
        Self {
            solver: Solver::new(HashMap::new(), Vec::new()),
            externals: FxHashMap::default(),
            next_label: 0,
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
        let mut next_label = self.next_label;
        let scheme = decode(
            text,
            &mut self.solver,
            0,
            type_ref,
            &mut |_| {
                let id = TyVarId(next_label);
                next_label = next_label.checked_add(1).expect("label binder overflow");
                id
            },
            &mut |index, solver| {
                let id = wire.variables.get(index)?.external?;
                Some(*externals.entry(id).or_insert_with(|| solver.fresh_at(0)))
            },
        )?;
        self.externals = externals;
        self.next_label = next_label;
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
    labels: Vec<String>,
    keys: Vec<(usize, usize)>,
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

#[derive(Serialize, Deserialize)]
struct Variable {
    depth: Option<u32>,
    external: Option<usize>,
    name: Option<String>,
    label: Option<usize>,
    mode: u8,
    difference: bool,
    input: bool,
    lower: usize,
    upper: usize,
    handled: usize,
    from: Vec<usize>,
    into: Vec<usize>,
}

fn add_label(labels: &mut Vec<TyVarId>, names: &mut Vec<String>, id: TyVarId, name: &str) {
    if let Some(i) = labels.iter().position(|label| *label == id) {
        if names[i].is_empty() {
            names[i] = name.to_string();
        }
    } else {
        labels.push(id);
        names.push(name.to_string());
    }
}

fn scheme_roots(s: &Solver, scheme: &Scheme) -> (Vec<V>, FxHashMap<V, usize>, Vec<Ty>) {
    let mut vars = Vec::new();
    let mut index = FxHashMap::default();
    let mut roots = vec![scheme.ty.clone()];
    let mut cursor = 0;
    let mut processed = 0;
    while cursor < roots.len() {
        let graph = s.records.graph(&roots[cursor]);
        for v in graph.variables() {
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
    let mut labels: Vec<TyVarId> = Vec::new();
    let mut label_names = Vec::new();
    for (id, _) in &scheme.key_slots {
        add_label(&mut labels, &mut label_names, *id, "");
    }
    for v in &vars {
        let st = &s.vars[v.0 as usize];
        if let Some(id) = st.label_key {
            add_label(
                &mut labels,
                &mut label_names,
                id,
                st.name.as_deref().unwrap_or(""),
            );
        }
    }
    for root in &roots {
        for param in records::label_params(root) {
            let id = records::label_var(&param).expect("scheme label has a binding identity");
            add_label(&mut labels, &mut label_names, id, &param.name);
        }
    }
    let label_index = |id: TyVarId| {
        labels
            .iter()
            .position(|v| *v == id)
            .expect("scheme label recorded")
    };
    let mut metadata = Vec::new();
    for v in &vars {
        let st = &s.vars[v.0 as usize];
        metadata.push(Variable {
            depth: (st.level > scheme.level).then(|| st.level - scheme.level),
            external: (st.level <= scheme.level).then(|| external_ids[v]),
            name: st.name.clone(),
            label: st.label_key.map(label_index),
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
    let mut envelope = Ty::tuple(roots);
    let relative = records::label_params(&envelope)
        .into_iter()
        .map(|p| {
            let mut relative = p.clone();
            relative.level = p.level.saturating_sub(scheme.level);
            (p, Label::Param(relative))
        })
        .collect();
    envelope = records::map_labels(&envelope, &relative);
    let graph = s.records.graph(&envelope);
    let mut nominal_ids = Vec::new();
    for (_, node) in graph.nodes() {
        if let Node::Con(Head::Named(TypeRef::Custom(id)), _, _) = node
            && !nominal_ids.contains(&id)
        {
            nominal_ids.push(id);
        }
    }
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
    let wire = records::codec::encode(
        &graph,
        type_name,
        &mut |v| index[&v].to_string(),
        &mut |p| {
            format!(
                "{}:{}",
                label_index(records::label_var(p).expect("label binder")),
                p.name
            )
        },
    );
    serde_json::to_string(&Bundle {
        version: 2,
        graph: wire,
        variables: metadata,
        labels: label_names,
        nominals,
        keys: scheme
            .key_slots
            .iter()
            .map(|(id, n)| (label_index(*id), *n))
            .collect(),
    })
    .expect("scheme serialization")
}

/// Restore a scheme with one allocation map for the visible type, its bounds,
/// and its labels. The external-variable callback owns cross-scheme sharing.
pub(super) fn decode(
    text: &str,
    solver: &mut Solver,
    base: u32,
    type_ref: &mut impl FnMut(&str) -> Option<TypeRef>,
    fresh_label: &mut impl FnMut(usize) -> TyVarId,
    external: &mut impl FnMut(usize, &mut Solver) -> Option<V>,
) -> Result<Scheme, String> {
    let bundle: Bundle = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if bundle.version != 2 {
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
    let labels: Vec<_> = (0..bundle.labels.len()).map(fresh_label).collect();
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
    let pool = s.records.clone();
    let mut scopes = FxHashMap::default();
    let graph = records::codec::decode(
        &bundle.graph,
        &pool,
        type_ref,
        &mut |name| {
            name.parse::<usize>()
                .ok()
                .and_then(|i| vars.get(i).copied())
        },
        &mut |scope, _, name| {
            let index = name.split_once(':')?.0.parse::<usize>().ok()?;
            let id = *labels.get(index)?;
            if scopes.values().any(|&other| other == index) {
                return None;
            }
            scopes.insert(scope, index);
            match records::variable_label(id, "", 0) {
                Label::Param(p) => Some(p.owner),
                _ => unreachable!(),
            }
        },
    )?;
    let mut remap = FxHashMap::default();
    for p in records::label_params(&graph) {
        let (index, name) = p.name.split_once(':').ok_or("invalid encoded label")?;
        let index: usize = index.parse().map_err(|_| "invalid encoded label index")?;
        let id = *labels.get(index).ok_or("unknown encoded label")?;
        if records::label_var(&p) != Some(id) {
            return Err("label scope disagrees with binder".into());
        }
        if bundle.labels[index] != name {
            return Err("label name disagrees with binder".into());
        }
        let level = base.checked_add(p.level).ok_or("label level overflow")?;
        let label = records::variable_label(id, name, level);
        remap.insert(p, label);
    }
    let graph = records::map_labels(&graph, &remap);
    let Ty::Graph(graph) = graph else {
        return Err("scheme graph is not an envelope".into());
    };
    let Node::Con(Head::Tuple(_), _, roots) = graph.node() else {
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
            let Node::Con(Head::Tuple(_), values, items) = graph.node_at(roots[1 + i * 3 + j])
            else {
                return Err("scheme bounds are not a tuple".into());
            };
            if values != ValueSet::ALL || items.len() != count {
                return Err("invalid scheme bounds".into());
            }
            parts.push(items.into_iter().map(|id| pool.ty(id)).collect::<Vec<_>>());
        }
        bounds.push(parts);
    }
    for (i, meta) in bundle.variables.iter().enumerate() {
        let mode = read_mode(meta.mode)?;
        let label = meta
            .label
            .map(|n| labels.get(n).copied().ok_or("unknown key binder"))
            .transpose()?;
        let edges = |ids: &[usize]| {
            ids.iter()
                .map(|id| vars.get(*id).copied().ok_or("unknown variable edge"))
                .collect::<Result<Vec<_>, _>>()
        };
        let from = edges(&meta.from)?;
        let into = edges(&meta.into)?;
        let st = &mut s.vars[vars[i].0 as usize];
        st.name = meta.name.clone();
        st.label_key = label;
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
    let keys = bundle
        .keys
        .iter()
        .map(|(i, slot)| {
            labels
                .get(*i)
                .copied()
                .map(|id| (id, *slot))
                .ok_or("unknown label key".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let ty = pool.ty(roots[0]);
    *solver = s;
    Ok(Scheme {
        level: base,
        ty,
        key_slots: keys,
    })
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
    pub fn equivalent(
        &self,
        other: &Self,
        left_name: &mut impl FnMut(TypeRef) -> String,
        right_name: &mut impl FnMut(TypeRef) -> String,
    ) -> Result<bool, String> {
        if self.template.params != other.template.params
            || self.template.slots != other.template.slots
            || self.modes != other.modes
        {
            return Ok(false);
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
            let graph = source.records.graph(&artifact.template.ty);
            let mut names = HashMap::new();
            for (_, node) in graph.nodes() {
                let Node::Con(Head::Named(ty), _, _) = node else {
                    continue;
                };
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
            let wire = records::codec::encode(
                &graph,
                &mut |ty| match ty {
                    TypeRef::Builtin(_) => format!("builtin:{}", name(ty)),
                    TypeRef::Custom(_) => format!("nominal:{}", name(ty)),
                },
                &mut |v| v.0.to_string(),
                &mut |_| unreachable!("alias label parameter"),
            );
            records::codec::decode(
                &wire,
                &solver.records,
                &mut |name| names.get(name).copied(),
                &mut |name| {
                    name.parse::<u32>()
                        .ok()
                        .filter(|&i| (i as usize) < artifact.template.slots)
                        .map(V)
                },
                &mut |_, _, _| None,
            )
        };
        let a = import(self, &left, left_name)?;
        let b = import(other, &right, right_name)?;
        // The subtype relation intentionally ignores argument labels, while
        // the package interface must preserve named-call availability. Wrap
        // each function in a disjoint tagged wrapper encoding its labels.
        // The ordinary proof then compares both the function type
        // and its call protocol, including inside records and unions.
        let graph_a = solver.records.graph(&a);
        let graph_b = solver.records.graph(&b);
        let mut tags = HashMap::<Vec<Option<String>>, crate::hir::TypeId>::new();
        let mut tag = |ty: Ty, _: bool| {
            let Ty::Fn(function) = &ty else {
                return ty;
            };
            let labels = function
                .params
                .iter()
                .map(|(label, _)| label.clone())
                .collect::<Vec<_>>();
            let id = *tags.entry(labels).or_insert_with(|| {
                let id = crate::hir::TypeId(next_id);
                next_id += 1;
                solver.variance.insert(id, vec![Variance::Co]);
                solver.comparison_wrappers.insert(id);
                id
            });
            Ty::named(TypeRef::Custom(id), vec![ty])
        };
        let a = graph_a.map_nodes(false, &mut tag);
        let b = graph_b.map_nodes(false, &mut tag);
        let prove = |a: &Ty, b: &Ty| {
            records::prove(
                &solver,
                a,
                b,
                &mut records::Demand::new(records::Demand::LIMIT, true),
            )
        };
        match (prove(&a, &b), prove(&b, &a)) {
            (records::Verdict::Proven, records::Verdict::Proven) => Ok(true),
            (records::Verdict::Refuted, _) | (_, records::Verdict::Refuted) => Ok(false),
            (records::Verdict::ResourceLimit, _) | (_, records::Verdict::ResourceLimit) => {
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
                    if matches!(prove(&a, &b), records::Verdict::Refuted)
                        || matches!(prove(&b, &a), records::Verdict::Refuted)
                    {
                        return Ok(false);
                    }
                }
                Err("alias equivalence could not be proven".into())
            }
            _ => Err("alias equivalence could not be proven".into()),
        }
    }
}
