//! Stack-safe structural operations on inference types.
//!
//! Types contain owned trees, while solver variables identify shared graph
//! nodes. These operations copy or compare the tree without following those
//! variable edges. In particular, a variable retains its identity.

use super::{FnTy, Head, Ty, lits::ValueSet};
use std::hash::{Hash, Hasher};

impl std::fmt::Debug for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        enum Part<'a> {
            Type(&'a Ty),
            Text(&'static str),
            Label(&'a Option<String>),
        }
        fn list<'a>(stack: &mut Vec<Part<'a>>, xs: &'a [Ty]) {
            for (i, x) in xs.iter().enumerate().rev() {
                stack.push(Part::Type(x));
                if i != 0 { stack.push(Part::Text(", ")); }
            }
        }
        let mut pending = vec![Part::Type(self)];
        while let Some(part) = pending.pop() {
            match part {
                Part::Text(s) => f.write_str(s)?,
                Part::Label(label) => write!(f, "{label:?}")?,
                Part::Type(t) => match t {
                    Ty::Var(v) => write!(f, "Var({v:?})")?,
                    Ty::Never => f.write_str("Never")?,
                    Ty::Top => f.write_str("Top")?,
                    Ty::Con(h, values, args) => {
                        write!(f, "Con({h:?}, {values:?}, [")?;
                        pending.push(Part::Text("])"));
                        list(&mut pending, args);
                    }
                    Ty::Fn(fun) => {
                        f.write_str("Fn(FnTy { params: [")?;
                        pending.push(Part::Text(" })"));
                        pending.push(Part::Type(&fun.env));
                        pending.push(Part::Text(", env: "));
                        pending.push(Part::Type(&fun.err));
                        pending.push(Part::Text(", err: "));
                        pending.push(Part::Type(&fun.ret));
                        pending.push(Part::Text("], ret: "));
                        for (i, (label, t)) in fun.params.iter().enumerate().rev() {
                            pending.push(Part::Text(")"));
                            pending.push(Part::Type(t));
                            pending.push(Part::Text(", "));
                            pending.push(Part::Label(label));
                            pending.push(Part::Text("("));
                            if i != 0 { pending.push(Part::Text(", ")); }
                        }
                    }
                    Ty::Union(xs) | Ty::Inter(xs) => {
                        f.write_str(if matches!(t, Ty::Union(_)) { "Union([" } else { "Inter([" })?;
                        pending.push(Part::Text("])"));
                        list(&mut pending, xs);
                    }
                    Ty::Diff(a, hs) => {
                        f.write_str("Diff(")?;
                        pending.push(Part::Text("])"));
                        list(&mut pending, hs);
                        pending.push(Part::Text(", ["));
                        pending.push(Part::Type(a));
                    }
                    Ty::Both(a, b) => {
                        f.write_str("Both(")?;
                        pending.push(Part::Text(")"));
                        pending.push(Part::Type(b));
                        pending.push(Part::Text(", "));
                        pending.push(Part::Type(a));
                    }
                },
            }
        }
        Ok(())
    }
}

/// Visit the owned type tree in source order without following solver edges.
pub(super) fn visit(t: &Ty, mut f: impl FnMut(&Ty)) {
    let mut pending = vec![t];
    while let Some(t) = pending.pop() {
        f(t);
        match t {
            Ty::Var(_) | Ty::Never | Ty::Top => {}
            Ty::Con(_, _, xs) | Ty::Union(xs) | Ty::Inter(xs) => pending.extend(xs.iter().rev()),
            Ty::Fn(fun) => {
                pending.extend([&fun.env, &fun.err, &fun.ret]);
                pending.extend(fun.params.iter().rev().map(|(_, t)| t));
            }
            Ty::Diff(a, xs) => {
                pending.extend(xs.iter().rev());
                pending.push(a);
            }
            Ty::Both(a, b) => {
                pending.push(b);
                pending.push(a);
            }
        }
    }
}

enum CopyTask<'a> {
    Visit(&'a Ty),
    Con(Head, ValueSet, usize),
    Fn(Vec<Option<String>>),
    Union(usize),
    Inter(usize),
    Diff(usize),
    Both,
}

fn take_children(results: &mut Vec<Ty>, n: usize) -> Vec<Ty> {
    let start = results
        .len()
        .checked_sub(n)
        .expect("type copy stack underflow");
    results.split_off(start)
}

