//! Exact record operations (records.md §§20–21).
//!
//! The pure kernel is tested before production type conversion is connected.

mod bdd;
mod bridge;
pub(super) mod codec;
mod graph;
mod proof;
#[cfg(test)]
mod relations;
mod shown;

pub use shown::ShownGraph;

pub(crate) use bdd::{Demand, ResourceLimit};
pub(crate) use bridge::{Field, Graph, Pool};
pub(crate) use graph::LabelParam;
pub(crate) use graph::{Id, Label, Node};
pub(crate) use proof::{Verdict, prove, prove_mode};

pub(crate) fn written_label(label: &crate::hir::RecordLabel, level: u32) -> Label {
    match label.parameter {
        Some(var) => variable_label(var, &label.name, level),
        None => Label::name(label.name.as_str()),
    }
}

pub(crate) fn variable_label(var: crate::hir::TyVarId, name: &str, level: u32) -> Label {
    Label::Param(LabelParam {
        owner: graph::LabelOwner::Artifact((1u64 << 63) | u64::from(var.0)),
        index: 0,
        level,
        name: name.into(),
    })
}

pub(crate) fn label_var(param: &LabelParam) -> Option<crate::hir::TyVarId> {
    match param.owner {
        graph::LabelOwner::Artifact(n) if n & (1u64 << 63) != 0 => {
            Some(crate::hir::TyVarId(n as u32))
        }
        _ => None,
    }
}

pub(crate) fn label_params(ty: &super::Ty) -> rustc_hash::FxHashSet<LabelParam> {
    use super::Ty;
    let mut result = rustc_hash::FxHashSet::default();
    let mut todo = vec![ty];
    while let Some(ty) = todo.pop() {
        match ty {
            Ty::Graph(g) => result.extend(g.label_params()),
            Ty::Con(_, _, args) | Ty::Union(args) | Ty::Inter(args) => todo.extend(args),
            Ty::Fn(f) => {
                todo.extend(f.params.iter().map(|(_, t)| t));
                todo.extend([&f.ret, &f.err, &f.env]);
            }
            Ty::Diff(base, heads) => {
                todo.push(base);
                todo.extend(heads);
            }
            Ty::Both(a, b) => {
                todo.push(a);
                todo.push(b);
            }
            Ty::Var(_) | Ty::Never | Ty::Top => {}
        }
    }
    result
}

/// Substitute label parameters throughout a type while preserving graph sharing.
pub(crate) fn map_labels(
    ty: &super::Ty,
    labels: &rustc_hash::FxHashMap<LabelParam, Label>,
) -> super::Ty {
    use super::{FnTy, Ty};
    match ty {
        Ty::Graph(g) => g.map_binders(&mut |v| Ty::Var(v), labels),
        Ty::Con(h, values, args) => Ty::Con(
            *h,
            values.clone(),
            args.iter().map(|t| map_labels(t, labels)).collect(),
        ),
        Ty::Fn(f) => Ty::Fn(Box::new(FnTy {
            params: f
                .params
                .iter()
                .map(|(n, t)| (n.clone(), map_labels(t, labels)))
                .collect(),
            ret: map_labels(&f.ret, labels),
            err: map_labels(&f.err, labels),
            env: map_labels(&f.env, labels),
        })),
        Ty::Union(ts) => Ty::Union(ts.iter().map(|t| map_labels(t, labels)).collect()),
        Ty::Inter(ts) => Ty::Inter(ts.iter().map(|t| map_labels(t, labels)).collect()),
        Ty::Diff(base, heads) => Ty::Diff(
            Box::new(map_labels(base, labels)),
            heads.iter().map(|t| map_labels(t, labels)).collect(),
        ),
        Ty::Both(a, b) => Ty::Both(
            Box::new(map_labels(a, labels)),
            Box::new(map_labels(b, labels)),
        ),
        Ty::Var(_) | Ty::Never | Ty::Top => ty.clone(),
    }
}
