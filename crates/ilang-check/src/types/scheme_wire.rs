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
    /// Whether the scheme includes structural records or retained obligations
    /// whose meaning is not captured by its displayed type alone.
    pub fn needs_semantic_comparison(&self) -> bool {
        let solver = self.catalog.solver.lock().expect("scheme snapshot");
        let (vars, _, roots) = scheme_roots(&solver, &self.scheme);
        vars.iter().any(|v| {
            solver.vars[v.0 as usize].difference || !solver.vars[v.0 as usize].handled.is_empty()
        }) || roots.iter().any(|ty| {
            solver
                .records
                .graph(ty)
                .nodes()
                .iter()
                .any(|(_, node)| matches!(node, Node::Rec(_) | Node::Without(..)))
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

#[derive(Clone, Serialize, Deserialize)]
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
        if a == b {
            return Ok(Some(true));
        }
        let prove = |a: &Ty, b: &Ty| {
            records::prove(
                &solver,
                a,
                b,
                &mut records::Demand::new(records::Demand::LIMIT, true),
            )
        };
        match (prove(&a, &b), prove(&b, &a)) {
            (records::Verdict::Proven, records::Verdict::Proven) => Ok(Some(true)),
            (records::Verdict::Refuted, _) | (_, records::Verdict::Refuted) => Ok(Some(false)),
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

fn remap_scheme_type(ty: &Ty, vars: &[usize], labels: &[usize], names: &[String]) -> Ty {
    let ty = super::alias::map_vars(ty, &mut |v| Ty::Var(V(vars[v.0 as usize] as u32)));
    let remap = records::label_params(&ty)
        .into_iter()
        .map(|p| {
            let index = records::label_var(&p).expect("interface label binder").0 as usize;
            let target = labels[index];
            let replacement =
                records::variable_label(TyVarId(target as u32), &names[target], p.level);
            (p, replacement)
        })
        .collect();
    records::map_labels(&ty, &remap)
}

fn tag_scheme_type(
    pool: &records::Pool,
    ty: &Ty,
    tags: &mut HashMap<Vec<Option<String>>, crate::hir::TypeId>,
    next: &mut u32,
) -> Ty {
    pool.graph(ty).map_nodes(false, &mut |ty, _| {
        let Ty::Fn(f) = &ty else {
            return ty;
        };
        let labels = f
            .params
            .iter()
            .map(|(label, _)| label.clone())
            .collect::<Vec<_>>();
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
    let pool = solver.records.clone();
    let root = tag_scheme_type(&pool, root, tags, next);
    for state in &mut solver.vars {
        for ty in state
            .lower
            .iter_mut()
            .chain(&mut state.upper)
            .chain(&mut state.handled)
        {
            *ty = tag_scheme_type(&pool, ty, tags, next);
        }
    }
    root
}

fn proves(
    solver: &Solver,
    left: &Ty,
    right: &Ty,
    mode: Mode,
    demand: &mut records::Demand,
) -> Result<bool, ()> {
    match records::prove_mode(solver, left, right, mode, demand) {
        records::Verdict::Proven => Ok(true),
        records::Verdict::ResourceLimit => Err(()),
        records::Verdict::Refuted | records::Verdict::Unknown => Ok(false),
    }
}

// `subject` is interpreted only under its own constraints. Every condition of
// `target` must follow without adding a bound to either solver.
struct SchemeMapping<'a> {
    variables: &'a [usize],
    labels: &'a [usize],
    names: &'a [String],
}

fn entails_mapped(
    subject: &Solver,
    subject_root: &Ty,
    target: &Solver,
    target_root: &Ty,
    mapped: SchemeMapping<'_>,
    demand: &mut records::Demand,
) -> Result<bool, ()> {
    let SchemeMapping {
        variables: mapping,
        labels,
        names,
    } = mapped;
    for (i, wanted) in target.vars.iter().enumerate() {
        let actual = &subject.vars[mapping[i]];
        if actual.row != wanted.row
            || actual.difference != wanted.difference
            || actual.input != wanted.input
        {
            return Ok(false);
        }
        let label = wanted
            .label_key
            .map(|id| TyVarId(labels[id.0 as usize] as u32));
        if actual.label_key != label {
            return Ok(false);
        }
        let variable = Ty::Var(V(mapping[i] as u32));
        let mode = actual.row;
        for lower in &wanted.lower {
            let lower = remap_scheme_type(lower, mapping, labels, names);
            if !proves(subject, &lower, &variable, mode, demand)? {
                return Ok(false);
            }
        }
        for upper in &wanted.upper {
            let upper = remap_scheme_type(upper, mapping, labels, names);
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
            let handled = remap_scheme_type(handled, mapping, labels, names);
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
    let target_root = remap_scheme_type(target_root, mapping, labels, names);
    proves(subject, subject_root, &target_root, Mode::Value, demand)
}

// A closed service form supplies an explicit counterexample when the only
// remaining quantified row is otherwise unconstrained.
fn handler_witness(source: &Solver, source_root: &Ty, target: &Solver, target_root: &Ty) -> bool {
    if source.vars.len() != 1 || target.vars.len() != 1 {
        return false;
    }
    let sites = source.records.graph(source_root).variable_sites(
        source,
        super::solve::Pol::Pos,
        false,
        false,
    );
    let positive = sites.iter().any(|(v, pol, _)| {
        *v == V(0) && matches!(pol, super::solve::Pol::Pos | super::solve::Pol::Both)
    });
    let negative = sites.iter().any(|(v, pol, _)| {
        *v == V(0) && matches!(pol, super::solve::Pol::Neg | super::solve::Pol::Both)
    });
    if !positive || !negative {
        return false;
    }
    let mut demand = records::Demand::new(records::Demand::LIMIT, true);
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
        if !source.records.graph(form).variables().is_empty() {
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
                    | FailKind::Arity
                    | FailKind::Invariant
                    | FailKind::EnvForms
                    | FailKind::NoField
            )
        })
}

fn ground_label_scheme(solver: &Solver, root: &Ty, labels: &[usize]) -> Option<Ty> {
    if solver.vars.iter().any(|v| {
        v.label_key.is_none()
            || v.row != Mode::Value
            || v.difference
            || !v.lower.is_empty()
            || !v.upper.is_empty()
            || !v.from.is_empty()
            || !v.into.is_empty()
            || !v.handled.is_empty()
    }) {
        return None;
    }
    let key = |id: TyVarId| format!("interface-key-{}", labels[id.0 as usize]);
    let ty = super::alias::map_vars(root, &mut |v| {
        let id = solver.vars[v.0 as usize].label_key.expect("label key");
        Ty::literal(&super::LitValue::Str(key(id)))
    });
    let mapped = records::label_params(&ty)
        .into_iter()
        .map(|p| {
            let id = records::label_var(&p).expect("label binder");
            (p, Label::name(key(id)))
        })
        .collect();
    Some(records::map_labels(&ty, &mapped))
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
        if left_bundle.keys.len() != right_bundle.keys.len() {
            return Ok(SchemeComparison::Different);
        }
        let mut right_labels = vec![usize::MAX; right_bundle.labels.len()];
        let mut left_labels = vec![usize::MAX; left_bundle.labels.len()];
        for &(right_label, slot) in &right_bundle.keys {
            let Some(&(left_label, _)) = left_bundle.keys.iter().find(|(_, at)| *at == slot) else {
                return Ok(SchemeComparison::Different);
            };
            if (right_labels[right_label] != usize::MAX && right_labels[right_label] != left_label)
                || (left_labels[left_label] != usize::MAX && left_labels[left_label] != right_label)
            {
                return Ok(SchemeComparison::Different);
            }
            right_labels[right_label] = left_label;
            left_labels[left_label] = right_label;
        }
        if right_labels.contains(&usize::MAX) || left_labels.contains(&usize::MAX) {
            return Ok(SchemeComparison::Unknown);
        }
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
            let mut demand = records::Demand::new(records::Demand::LIMIT, true);
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
            let forward = find_cover(
                left,
                right,
                &left_labels,
                &right_bundle.labels,
                tag_base,
                &mut demand,
            )?;
            if !forward {
                return Ok(false);
            }
            find_cover(
                right,
                left,
                &right_labels,
                &left_bundle.labels,
                tag_base,
                &mut demand,
            )
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
        let identity_labels: Vec<_> = (0..left_labels.len()).collect();
        let witnesses = || {
            compare_witnesses(
                WitnessSide {
                    solver: &left_solver,
                    root: &left_root,
                    metadata: &left_bundle.variables,
                    labels: &identity_labels,
                },
                WitnessSide {
                    solver: &right_solver,
                    root: &right_root,
                    metadata: &right_bundle.variables,
                    labels: &right_labels,
                },
            )
        };
        if left_bundle.variables.len() != right_bundle.variables.len() {
            match cover() {
                Ok(true) => return Ok(SchemeComparison::Equivalent(vec![vec![]])),
                Err(()) => return Ok(SchemeComparison::ResourceLimit),
                Ok(false) => (),
            }
            let all_general = left_bundle
                .variables
                .iter()
                .chain(&right_bundle.variables)
                .all(|v| v.depth.is_some());
            if closed_environment || all_general {
                return Ok(witnesses());
            }
            return Ok(SchemeComparison::Unknown);
        }
        let count = left_bundle.variables.len();
        let mut candidates = Vec::with_capacity(count);
        for b in &right_bundle.variables {
            let mut matches = Vec::new();
            for (i, a) in left_bundle.variables.iter().enumerate() {
                let label = b.label.map(|n| right_labels[n]);
                if a.depth == b.depth
                    && a.external.is_some() == b.external.is_some()
                    && a.mode == b.mode
                    && a.difference == b.difference
                    && a.input == b.input
                    && a.label == label
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
        let mut demand = records::Demand::new(records::Demand::LIMIT, true);
        loop {
            let state: Vec<u64> = std::iter::once(u64::MAX)
                .chain(mapping[..depth].iter().map(|v| *v as u64))
                .collect();
            if demand.charge_state(&state).is_err() {
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
                        labels: &right_labels,
                        names: &left_bundle.labels,
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
                            labels: &left_labels,
                            names: &right_bundle.labels,
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
                    _ if left_bundle.variables.iter().all(|v| v.depth.is_some())
                        && right_bundle.variables.iter().all(|v| v.depth.is_some()) =>
                    {
                        let identity: Vec<_> = (0..left_labels.len()).collect();
                        if let (Some(a), Some(b)) = (
                            ground_label_scheme(&left_solver, &left_root, &identity),
                            ground_label_scheme(&right_solver, &right_root, &right_labels),
                        ) {
                            let mut forward = left_solver.clone();
                            let mut backward = right_solver.clone();
                            forward.constrain(&a, &b);
                            backward.constrain(&b, &a);
                            if definite_failure(&forward) || definite_failure(&backward) {
                                return Ok(SchemeComparison::Different);
                            }
                        }
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
        let all_general = left_bundle
            .variables
            .iter()
            .chain(&right_bundle.variables)
            .all(|v| v.depth.is_some());
        if solutions.is_empty() {
            match cover() {
                Ok(true) => return Ok(SchemeComparison::Equivalent(vec![vec![]])),
                Err(()) => return Ok(SchemeComparison::ResourceLimit),
                Ok(false) => (),
            }
            if closed_environment || all_general {
                Ok(witnesses())
            } else {
                Ok(SchemeComparison::Unknown)
            }
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
        let visible = solver.records.graph(root).variables();
        let candidate = (0..solver.vars.len()).find(|&i| {
            let v = V(i as u32);
            let st = &solver.vars[i];
            if visible.contains(&v)
                || (metadata[i].depth.is_none() && !closed_environment)
                || st.input
                || st.row != Mode::Value
                || st.difference
                || st.label_key.is_some()
                || !st.handled.is_empty()
            {
                return false;
            }
            for state in &solver.vars {
                for ty in state.lower.iter().chain(&state.upper).chain(&state.handled) {
                    if solver.records.graph(ty).variables().contains(&v)
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

fn aggregate_schemes(schemes: &[SchemeArtifact], stride: usize) -> Result<SchemeArtifact, String> {
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
    let mut keys = Vec::new();
    for (i, wire) in wires.iter().enumerate() {
        let artifact = environment.read(wire, &mut |name| identities.get(name).copied())?;
        for (id, slot) in artifact.scheme.key_slots {
            let slot = i
                .checked_mul(stride)
                .and_then(|base| base.checked_add(slot))
                .ok_or("interface key position overflow")?;
            keys.push((id, slot));
        }
        roots.push(artifact.scheme.ty);
    }
    let scheme = Scheme {
        level: 0,
        ty: Ty::tuple(roots),
        key_slots: keys,
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
    let max_slot = left
        .iter()
        .chain(right)
        .flat_map(|s| s.scheme.key_slots.iter().map(|(_, slot)| *slot))
        .max()
        .unwrap_or(0);
    let stride = max_slot
        .checked_add(1)
        .ok_or("interface key position overflow")?;
    let a = aggregate_schemes(left, stride)?;
    let b = aggregate_schemes(right, stride)?;
    a.compare_impl(&b, left_name, right_name, true)
}

// Construct concrete interface instances from input choices and the complete
// lower bounds of non-input variables. A failed proof is not a witness.
#[derive(Clone)]
enum WitnessChoice {
    Type(Ty),
    Upper,
    Lower,
}

fn ground_known_scheme(
    solver: &Solver,
    root: &Ty,
    metadata: &[Variable],
    mut choices: usize,
    domain: &[WitnessChoice],
    labels: &[usize],
    keys: &[String],
) -> Option<Ty> {
    if solver
        .vars
        .iter()
        .any(|state| state.difference && state.handled.is_empty())
    {
        return None;
    }
    let mut values = vec![None; solver.vars.len()];
    let mut use_upper = vec![false; solver.vars.len()];
    for (i, state) in solver.vars.iter().enumerate() {
        if let Some(label) = state.label_key {
            values[i] = Some(Ty::literal(&super::LitValue::Str(
                keys[labels[label.0 as usize]].clone(),
            )));
        } else if state.input || metadata[i].depth.is_some() {
            match &domain[choices % domain.len()] {
                WitnessChoice::Type(ty) => values[i] = Some(ty.clone()),
                WitnessChoice::Upper => use_upper[i] = true,
                WitnessChoice::Lower => (),
            }
            choices /= domain.len();
        }
    }
    let mut scratch = solver.scheme_snapshot();
    loop {
        let mut progress = false;
        for (i, state) in solver.vars.iter().enumerate() {
            if values[i].is_some() {
                continue;
            }
            let parts: Vec<_> = if use_upper[i] {
                state
                    .upper
                    .iter()
                    .cloned()
                    .chain(state.into.iter().copied().map(Ty::Var))
                    .collect()
            } else {
                state
                    .lower
                    .iter()
                    .cloned()
                    .chain(state.from.iter().copied().map(Ty::Var))
                    .collect()
            };
            if parts
                .iter()
                .flat_map(|t| solver.records.graph(t).variables())
                .any(|v| values[v.0 as usize].is_none())
            {
                continue;
            }
            let parts = parts
                .iter()
                .map(|ty| {
                    let ty = super::alias::map_vars(ty, &mut |v| {
                        values[v.0 as usize].clone().expect("ground dependency")
                    });
                    witness_labels(&ty, labels, keys)
                })
                .collect();
            values[i] = Some(if use_upper[i] {
                scratch.inter(parts)
            } else {
                scratch.union(parts)
            });
            progress = true;
        }
        if values.iter().all(Option::is_some) {
            break;
        }
        if !progress {
            return None;
        }
    }
    let ground = |ty: &Ty| {
        let ty = super::alias::map_vars(ty, &mut |v| {
            values[v.0 as usize].clone().expect("ground variable")
        });
        witness_labels(&ty, labels, keys)
    };
    for (i, state) in solver.vars.iter().enumerate() {
        let value = values[i].as_ref().expect("ground variable");
        if state.row != Mode::Value && !witness_row_value(solver, value, state.row) {
            return None;
        }
        for lower in &state.lower {
            witness_constraint(&mut scratch, &ground(lower), value, state.row);
        }
        for upper in &state.upper {
            witness_constraint(&mut scratch, value, &ground(upper), state.row);
        }
        for from in &state.from {
            witness_constraint(
                &mut scratch,
                values[from.0 as usize].as_ref().unwrap(),
                value,
                state.row,
            );
        }
        for into in &state.into {
            witness_constraint(
                &mut scratch,
                value,
                values[into.0 as usize].as_ref().unwrap(),
                state.row,
            );
        }
        for handler in &state.handled {
            let mut demand = records::Demand::new(records::Demand::LIMIT, true);
            if !handler_entailed(&scratch, value, &ground(handler), state.row, &mut demand).ok()? {
                return None;
            }
        }
    }
    scratch.failures.is_empty().then(|| ground(root))
}

fn witness_constraint(solver: &mut Solver, left: &Ty, right: &Ty, mode: Mode) {
    match mode {
        Mode::Value => solver.constrain(left, right),
        Mode::Row => solver.constrain_row(left, right),
        Mode::Env => solver.constrain_env(left, right),
    }
}

fn witness_row_value(solver: &Solver, value: &Ty, mode: Mode) -> bool {
    let graph = solver.records.graph(value);
    let mut pending = vec![graph.id];
    while let Some(id) = pending.pop() {
        match graph.node_at(id) {
            Node::Never | Node::Con(Head::Named(TypeRef::Custom(_)), _, _) => (),
            Node::Union(a, b) | Node::Inter(a, b) => pending.extend([a, b]),
            Node::Con(
                Head::Named(TypeRef::Builtin(
                    crate::hir::BuiltinType::Int | crate::hir::BuiltinType::String,
                )),
                _,
                _,
            ) if mode == Mode::Row => (),
            _ => return false,
        }
    }
    true
}

fn witness_labels(ty: &Ty, labels: &[usize], keys: &[String]) -> Ty {
    let mapping = records::label_params(ty)
        .into_iter()
        .map(|p| {
            let id = records::label_var(&p).expect("interface label binder").0 as usize;
            (p, Label::name(keys[labels[id]].as_str()))
        })
        .collect();
    records::map_labels(ty, &mapping)
}

fn shared_group_counterexample(
    source: &Solver,
    root: &Ty,
    metadata: &[Variable],
    target: &Solver,
    target_root: &Ty,
    source_labels: &[usize],
    target_labels: &[usize],
) -> Option<bool> {
    // In this fragment decomposition produces only conjunctive lower and
    // upper bounds on variables. No record/Boolean branch can require a
    // different choice, so an incompatible pair of closed bounds refutes
    // every instantiation of the target.
    let flat_target = target.vars.iter().all(|state| {
        state.row == Mode::Value
            && state.label_key.is_none()
            && !state.difference
            && state.handled.is_empty()
    }) && std::iter::once(target_root)
        .chain(
            target
                .vars
                .iter()
                .flat_map(|state| state.lower.iter().chain(&state.upper)),
        )
        .all(|ty| {
            target.records.graph(ty).nodes().iter().all(|(_, node)| {
                matches!(
                    node,
                    Node::Never | Node::Top | Node::Var(_) | Node::Con(..) | Node::Fn(_)
                )
            })
        });
    let inputs = source
        .vars
        .iter()
        .zip(metadata)
        .filter(|(state, meta)| state.label_key.is_none() && (state.input || meta.depth.is_some()))
        .count();
    let mut labels = std::collections::BTreeSet::new();
    for (solver, root) in [(source, root), (target, target_root)] {
        let roots = std::iter::once(root).chain(
            solver
                .vars
                .iter()
                .flat_map(|s| s.lower.iter().chain(&s.upper).chain(&s.handled)),
        );
        for ty in roots {
            for (_, node) in solver.records.graph(ty).nodes() {
                if let Node::Rec(fields) = node {
                    labels.extend(fields.into_iter().filter_map(|(label, _)| match label {
                        Label::Name(name) => Some(name.to_string()),
                        _ => None,
                    }));
                }
            }
        }
    }
    let count = source_labels
        .iter()
        .chain(target_labels)
        .copied()
        .max()
        .map_or(0, |n| n + 1);
    let mut nonce = 0usize;
    let keys = loop {
        let keys: Vec<_> = (0..count)
            .map(|i| format!("\0i-comparison-{nonce}-{i}"))
            .collect();
        if keys.iter().all(|key| !labels.contains(key)) {
            break keys;
        }
        nonce += 1;
    };
    let named_labels: Vec<_> = labels.iter().cloned().collect();
    let mut scenarios = vec![keys.clone()];
    for i in 0..keys.len() {
        for other in 0..i {
            let mut equal = keys.clone();
            equal[i] = equal[other].clone();
            if !scenarios.contains(&equal) {
                scenarios.push(equal);
            }
        }
        for label in &named_labels {
            let mut equal = keys.clone();
            equal[i] = label.clone();
            if !scenarios.contains(&equal) {
                scenarios.push(equal);
            }
        }
    }
    labels.extend(keys.iter().cloned());
    let integer = Ty::named(TypeRef::Builtin(crate::hir::BuiltinType::Int), Vec::new());
    let string = Ty::named(
        TypeRef::Builtin(crate::hir::BuiltinType::String),
        Vec::new(),
    );
    let mut domain = vec![
        WitnessChoice::Type(integer.clone()),
        WitnessChoice::Type(string.clone()),
        WitnessChoice::Type(Ty::Never),
        WitnessChoice::Type(Ty::Top),
        WitnessChoice::Type(source.records.record(Vec::new())),
        WitnessChoice::Upper,
        WitnessChoice::Lower,
    ];
    for solver in [source, target] {
        for handler in solver.vars.iter().flat_map(|state| &state.handled) {
            if !solver.records.graph(handler).variables().is_empty() {
                continue;
            }
            domain.push(WitnessChoice::Type(handler.clone()));
            let view = match handler {
                Ty::Graph(graph) => graph.view(),
                other => Some(other.clone()),
            };
            if let Some(Ty::Con(head @ Head::Named(TypeRef::Custom(_)), values, args)) = view {
                for i in 0..args.len() {
                    for replacement in [&integer, &string] {
                        let mut alternate = args.clone();
                        alternate[i] = replacement.clone();
                        domain.push(WitnessChoice::Type(Ty::Con(
                            head,
                            values.clone(),
                            alternate,
                        )));
                    }
                }
            }
        }
    }
    for name in &labels {
        for value in [&integer, &string] {
            domain.push(WitnessChoice::Type(
                source
                    .records
                    .record(vec![(Label::name(name.as_str()), value.clone())]),
            ));
        }
    }
    if !labels.is_empty() {
        for value in [&integer, &string] {
            domain.push(WitnessChoice::Type(
                source.records.record(
                    labels
                        .iter()
                        .map(|name| (Label::name(name.as_str()), value.clone()))
                        .collect(),
                ),
            ));
        }
    }
    let combinations = domain.len().checked_pow(u32::try_from(inputs).ok()?)?;
    let mut work = 0;
    for keys in scenarios {
        for choices in 0..combinations {
            if work == records::Demand::LIMIT {
                return None;
            }
            work += 1;
            let Some(ground) = ground_known_scheme(
                source,
                root,
                metadata,
                choices,
                &domain,
                source_labels,
                &keys,
            ) else {
                continue;
            };
            let mut forward = target.scheme_snapshot();
            for state in &mut forward.vars {
                for bound in state
                    .lower
                    .iter_mut()
                    .chain(&mut state.upper)
                    .chain(&mut state.handled)
                {
                    *bound = witness_labels(bound, target_labels, &keys);
                }
            }
            let target_root = witness_labels(target_root, target_labels, &keys);
            let fixed: Vec<_> = forward
                .vars
                .iter()
                .enumerate()
                .filter_map(|(i, state)| state.label_key.map(|label| (V(i as u32), label)))
                .collect();
            for (var, label) in fixed {
                let key = Ty::literal(&super::LitValue::Str(
                    keys[target_labels[label.0 as usize]].clone(),
                ));
                forward.constrain(&Ty::Var(var), &key);
                forward.constrain(&key, &Ty::Var(var));
            }
            if !forward.failures.is_empty() {
                continue;
            }
            let closed = forward.vars.iter().all(|state| state.label_key.is_some());
            let comparable = if closed {
                super::alias::map_vars(&target_root, &mut |v| {
                    let label = forward.vars[v.0 as usize]
                        .label_key
                        .expect("closed key parameter");
                    Ty::literal(&super::LitValue::Str(
                        keys[target_labels[label.0 as usize]].clone(),
                    ))
                })
            } else {
                target_root.clone()
            };
            forward.constrain(&comparable, &ground);
            let flat_ground = source
                .records
                .graph(&ground)
                .nodes()
                .iter()
                .all(|(_, node)| {
                    matches!(node, Node::Never | Node::Top | Node::Con(..) | Node::Fn(_))
                });
            if ((closed || (flat_target && flat_ground)) && definite_failure(&forward))
                || rejected_service_witness(target, &target_root, &ground)
            {
                return Some(true);
            }
        }
    }
    Some(false)
}

struct WitnessSide<'a> {
    solver: &'a Solver,
    root: &'a Ty,
    metadata: &'a [Variable],
    labels: &'a [usize],
}

fn compare_witnesses(left: WitnessSide<'_>, right: WitnessSide<'_>) -> SchemeComparison {
    let forward = shared_group_counterexample(
        left.solver,
        left.root,
        left.metadata,
        right.solver,
        right.root,
        left.labels,
        right.labels,
    );
    if forward == Some(true) {
        return SchemeComparison::Different;
    }
    let backward = shared_group_counterexample(
        right.solver,
        right.root,
        right.metadata,
        left.solver,
        left.root,
        right.labels,
        left.labels,
    );
    if backward == Some(true) {
        return SchemeComparison::Different;
    }
    if forward.is_none() || backward.is_none() {
        SchemeComparison::ResourceLimit
    } else {
        SchemeComparison::Unknown
    }
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
            solver.vars[i].label_key.is_some()
                || solver.vars[i].row != solver.vars[first].row
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
                || state.label_key.is_some()
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
                        && !solver.records.graph(ty).variables().contains(&v)
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

/// Find an explicit, universally valid instantiation of `cover` in terms of
/// the subject's binders. The subject's constraints are never strengthened.
#[derive(Clone, Copy)]
struct CoverSide<'a> {
    solver: &'a Solver,
    root: &'a Ty,
    metadata: &'a [Variable],
}

fn find_cover(
    cover: CoverSide<'_>,
    subject: CoverSide<'_>,
    label_map: &[usize],
    label_names: &[String],
    tag_base: u32,
    demand: &mut records::Demand,
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
    let roots = std::iter::once(subject_root).chain(
        subject
            .vars
            .iter()
            .flat_map(|s| s.lower.iter().chain(&s.upper)),
    );
    for ty in roots {
        let graph = subject.records.graph(ty);
        for (id, _) in graph.nodes() {
            let candidate = graph.pool().ty(id);
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
    }
    // Closed pieces of the covering scheme and Boolean combinations of
    // subject atoms are valid instantiation expressions even when they do
    // not occur verbatim in the subject root.
    let cover_roots = std::iter::once(cover_root).chain(
        cover
            .vars
            .iter()
            .flat_map(|s| s.lower.iter().chain(&s.upper)),
    );
    for ty in cover_roots {
        let graph = cover.records.graph(ty);
        for (id, _) in graph.nodes() {
            let candidate = graph.pool().ty(id);
            if graph.pool().graph(&candidate).variables().is_empty()
                && records::label_params(&candidate).is_empty()
                && !candidates.contains(&candidate)
            {
                candidates.push(candidate);
            }
        }
    }
    let atoms: Vec<_> = candidates
        .iter()
        .filter(|ty| match ty {
            Ty::Var(_) => true,
            Ty::Con(Head::Named(_), _, args) => args.is_empty(),
            _ => matches!(subject.records.graph(ty).node(), Node::Rec(_)),
        })
        .cloned()
        .collect();
    let combine_scope = demand.new_scope();
    for i in 0..atoms.len() {
        for j in i + 1..atoms.len() {
            demand
                .charge_state(&[u64::MAX - 2, combine_scope, i as u64, j as u64])
                .map_err(|_| ())?;
            for candidate in [
                subject.records.union(&[atoms[i].clone(), atoms[j].clone()]),
                subject.records.inter(&[atoms[i].clone(), atoms[j].clone()]),
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
        for candidate in [
            subject.records.union(&subject_vars),
            subject.records.inter(&subject_vars),
        ] {
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
    }
    let mut choices = Vec::with_capacity(cover.vars.len());
    for state in &cover.vars {
        if let Some(label) = state.label_key {
            let target = TyVarId(label_map[label.0 as usize] as u32);
            let options = subject
                .vars
                .iter()
                .enumerate()
                .filter_map(|(i, s)| (s.label_key == Some(target)).then_some(Ty::Var(V(i as u32))))
                .collect::<Vec<_>>();
            if options.is_empty() {
                return Ok(false);
            }
            choices.push(options);
        } else if state.row != Mode::Value {
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
    let scope = demand.new_scope();
    loop {
        let state: Vec<u64> = [u64::MAX - 1, scope]
            .into_iter()
            .chain(positions[..depth].iter().map(|v| *v as u64))
            .collect();
        demand.charge_state(&state).map_err(|_| ())?;
        if depth == count {
            let substitute = |ty: &Ty| {
                let labels = records::label_params(ty)
                    .into_iter()
                    .map(|p| {
                        let id = records::label_var(&p).expect("interface label binder").0 as usize;
                        let target = label_map[id];
                        let value = records::variable_label(
                            TyVarId(target as u32),
                            &label_names[target],
                            p.level,
                        );
                        (p, value)
                    })
                    .collect();
                let ty = records::map_labels(ty, &labels);
                super::alias::map_vars(&ty, &mut |v| {
                    choices[v.0 as usize][positions[v.0 as usize]].clone()
                })
            };
            let mut valid = true;
            for (i, state) in cover.vars.iter().enumerate() {
                let value = &choices[i][positions[i]];
                for lower in &state.lower {
                    if !proves(subject, &substitute(lower), value, state.row, demand)? {
                        valid = false;
                        break;
                    }
                }
                if !valid {
                    break;
                }
                for upper in &state.upper {
                    if !proves(subject, value, &substitute(upper), state.row, demand)? {
                        valid = false;
                        break;
                    }
                }
                if !valid {
                    break;
                }
                for from in &state.from {
                    if !proves(
                        subject,
                        &choices[from.0 as usize][positions[from.0 as usize]],
                        value,
                        state.row,
                        demand,
                    )? {
                        valid = false;
                        break;
                    }
                }
                if !valid {
                    break;
                }
                for into in &state.into {
                    if !proves(
                        subject,
                        value,
                        &choices[into.0 as usize][positions[into.0 as usize]],
                        state.row,
                        demand,
                    )? {
                        valid = false;
                        break;
                    }
                }
                if !valid {
                    break;
                }
                for handler in &state.handled {
                    if !handler_entailed(subject, value, &substitute(handler), state.row, demand)? {
                        valid = false;
                        break;
                    }
                }
                if !valid {
                    break;
                }
            }
            if valid {
                let target = substitute(cover_root);
                let mut solver = subject.scheme_snapshot();
                let mut tags = HashMap::new();
                let mut next = tag_base;
                let left = tag_scheme_type(&solver.records, &target, &mut tags, &mut next);
                let right = tag_scheme_type(&solver.records, subject_root, &mut tags, &mut next);
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

fn row_candidate(solver: &Solver, ty: &Ty, mode: Mode) -> bool {
    let graph = solver.records.graph(ty);
    graph.nodes().into_iter().all(|(_, node)| match node {
        Node::Never => true,
        Node::Var(v) => solver.vars[v.0 as usize].row == mode,
        Node::Union(..) | Node::Inter(..) => true,
        _ => false,
    })
}

/// Prove a retained same-form handler for a candidate row without installing
/// it into the subject. Every union branch must retain the obligation.
fn handler_entailed(
    subject: &Solver,
    value: &Ty,
    handler: &Ty,
    mode: Mode,
    demand: &mut records::Demand,
) -> Result<bool, ()> {
    let graph = subject.records.graph(value);
    let mut nodes = graph.nodes();
    nodes.sort_unstable_by_key(|(id, _)| *id);
    let mut proven = FxHashMap::default();
    for (id, node) in nodes {
        let yes = match node {
            Node::Never => true,
            Node::Var(v) => {
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
            Node::Con(head, values, args) => {
                if Some(head) != handler.value_head() {
                    true
                } else {
                    let ty = subject
                        .records
                        .ty(subject.records.intern(Node::Con(head, values, args)));
                    proves(subject, &ty, handler, mode, demand)?
                        && proves(subject, handler, &ty, mode, demand)?
                }
            }
            Node::Union(a, b) => proven[&a] && proven[&b],
            Node::Inter(a, b) => proven[&a] || proven[&b],
            Node::Diff(base, _) => proven[&base],
            _ => false,
        };
        proven.insert(id, yes);
    }
    Ok(proven[&graph.id])
}

/// A concrete service can distinguish two correlated environment schemes if
/// every target row binder rejects that form, and the same row is observable
/// in both input and output positions.
fn rejected_service_witness(target: &Solver, target_root: &Ty, ground: &Ty) -> bool {
    use super::solve::Pol;
    if target.vars.is_empty()
        || target
            .vars
            .iter()
            .any(|v| v.row != Mode::Env || v.handled.is_empty())
    {
        return false;
    }
    let graph = target.records.graph(ground);
    for (id, node) in graph.nodes() {
        let Node::Con(Head::Named(TypeRef::Custom(ctor)), _, _) = node else {
            continue;
        };
        if target.comparison_wrappers.contains(&ctor) {
            continue;
        }
        let form = target.records.ty(id);
        let mut all_reject = true;
        for i in 0..target.vars.len() {
            let mut trial = target.scheme_snapshot();
            trial.constrain_env(&form, &Ty::Var(V(i as u32)));
            if !definite_failure(&trial) {
                all_reject = false;
                break;
            }
        }
        if !all_reject {
            continue;
        }
        let mut proof = target.scheme_snapshot();
        let marker = proof.fresh_at(0);
        proof.vars[marker.0 as usize].row = Mode::Env;
        proof.vars[marker.0 as usize].input = true;
        let target_shape = super::alias::map_vars(target_root, &mut |_| Ty::Var(marker));
        let form_id = proof.records.import(&form);
        let source_shape = proof.records.graph(ground).map_nodes(false, &mut |ty, _| {
            if proof.records.import(&ty) == form_id {
                Ty::Var(marker)
            } else {
                ty
            }
        });
        let sites =
            proof
                .records
                .graph(&target_shape)
                .variable_sites(&proof, Pol::Pos, false, false);
        let positive = sites
            .iter()
            .any(|(v, p, _)| *v == marker && matches!(p, Pol::Pos | Pol::Both));
        let negative = sites
            .iter()
            .any(|(v, p, _)| *v == marker && matches!(p, Pol::Neg | Pol::Both));
        if !positive || !negative {
            continue;
        }
        let mut demand = records::Demand::new(records::Demand::LIMIT, true);
        if matches!(
            proves(
                &proof,
                &target_shape,
                &source_shape,
                Mode::Value,
                &mut demand
            ),
            Ok(true)
        ) && matches!(
            proves(
                &proof,
                &source_shape,
                &target_shape,
                Mode::Value,
                &mut demand
            ),
            Ok(true)
        ) {
            return true;
        }
    }
    false
}