impl Clone for Ty {
    fn clone(&self) -> Self {
        match self {
            Ty::Var(v) => return Ty::Var(*v),
            Ty::Never => return Ty::Never,
            Ty::Top => return Ty::Top,
            Ty::Con(h, values, args) if args.is_empty() => {
                return Ty::Con(*h, values.clone(), Vec::new());
            }
            _ => {}
        }
        let mut tasks = vec![CopyTask::Visit(self)];
        let mut results = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                CopyTask::Visit(t) => match t {
                    Ty::Var(v) => results.push(Ty::Var(*v)),
                    Ty::Never => results.push(Ty::Never),
                    Ty::Top => results.push(Ty::Top),
                    Ty::Con(h, values, args) => {
                        tasks.push(CopyTask::Con(*h, values.clone(), args.len()));
                        tasks.extend(args.iter().rev().map(CopyTask::Visit));
                    }
                    Ty::Fn(f) => {
                        tasks.push(CopyTask::Fn(
                            f.params.iter().map(|(n, _)| n.clone()).collect(),
                        ));
                        tasks.push(CopyTask::Visit(&f.env));
                        tasks.push(CopyTask::Visit(&f.err));
                        tasks.push(CopyTask::Visit(&f.ret));
                        tasks.extend(f.params.iter().rev().map(|(_, t)| CopyTask::Visit(t)));
                    }
                    Ty::Union(xs) | Ty::Inter(xs) => {
                        tasks.push(if matches!(t, Ty::Union(_)) {
                            CopyTask::Union(xs.len())
                        } else {
                            CopyTask::Inter(xs.len())
                        });
                        tasks.extend(xs.iter().rev().map(CopyTask::Visit));
                    }
                    Ty::Diff(base, rest) => {
                        tasks.push(CopyTask::Diff(rest.len()));
                        tasks.extend(rest.iter().rev().map(CopyTask::Visit));
                        tasks.push(CopyTask::Visit(base));
                    }
                    Ty::Both(a, b) => {
                        tasks.push(CopyTask::Both);
                        tasks.push(CopyTask::Visit(b));
                        tasks.push(CopyTask::Visit(a));
                    }
                },
                CopyTask::Con(h, v, n) => {
                    let children = take_children(&mut results, n);
                    results.push(Ty::Con(h, v, children));
                }
                CopyTask::Union(n) => {
                    let children = take_children(&mut results, n);
                    results.push(Ty::Union(children));
                }
                CopyTask::Inter(n) => {
                    let children = take_children(&mut results, n);
                    results.push(Ty::Inter(children));
                }
                CopyTask::Diff(n) => {
                    let mut parts = take_children(&mut results, n + 1).into_iter();
                    let base = parts.next().expect("type difference base");
                    results.push(Ty::Diff(Box::new(base), parts.collect()));
                }
                CopyTask::Both => {
                    let mut parts = take_children(&mut results, 2).into_iter();
                    results.push(Ty::Both(
                        Box::new(parts.next().unwrap()),
                        Box::new(parts.next().unwrap()),
                    ));
                }
                CopyTask::Fn(labels) => {
                    let n = labels.len();
                    let mut parts = take_children(&mut results, n + 3).into_iter();
                    let params = labels
                        .into_iter()
                        .map(|label| (label, parts.next().unwrap()))
                        .collect();
                    results.push(Ty::Fn(Box::new(FnTy {
                        params,
                        ret: parts.next().unwrap(),
                        err: parts.next().unwrap(),
                        env: parts.next().unwrap(),
                    })));
                }
            }
        }
        assert_eq!(results.len(), 1, "type copy stack must contain one result");
        results.pop().unwrap()
    }
}

