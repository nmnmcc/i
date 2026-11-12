//! Versioned type representation used by scheme interfaces. Equal parts are
//! written once; every edge refers to an earlier node.
use super::{FnTy, Head, ShapeId, Ty, V, lits::ValueSet};
use crate::hir::TypeRef;
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
    Data {
        positions: u32,
        labels: Vec<String>,
        args: Vec<usize>,
    },
    Function {
        params: Vec<(Option<String>, usize)>,
        ret: usize,
        err: usize,
        env: usize,
    },
    Union {
        members: Vec<usize>,
    },
    Inter {
        members: Vec<usize>,
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

struct Encoder<'a, N, W> {
    nodes: Vec<WireNode>,
    seen: FxHashMap<Ty, usize>,
    type_name: &'a mut N,
    variable: &'a mut W,
}

impl<N: FnMut(TypeRef) -> String, W: FnMut(V) -> String> Encoder<'_, N, W> {
    fn node(&mut self, t: &Ty) -> usize {
        if let Some(i) = self.seen.get(t) {
            return *i;
        }
        let all = |e: &mut Self, xs: &[Ty]| xs.iter().map(|x| e.node(x)).collect::<Vec<_>>();
        let node = match t {
            Ty::Never => WireNode::Never,
            Ty::Top => WireNode::Top,
            Ty::Var(v) => WireNode::Var {
                name: (self.variable)(*v),
            },
            Ty::Con(Head::Named(r), values, args) => {
                let listed = values.items().map(ToString::to_string).collect();
                let values = if values.is_all() {
                    WireValues::All
                } else if values.is_finite() {
                    WireValues::Only(listed)
                } else {
                    WireValues::Except(listed)
                };
                WireNode::Named {
                    name: (self.type_name)(*r),
                    values,
                    args: all(self, args),
                }
            }
            Ty::Con(Head::Shape(shape), _, args) => WireNode::Data {
                positions: shape.positions(),
                labels: shape.labels().to_vec(),
                args: all(self, args),
            },
            Ty::Con(Head::Fn, _, _) => unreachable!("a function type is Ty::Fn"),
            Ty::Fn(f) => WireNode::Function {
                params: f
                    .params
                    .iter()
                    .map(|(l, p)| (l.clone(), self.node(p)))
                    .collect(),
                ret: self.node(&f.ret),
                err: self.node(&f.err),
                env: self.node(&f.env),
            },
            Ty::Union(ms) => WireNode::Union {
                members: all(self, ms),
            },
            Ty::Inter(ms) => WireNode::Inter {
                members: all(self, ms),
            },
            Ty::Diff(base, heads) => WireNode::Diff {
                base: self.node(base),
                heads: all(self, heads),
            },
            Ty::Both(a, b) => WireNode::Both {
                a: self.node(a),
                b: self.node(b),
            },
        };
        self.nodes.push(node);
        let i = self.nodes.len() - 1;
        self.seen.insert(t.clone(), i);
        i
    }
}

/// Encode a type. `type_name` gives the stable identity of each named
/// type; `variable` numbers scheme variables.
pub(crate) fn encode(
    t: &Ty,
    type_name: &mut impl FnMut(TypeRef) -> String,
    variable: &mut impl FnMut(V) -> String,
) -> String {
    let mut e = Encoder {
        nodes: Vec::new(),
        seen: FxHashMap::default(),
        type_name,
        variable,
    };
    let root = e.node(t);
    serde_json::to_string(&Bundle {
        version: 2,
        root,
        nodes: e.nodes,
    })
    .expect("type is serializable")
}

/// Decode an untrusted bundle. The caller resolves named types and scheme
/// variables.
pub(crate) fn decode(
    text: &str,
    type_ref: &mut impl FnMut(&str) -> Option<TypeRef>,
    variable: &mut impl FnMut(&str) -> Option<V>,
) -> Result<Ty, String> {
    let bundle: Bundle = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if bundle.version != 2 || bundle.root >= bundle.nodes.len() {
        return Err("invalid type version or root".into());
    }
    let mut done: Vec<Ty> = Vec::with_capacity(bundle.nodes.len());
    for node in bundle.nodes {
        let get = |done: &Vec<Ty>, i: usize| {
            done.get(i)
                .cloned()
                .ok_or_else(|| "type edge is not an earlier node".to_string())
        };
        let all = |done: &Vec<Ty>, xs: &[usize]| {
            xs.iter()
                .map(|i| get(done, *i))
                .collect::<Result<Vec<_>, _>>()
        };
        let t = match node {
            WireNode::Never => Ty::Never,
            WireNode::Top => Ty::Top,
            WireNode::Var { name } => Ty::Var(variable(&name).ok_or("unknown type variable")?),
            WireNode::Named { name, values, args } => {
                let r = type_ref(&name).ok_or("unknown named type")?;
                let values = match values {
                    WireValues::All => ValueSet::ALL,
                    WireValues::Only(xs) => {
                        ValueSet::only(xs.into_iter().map(Arc::from)).ok_or("empty value set")?
                    }
                    WireValues::Except(xs) => ValueSet::except(xs.into_iter().map(Arc::from)),
                };
                Ty::Con(Head::Named(r), values, all(&done, &args)?)
            }
            WireNode::Data {
                positions,
                labels,
                args,
            } => {
                let mut sorted = labels.clone();
                sorted.sort();
                sorted.dedup();
                if sorted != labels || positions as usize + labels.len() != args.len() {
                    return Err("invalid data shape".into());
                }
                Ty::data(ShapeId::new(positions, labels), all(&done, &args)?)
            }
            WireNode::Function {
                params,
                ret,
                err,
                env,
            } => Ty::Fn(Box::new(FnTy {
                params: params
                    .into_iter()
                    .map(|(l, p)| Ok((l, get(&done, p)?)))
                    .collect::<Result<_, String>>()?,
                ret: get(&done, ret)?,
                err: get(&done, err)?,
                env: get(&done, env)?,
            })),
            WireNode::Union { members } => Ty::Union(all(&done, &members)?),
            WireNode::Inter { members } => Ty::Inter(all(&done, &members)?),
            WireNode::Diff { base, heads } => {
                Ty::Diff(Box::new(get(&done, base)?), all(&done, &heads)?)
            }
            WireNode::Both { a, b } => Ty::Both(Box::new(get(&done, a)?), Box::new(get(&done, b)?)),
        };
        done.push(t);
    }
    Ok(done.swap_remove(bundle.root))
}
