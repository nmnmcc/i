//! Exact graph output. Graph identity is an implementation identity, never a
//! test for set inequality. Full graph output is an interface artifact (§U6).

use super::{Graph, Label, Node};
use crate::{
    hir::TypeRef,
    types::{Head, ShownType, Ty, V},
};
use rustc_hash::FxHashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShownGraph {
    pub(crate) graph: Graph,
    pub(crate) names: Vec<(V, ShownType)>,
}

impl ShownGraph {
    /// Read a versioned graph bundle, resolving named types in its destination.
    pub fn from_bundle(
        text: &str,
        type_ref: &mut impl FnMut(&str) -> Option<TypeRef>,
    ) -> Result<ShownType, String> {
        let body = text
            .strip_prefix("type-graph-v1(")
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| "expected a type-graph-v1 bundle".to_string())?;
        let pool = super::Pool::default();
        let mut variables = FxHashMap::default();
        let mut names = Vec::new();
        let mut variable = |name: &str| {
            let id = *variables.entry(name.to_string()).or_insert_with(|| {
                let id = V(names.len() as u32);
                names.push(name.to_string());
                id
            });
            Some(id)
        };
        let ty = super::codec::decode(body, &pool, type_ref, &mut variable, &mut |_, _, _| {
            Some(super::graph::LabelOwner::artifact())
        })?;
        Ok(crate::types::display::tree_with(&ty, &mut |v| {
            ShownType::Var(names[v.0 as usize].clone())
        }))
    }

    pub(crate) fn new(graph: &Graph, name: &mut dyn FnMut(V) -> ShownType) -> Self {
        Self {
            graph: graph.clone(),
            names: graph
                .variables()
                .into_iter()
                .map(|v| (v, name(v)))
                .collect(),
        }
    }

    pub fn named_types(&self) -> Vec<TypeRef> {
        self.graph
            .nodes()
            .into_iter()
            .filter_map(|(_, node)| match node {
                Node::Con(Head::Named(t), _, _) => Some(t),
                _ => None,
            })
            .collect()
    }

    /// This is exact graph output, not the abbreviated diagnostic view.
    pub fn text_with(
        &self,
        ty: &mut dyn FnMut(TypeRef) -> String,
        var: &mut dyn FnMut(&str) -> String,
    ) -> String {
        if let Some(text) = self.inline_text(ty, var) {
            return text;
        }
        self.bundle_text(ty, var)
    }

    // The provisional record surface is isolated here. It is independent of
    // graph storage, the proof relation and the eventual graph wire decoder.
    fn inline_text(
        &self,
        ty: &mut dyn FnMut(TypeRef) -> String,
        var: &mut dyn FnMut(&str) -> String,
    ) -> Option<String> {
        let mut nodes = self.graph.nodes();
        nodes.sort_unstable_by_key(|(id, _)| *id);
        let mut costs = FxHashMap::default();
        for (id, node) in &nodes {
            if matches!(node, Node::Both(..)) {
                return None;
            }
            let cost = node
                .children()
                .into_iter()
                .fold(1usize, |n, child| n.saturating_add(costs[&child]).min(513));
            costs.insert(*id, cost);
        }
        if costs[&self.graph.id] > 512 {
            return None;
        }
        #[derive(Clone)]
        struct Text {
            body: String,
            atom: bool,
        }
        let mut texts: FxHashMap<super::Id, Text> = FxHashMap::default();
        let names: FxHashMap<_, _> = self
            .names
            .iter()
            .map(|(v, t)| (*v, t.text_with(ty, var)))
            .collect();
        for (id, node) in nodes {
            let child = |id| &texts[&id].body;
            let group = |id| {
                if texts[&id].atom {
                    texts[&id].body.clone()
                } else {
                    format!("{{ {} }}", texts[&id].body)
                }
            };
            let label = |l: &Label, var: &mut dyn FnMut(&str) -> String| match l {
                Label::Name(_) => l.text(),
                Label::Param(p) => var(&p.name),
            };
            let (body, atom) = match node {
                Node::Never => ("Never".into(), true),
                Node::Top => ("_".into(), true),
                Node::Var(v) => (names[&v].clone(), true),
                Node::Con(Head::Named(t), values, args) if args.is_empty() && !values.is_all() => {
                    let atom = crate::types::display::tree(
                        &Ty::Con(Head::Named(t), values, vec![]),
                        &mut |_| unreachable!("scalar"),
                    );
                    (atom.text_with(ty, var), false)
                }
                Node::Con(Head::Named(t), _, args) => {
                    let name = ty(t);
                    (
                        if args.is_empty() {
                            name
                        } else {
                            format!(
                                "{name}({})",
                                args.into_iter().map(group).collect::<Vec<_>>().join(", ")
                            )
                        },
                        true,
                    )
                }
                Node::Con(Head::Tuple(_), _, args) => (
                    format!(
                        "({})",
                        args.into_iter().map(group).collect::<Vec<_>>().join(", ")
                    ),
                    true,
                ),
                Node::Con(Head::Record | Head::Fn, _, _) => unreachable!("separate node forms"),
                Node::Fn(fun) => {
                    let params = fun
                        .params
                        .into_iter()
                        .map(|(label, id)| match label {
                            Some(label) => format!("{label}: {}", group(id)),
                            None => group(id),
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let mut body = format!("({params}) -> {}", group(fun.ret));
                    if fun.err != super::Id::NEVER {
                        body.push_str(&format!(" ? {}", group(fun.err)));
                    }
                    if fun.env != super::Id::NEVER {
                        body.push_str(&format!(" ! {}", group(fun.env)));
                    }
                    (body, false)
                }
                Node::Rec(fields) => (
                    if fields.is_empty() {
                        "Record".into()
                    } else {
                        format!(
                            "({})",
                            fields
                                .into_iter()
                                .map(|(l, id)| format!("{}: {}", label(&l, var), child(id)))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    },
                    true,
                ),
                Node::Union(a, b) => (format!("{} | {}", group(a), group(b)), false),
                Node::Inter(a, b) => (format!("{} & {}", group(a), group(b)), false),
                Node::Without(base, labels) => (
                    format!(
                        "(..{}, {})",
                        group(base),
                        labels
                            .iter()
                            .map(|l| format!("-{}", label(l, var)))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    true,
                ),
                Node::Diff(base, heads) => (
                    format!(
                        "{} - {}",
                        group(base),
                        heads.into_iter().map(group).collect::<Vec<_>>().join(" - ")
                    ),
                    false,
                ),
                Node::Both(..) => unreachable!("not an inline type"),
            };
            texts.insert(id, Text { body, atom });
        }
        Some(texts.remove(&self.graph.id).expect("root text").body)
    }

    fn bundle_text(
        &self,
        ty: &mut dyn FnMut(TypeRef) -> String,
        var: &mut dyn FnMut(&str) -> String,
    ) -> String {
        let names: FxHashMap<_, _> = self
            .names
            .iter()
            .map(|(v, t)| (*v, t.text_with(ty, var)))
            .collect();
        let wire = super::codec::encode(
            &self.graph,
            &mut |t| ty(t),
            &mut |v| names[&v].clone(),
            &mut |param| var(&param.name),
        );
        format!("type-graph-v1({wire})")
    }
}