impl PartialEq for Ty {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Ty::Var(a), Ty::Var(b)) => return a == b,
            (Ty::Never, Ty::Never) | (Ty::Top, Ty::Top) => return true,
            (Ty::Con(a, av, ax), Ty::Con(b, bv, bx)) if ax.is_empty() && bx.is_empty() => {
                return a == b && av == bv;
            }
            _ => {}
        }
        let mut pending = vec![(self, other)];
        while let Some((a, b)) = pending.pop() {
            match (a, b) {
                (Ty::Var(x), Ty::Var(y)) if x == y => {}
                (Ty::Never, Ty::Never) | (Ty::Top, Ty::Top) => {}
                (Ty::Con(x, xv, xa), Ty::Con(y, yv, ya))
                    if x == y && xv == yv && xa.len() == ya.len() =>
                {
                    pending.extend(xa.iter().zip(ya))
                }
                (Ty::Union(xs), Ty::Union(ys)) | (Ty::Inter(xs), Ty::Inter(ys))
                    if xs.len() == ys.len() =>
                {
                    pending.extend(xs.iter().zip(ys))
                }
                (Ty::Diff(x, xs), Ty::Diff(y, ys)) if xs.len() == ys.len() => {
                    pending.push((x, y));
                    pending.extend(xs.iter().zip(ys));
                }
                (Ty::Both(a, b), Ty::Both(c, d)) => {
                    pending.push((a, c));
                    pending.push((b, d));
                }
                (Ty::Fn(x), Ty::Fn(y)) if x.params.len() == y.params.len() => {
                    for ((xl, xt), (yl, yt)) in x.params.iter().zip(&y.params) {
                        if xl != yl {
                            return false;
                        }
                        pending.push((xt, yt));
                    }
                    pending.extend([(&x.ret, &y.ret), (&x.err, &y.err), (&x.env, &y.env)]);
                }
                _ => return false,
            }
        }
        true
    }
}

impl Eq for Ty {}

impl Hash for Ty {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Ty::Var(v) => {
                std::mem::discriminant(self).hash(state);
                v.hash(state);
                return;
            }
            Ty::Never | Ty::Top => {
                std::mem::discriminant(self).hash(state);
                return;
            }
            Ty::Con(h, values, args) if args.is_empty() => {
                std::mem::discriminant(self).hash(state);
                h.hash(state);
                values.hash(state);
                args.len().hash(state);
                return;
            }
            _ => {}
        }
        let mut pending = vec![self];
        while let Some(t) = pending.pop() {
            std::mem::discriminant(t).hash(state);
            match t {
                Ty::Var(v) => v.hash(state),
                Ty::Never | Ty::Top => {}
                Ty::Con(h, values, args) => {
                    h.hash(state);
                    values.hash(state);
                    args.len().hash(state);
                    pending.extend(args.iter().rev());
                }
                Ty::Union(xs) | Ty::Inter(xs) => {
                    xs.len().hash(state);
                    pending.extend(xs.iter().rev());
                }
                Ty::Diff(base, rest) => {
                    rest.len().hash(state);
                    pending.extend(rest.iter().rev());
                    pending.push(base);
                }
                Ty::Both(a, b) => {
                    pending.push(b);
                    pending.push(a);
                }
                Ty::Fn(f) => {
                    f.params.len().hash(state);
                    for (label, _) in &f.params {
                        label.hash(state);
                    }
                    pending.push(&f.env);
                    pending.push(&f.err);
                    pending.push(&f.ret);
                    pending.extend(f.params.iter().rev().map(|(_, t)| t));
                }
            }
        }
    }
}

