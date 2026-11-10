//! Versioned, shared type-graph representation used by scheme interfaces.
//! Every edge refers to an earlier node; labels keep explicit scope identities.
use super::graph::{Function, LabelOwner};
use super::{Graph, Id, Label, LabelParam, Node, Pool};
use crate::hir::{BuiltinType, TypeRef};
use crate::types::{Head, Ty, V, lits::ValueSet};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
struct Bundle {
    version: u32,
    root: usize,
    nodes: Vec<WireNode>,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
enum WireNode {
    Never,
    Top,
    Var {
        name: String,
    },
    Named {
        name: String,
        values: WireValues,
        args: Vec<usize>,
    },
    Tuple {
        args: Vec<usize>,
    },
    Function {
        params: Vec<(Option<String>, usize)>,
        ret: usize,
        err: usize,
        env: usize,
    },
    Record {
        fields: Vec<(WireLabel, usize)>,
    },
    Union {
        a: usize,
        b: usize,
    },
    Inter {
        a: usize,
        b: usize,
    },
    Without {
        base: usize,
        labels: Vec<WireLabel>,
    },
    Diff {
        base: usize,
        heads: Vec<usize>,
    },
    Both {
        a: usize,
        b: usize,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", content = "values")]
enum WireValues {
    All,
    Only(Vec<String>),
    Except(Vec<String>),
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
enum WireLabel {
    Name {
        name: String,
    },
    Param {
        scope: usize,
        index: u32,
        level: u32,
        name: String,
    },
}

/// Encode a graph without expanding shared nodes. `type_name` gives the stable
/// identity of each named type; `variable` numbers scheme variables.
pub(crate) fn encode(
    graph: &Graph,
    type_name: &mut impl FnMut(TypeRef) -> String,
    variable: &mut impl FnMut(V) -> String,
    label_name: &mut impl FnMut(&LabelParam) -> String,
) -> String {
    let mut nodes = graph.nodes();
    nodes.sort_unstable_by_key(|(id, _)| *id);
    let indices: FxHashMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i))
        .collect();
    let mut owners = FxHashMap::default();
    let mut label = |label: Label| match label {
        Label::Name(name) => WireLabel::Name {
            name: name.to_string(),
        },
        Label::Param(p) => {
            let next = owners.len();
            let scope = *owners.entry(p.owner).or_insert(next);
            WireLabel::Param {
                scope,
                index: p.index,
                level: p.level,
                name: label_name(&p),
            }
        }
    };
    let nodes = nodes
        .into_iter()
        .map(|(_, node)| {
            let at = |id: Id| indices[&id];
            match node {
                Node::Never => WireNode::Never,
                Node::Top => WireNode::Top,
                Node::Var(v) => WireNode::Var { name: variable(v) },
                Node::Con(Head::Named(t), values, args) => {
                    let listed = values.items().map(ToString::to_string).collect();
                    let values = if values.is_all() {
                        WireValues::All
                    } else if values.is_finite() {
                        WireValues::Only(listed)
                    } else {
                        WireValues::Except(listed)
                    };
                    WireNode::Named {
                        name: type_name(t),
                        values,
                        args: args.into_iter().map(at).collect(),
                    }
                }
                Node::Con(Head::Tuple(_), _, args) => WireNode::Tuple {
                    args: args.into_iter().map(at).collect(),
                },
                Node::Con(Head::Fn | Head::Record, _, _) => {
                    unreachable!("function and record nodes are separate")
                }
                Node::Fn(f) => WireNode::Function {
                    params: f
                        .params
                        .into_iter()
                        .map(|(n, t)| (n.map(|n| n.to_string()), at(t)))
                        .collect(),
                    ret: at(f.ret),
                    err: at(f.err),
                    env: at(f.env),
                },
                Node::Rec(fields) => WireNode::Record {
                    fields: fields.into_iter().map(|(l, t)| (label(l), at(t))).collect(),
                },
                Node::Union(a, b) => WireNode::Union { a: at(a), b: at(b) },
                Node::Inter(a, b) => WireNode::Inter { a: at(a), b: at(b) },
                Node::Without(base, labels) => WireNode::Without {
                    base: at(base),
                    labels: labels.into_iter().map(&mut label).collect(),
                },
                Node::Diff(base, heads) => WireNode::Diff {
                    base: at(base),
                    heads: heads.into_iter().map(at).collect(),
                },
                Node::Both(a, b) => WireNode::Both { a: at(a), b: at(b) },
            }
        })
        .collect();
    serde_json::to_string(&Bundle {
        version: 1,
        root: indices[&graph.id],
        nodes,
    })
    .expect("type graph is serializable")
}

/// Decode an untrusted bundle. The caller resolves named types and allocates
/// scheme variables and label scopes once, preserving their identities.
pub(crate) fn decode(
    text: &str,
    pool: &Pool,
    type_ref: &mut impl FnMut(&str) -> Option<TypeRef>,
    variable: &mut impl FnMut(&str) -> Option<V>,
    scope_owner: &mut impl FnMut(usize, u32, &str) -> Option<LabelOwner>,
) -> Result<Ty, String> {
    let bundle: Bundle = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if bundle.version != 1 || bundle.root >= bundle.nodes.len() {
        return Err("invalid type graph version or root".into());
    }
    let mut owners: FxHashMap<usize, LabelOwner> = FxHashMap::default();
    let mut ids: Vec<Id> = Vec::with_capacity(bundle.nodes.len());
    for node in bundle.nodes {
        let get = |index: usize| {
            ids.get(index)
                .copied()
                .ok_or_else(|| "type graph edge is not an earlier node".to_string())
        };
        let mut label = |l: WireLabel| -> Result<Label, String> {
            Ok(match l {
                WireLabel::Name { name } => Label::name(name),
                WireLabel::Param {
                    scope,
                    index,
                    level,
                    name,
                } => {
                    let owner = match owners.get(&scope) {
                        Some(owner) => *owner,
                        None => {
                            let owner = scope_owner(scope, index, &name)
                                .ok_or_else(|| "unknown label scope".to_string())?;
                            owners.insert(scope, owner);
                            owner
                        }
                    };
                    Label::Param(LabelParam {
                        owner,
                        index,
                        level,
                        name: Arc::from(name),
                    })
                }
            })
        };
        let node = match node {
            WireNode::Never => Node::Never,
            WireNode::Top => Node::Top,
            WireNode::Var { name } => {
                Node::Var(variable(&name).ok_or_else(|| "unknown scheme variable".to_string())?)
            }
            WireNode::Named { name, values, args } => {
                let ty = type_ref(&name).ok_or_else(|| format!("unknown named type {name}"))?;
                let values = match values {
                    WireValues::All => ValueSet::ALL,
                    WireValues::Only(v) => ValueSet::only(v.into_iter().map(Arc::from))
                        .ok_or_else(|| "empty scalar value set".to_string())?,
                    WireValues::Except(v) => ValueSet::except(v.into_iter().map(Arc::from)),
                };
                if !values.is_all()
                    && !matches!(ty, TypeRef::Builtin(BuiltinType::Int | BuiltinType::String))
                {
                    return Err("value set on a non-scalar type".into());
                }
                Node::Con(
                    Head::Named(ty),
                    values,
                    args.into_iter().map(get).collect::<Result<_, _>>()?,
                )
            }
            WireNode::Tuple { args } => {
                let args = args.into_iter().map(get).collect::<Result<Vec<_>, _>>()?;
                Node::Con(Head::Tuple(args.len() as u32), ValueSet::ALL, args)
            }
            WireNode::Function {
                params,
                ret,
                err,
                env,
            } => Node::Fn(Function {
                params: params
                    .into_iter()
                    .map(|(n, t)| Ok((n.map(Arc::from), get(t)?)))
                    .collect::<Result<_, String>>()?,
                ret: get(ret)?,
                err: get(err)?,
                env: get(env)?,
            }),
            WireNode::Record { fields } => {
                let mut fields = fields
                    .into_iter()
                    .map(|(l, t)| Ok((label(l)?, get(t)?)))
                    .collect::<Result<Vec<_>, String>>()?;
                fields.sort_by(|a, b| a.0.cmp(&b.0));
                if fields.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                    return Err("duplicate record label in type graph".into());
                }
                Node::Rec(fields)
            }
            WireNode::Union { a, b } => Node::Union(get(a)?, get(b)?),
            WireNode::Inter { a, b } => Node::Inter(get(a)?, get(b)?),
            WireNode::Without { base, labels } => {
                let mut labels: Vec<_> = labels
                    .into_iter()
                    .map(&mut label)
                    .collect::<Result<_, _>>()?;
                labels.sort();
                if labels.windows(2).any(|pair| pair[0] == pair[1]) {
                    return Err("duplicate removed label in type graph".into());
                }
                Node::Without(get(base)?, labels)
            }
            WireNode::Diff { base, heads } => Node::Diff(
                get(base)?,
                heads.into_iter().map(get).collect::<Result<_, _>>()?,
            ),
            WireNode::Both { a, b } => Node::Both(get(a)?, get(b)?),
        };
        ids.push(pool.intern(node));
    }
    Ok(pool.ty(ids[bundle.root]))
}

#[cfg(test)]
mod tests {
    use super::super::bridge::Field;
    use super::*;

