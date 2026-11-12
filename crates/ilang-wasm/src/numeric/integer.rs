//! Exact signed-i31 entry regions. General Int never uses int_to_i64.
use super::rewrite;
use crate::ir::*;
use std::collections::{BTreeMap, BTreeSet};

/// Follow parameter aliases to integer operations. This chooses a guarded
/// encoding, not a type fact: every selected argument is tested at the edge.
pub fn parameters(f: &Func) -> Vec<bool> {
    let mut defs = BTreeMap::<Local, Vec<Atom>>::new();
    let mut body = f.body.clone();
    rewrite::walk(&mut body, &mut |s| {
        if let Stmt::Let { dst, value, .. } = s {
            defs.entry(*dst).or_default().push(match value {
                Value::Atom(a) => a.clone(),
                _ => Atom::Nil,
            });
        }
    });
    let mut roots = (0..f.arity).map(|l| (l, l)).collect::<BTreeMap<_, _>>();
    loop {
        let before = roots.len();
        for (&dst, values) in &defs {
            if values.len() == 1
                && let Atom::Local(source) = &values[0]
                && let Some(&root) = roots.get(source)
            {
                roots.insert(dst, root);
            }
        }
        if roots.len() == before {
            break;
        }
    }
    let mut used = BTreeSet::new();
    rewrite::walk(&mut body, &mut |s| {
        if let Stmt::Let {
            value:
                Value::Op(
                    Op::IntAdd
                    | Op::IntSub
                    | Op::IntMul
                    | Op::IntDiv
                    | Op::IntRem
                    | Op::IntNeg
                    | Op::IntLt
                    | Op::IntLe
                    | Op::IntGt
                    | Op::IntGe,
                    args,
                ),
            ..
        } = s
        {
            for a in args {
                if let Atom::Local(l) = a
                    && let Some(&root) = roots.get(l)
                {
                    used.insert(root);
                }
            }
        }
    });
    (0..f.arity).map(|l| used.contains(&l)).collect()
}