impl Drop for Ty {
    fn drop(&mut self) {
        fn push(t: Ty, pending: &mut Vec<Ty>) {
            if !matches!(t, Ty::Never) {
                pending.push(t);
            }
        }
        fn detach(t: &mut Ty, pending: &mut Vec<Ty>) {
            match t {
                Ty::Con(_, _, xs) | Ty::Union(xs) | Ty::Inter(xs) => pending.append(xs),
                Ty::Fn(f) => {
                    pending.extend(std::mem::take(&mut f.params).into_iter().map(|(_, t)| t));
                    push(std::mem::replace(&mut f.ret, Ty::Never), pending);
                    push(std::mem::replace(&mut f.err, Ty::Never), pending);
                    push(std::mem::replace(&mut f.env, Ty::Never), pending);
                }
                Ty::Diff(base, xs) => {
                    pending.append(xs);
                    push(std::mem::replace(base.as_mut(), Ty::Never), pending);
                }
                Ty::Both(a, b) => {
                    push(std::mem::replace(a.as_mut(), Ty::Never), pending);
                    push(std::mem::replace(b.as_mut(), Ty::Never), pending);
                }
                Ty::Var(_) | Ty::Never | Ty::Top => {}
            }
        }
        let mut pending = Vec::new();
        detach(self, &mut pending);
        while let Some(mut node) = pending.pop() {
            detach(&mut node, &mut pending);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{BuiltinType, TypeRef};
    use crate::types::{
        V,
        solve::{Mode, Solver},
    };
    use std::collections::hash_map::DefaultHasher;

    fn deep(depth: usize, leaf: Ty) -> Ty {
        let mut t = leaf;
        for i in 0..depth {
            t = match i % 6 {
                0 => Ty::named(TypeRef::Builtin(BuiltinType::List), vec![t]),
                1 => Ty::Fn(Box::new(FnTy {
                    params: vec![(Some("p".into()), Ty::Var(V(0)))],
                    ret: t,
                    err: Ty::Never,
                    env: Ty::Never,
                })),
                2 => Ty::Union(vec![t, Ty::Never]),
                3 => Ty::Inter(vec![t, Ty::Top]),
                4 => Ty::Diff(Box::new(t), vec![Ty::Never]),
                _ => Ty::Both(Box::new(t), Box::new(Ty::Var(V(0)))),
            };
        }
        t
    }

    fn hash(t: &Ty) -> u64 {
        let mut h = DefaultHasher::new();
        t.hash(&mut h);
        h.finish()
    }

    #[test]
    fn owned_type_operations_keep_deep_structure_and_variable_identity() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let t = deep(10_000, Ty::Var(V(0)));
                let copied = t.clone();
                assert!(t == copied);
                assert_eq!(hash(&t), hash(&copied));
                let different = deep(10_000, Ty::Var(V(1)));
                assert!(t != different);
            let text = format!("{t:?}");
            assert!(text.starts_with("Inter([") && text.contains("Var(V(0))"));
                let mut visited = 0;
                visit(&t, |_| visited += 1);
                assert!(visited > 10_000);
                drop((t, copied, different));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn debug_keeps_the_structural_form() {
        let t = Ty::Fn(Box::new(FnTy {
            params: vec![(Some("p".into()), Ty::Var(V(0)))],
            ret: Ty::Never,
            err: Ty::Never,
            env: Ty::Never,
        }));
        assert_eq!(format!("{t:?}"), "Fn(FnTy { params: [(Some(\"p\"), Var(V(0)))], ret: Never, err: Never, env: Never })");
    }

    #[test]
    fn solver_finds_deep_levels_and_row_modes() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut s = Solver::new(Default::default(), Vec::new());
                let v = s.fresh_at(7);
                let t = deep(10_000, Ty::Var(v));
                assert_eq!(s.level_of(&t), 7);
                let mut row = Ty::Var(v);
                s.mark_mode(&row, Mode::Row);
                for _ in 0..10_000 {
                    row = Ty::Diff(Box::new(row), Vec::new());
                }
                assert!(s.is_row(&row));
                assert!(row.is_varlike());
                drop((t, row));
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn freshen_deep_scheme_keeps_shared_variables_and_bounds() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut s = Solver::new(Default::default(), Vec::new());
                let general = s.fresh_at(3);
                let weak = s.fresh_at(0);
                let int = Ty::named(TypeRef::Builtin(BuiltinType::Int), vec![]);
                s.constrain(&int, &Ty::Var(general));
                let mut t = Ty::tuple(vec![Ty::Var(general), Ty::Var(weak), Ty::Var(general)]);
                for _ in 0..10_000 {
                    t = Ty::named(TypeRef::Builtin(BuiltinType::List), vec![t]);
                }
                s.level = 2;
                let mut map = Default::default();
                let fresh = s.freshen(&t, 0, &mut map);
                let new = map[&general];
                assert_ne!(new, general);
                assert!(!map.contains_key(&weak));
                let mut end = &fresh;
                for _ in 0..10_000 {
                    let Ty::Con(_, _, args) = end else {
                        panic!("expected list")
                    };
                    end = &args[0];
                }
                let Ty::Con(Head::Tuple(3), _, parts) = end else {
                    panic!("expected tuple")
                };
                assert!(parts == &[Ty::Var(new), Ty::Var(weak), Ty::Var(new)]);
                assert!(s.vars[new.0 as usize].lower.contains(&int));
                drop((t, fresh));
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