    #[test]
    fn graph_round_trip_preserves_sharing_fields_and_scoped_labels() {
        let source = Pool::default();
        let label = Label::Param(LabelParam {
            owner: LabelOwner::artifact(),
            index: 0,
            level: 1,
            name: Arc::from("K"),
        });
        let record = source.record(vec![
            (Label::name("a"), Ty::Var(V(0))),
            (label.clone(), Ty::Var(V(0))),
        ]);
        let record_graph = match &record {
            Ty::Graph(g) => g,
            _ => panic!("record graph"),
        };
        let bytes = encode(
            record_graph,
            &mut |t| match t {
                TypeRef::Builtin(b) => b.name().into(),
                TypeRef::Custom(_) => panic!("no custom types"),
            },
            &mut |v| v.0.to_string(),
            &mut |param| param.name.to_string(),
        );
        let destination = Pool::default();
        let decoded = decode(
            &bytes,
            &destination,
            &mut |name| {
                BuiltinType::ALL
                    .into_iter()
                    .find(|b| b.name() == name)
                    .map(TypeRef::Builtin)
            },
            &mut |v: &str| v.parse().ok().map(V),
            &mut |_, _, _| Some(LabelOwner::artifact()),
        )
        .unwrap();
        let Ty::Graph(decoded) = decoded else {
            panic!("decoded record graph");
        };
        let fields = decoded.fields().expect("record fields");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].1, Ty::Var(V(0)));
        assert_eq!(fields[1].1, Ty::Var(V(0)));
        let Label::Param(decoded_label) = &fields[1].0 else {
            panic!("scoped label");
        };
        let Label::Param(original_label) = label else {
            unreachable!()
        };
        assert_ne!(decoded_label.owner, original_label.owner);
        assert_eq!(decoded_label.name, original_label.name);
        assert!(matches!(
            decoded.field(&Label::name("a")),
            Field::Found(Ty::Var(V(0)))
        ));
    }

    #[test]
    fn graph_decode_rejects_invalid_edges_and_versions() {
        let pool = Pool::default();
        let mut ty = |_: &str| None;
        let mut var = |_: &str| Some(V(0));
        assert!(
            decode(
                r#"{"version":2,"root":0,"nodes":[{"kind":"Top"}]}"#,
                &pool,
                &mut ty,
                &mut var,
                &mut |_, _, _| Some(LabelOwner::artifact())
            )
            .is_err()
        );
        assert!(
            decode(
                r#"{"version":1,"root":0,"nodes":[{"kind":"Union","a":0,"b":0}]}"#,
                &pool,
                &mut ty,
                &mut var,
                &mut |_, _, _| Some(LabelOwner::artifact())
            )
            .is_err()
        );
    }

    #[test]
    fn shared_branch_family_round_trips_without_expansion() {
        let pool = Pool::default();
        let mut root = pool.record(Vec::new());
        for i in 0..20 {
            let a = pool.record(vec![(Label::name(format!("a{i}")), Ty::Var(V(0)))]);
            let b = pool.record(vec![(Label::name(format!("b{i}")), Ty::Var(V(0)))]);
            root = pool.union(&[pool.inter(&[root.clone(), a]), pool.inter(&[root, b])]);
        }
        let Ty::Graph(graph) = root else {
            panic!("shared graph");
        };
        let wire = encode(
            &graph,
            &mut |t| format!("{t:?}"),
            &mut |v| v.0.to_string(),
            &mut |param| param.name.to_string(),
        );
        assert!(
            wire.len() < 20_000,
            "the wire format must preserve shared nodes"
        );
        let destination = Pool::default();
        let decoded = decode(
            &wire,
            &destination,
            &mut |_: &str| None,
            &mut |v: &str| v.parse().ok().map(V),
            &mut |_, _, _| Some(LabelOwner::artifact()),
        )
        .unwrap();
        let Ty::Graph(decoded) = decoded else {
            panic!("decoded graph");
        };
        assert_eq!(
            wire,
            encode(
                &decoded,
                &mut |t| format!("{t:?}"),
                &mut |v| v.0.to_string(),
                &mut |param| param.name.to_string()
            )
        );
        let shown = super::super::shown::ShownGraph::new(&graph, &mut |v| {
            crate::types::ShownType::Var(format!("A{}", v.0))
        });
        let rendered = shown.text_with(&mut |t| format!("{t:?}"), &mut |v| v.to_string());
        assert!(rendered.starts_with("type-graph-v1("));
        let restored =
            super::super::shown::ShownGraph::from_bundle(&rendered, &mut |_: &str| None).unwrap();
        assert_eq!(
            rendered,
            restored.text_with(&mut |t| format!("{t:?}"), &mut |v| v.to_string())
        );
    }
}
