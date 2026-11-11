//! Stack-safe ownership and structural operations for shown types.
use super::{ShownFn, ShownType};
use crate::hir::TypeRef;
use std::hash::{Hash, Hasher};

impl Drop for ShownType {
    fn drop(&mut self) {
        fn push(t: ShownType, pending: &mut Vec<ShownType>) {
            if !matches!(t, ShownType::Hole) {
                pending.push(t);
            }
        }
        fn detach(t: &mut ShownType, pending: &mut Vec<ShownType>) {
            match t {
                ShownType::Named(_, xs)
                | ShownType::Tuple(xs)
                | ShownType::Union(xs)
                | ShownType::Inter(xs) => pending.append(xs),
                ShownType::Fn(f) => {
                    pending.extend(std::mem::take(&mut f.params).into_iter().map(|(_, t)| t));
                    push(std::mem::replace(&mut f.ret, ShownType::Hole), pending);
                    if let Some(t) = f.err.take() {
                        push(t, pending);
                    }
                    if let Some(t) = f.env.take() {
                        push(t, pending);
                    }
                }
                ShownType::Diff(a, xs) => {
                    pending.append(xs);
                    push(std::mem::replace(a.as_mut(), ShownType::Hole), pending);
                }
                _ => {}
            }
        }
        let mut pending = Vec::new();
        detach(self, &mut pending);
        while let Some(mut t) = pending.pop() {
            detach(&mut t, &mut pending);
        }
    }
}

impl Clone for ShownType {
    fn clone(&self) -> Self {
        enum Task<'a> {
            Visit(&'a ShownType),
            Named(TypeRef, usize),
            Tuple(usize),
            Union(usize),
            Inter(usize),
            Diff(usize),
            Fn(Vec<Option<String>>, bool, bool),
        }
        let mut tasks = vec![Task::Visit(self)];
        let mut results = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Task::Visit(t) => match t {
                    ShownType::Lit(s) => results.push(ShownType::Lit(s.clone())),
                    ShownType::Var(s) => results.push(ShownType::Var(s.clone())),
                    ShownType::Hole => results.push(ShownType::Hole),
                    ShownType::Never => results.push(ShownType::Never),
                    ShownType::Named(r, xs) => {
                        tasks.push(Task::Named(*r, xs.len()));
                        tasks.extend(xs.iter().rev().map(Task::Visit));
                    }
                    ShownType::Tuple(xs) => {
                        tasks.push(Task::Tuple(xs.len()));
                        tasks.extend(xs.iter().rev().map(Task::Visit));
                    }
                    ShownType::Union(xs) => {
                        tasks.push(Task::Union(xs.len()));
                        tasks.extend(xs.iter().rev().map(Task::Visit));
                    }
                    ShownType::Inter(xs) => {
                        tasks.push(Task::Inter(xs.len()));
                        tasks.extend(xs.iter().rev().map(Task::Visit));
                    }
                    ShownType::Diff(a, xs) => {
                        tasks.push(Task::Diff(xs.len()));
                        tasks.extend(xs.iter().rev().map(Task::Visit));
                        tasks.push(Task::Visit(a));
                    }
                    ShownType::Fn(f) => {
                        tasks.push(Task::Fn(
                            f.params.iter().map(|(n, _)| n.clone()).collect(),
                            f.err.is_some(),
                            f.env.is_some(),
                        ));
                        if let Some(t) = &f.env {
                            tasks.push(Task::Visit(t));
                        }
                        if let Some(t) = &f.err {
                            tasks.push(Task::Visit(t));
                        }
                        tasks.push(Task::Visit(&f.ret));
                        tasks.extend(f.params.iter().rev().map(|(_, t)| Task::Visit(t)));
                    }
                },
                Task::Named(r, n) => {
                    let xs = results.split_off(results.len() - n);
                    results.push(ShownType::Named(r, xs));
                }
                Task::Tuple(n) => {
                    let xs = results.split_off(results.len() - n);
                    results.push(ShownType::Tuple(xs));
                }
                Task::Union(n) => {
                    let xs = results.split_off(results.len() - n);
                    results.push(ShownType::Union(xs));
                }
                Task::Inter(n) => {
                    let xs = results.split_off(results.len() - n);
                    results.push(ShownType::Inter(xs));
                }
                Task::Diff(n) => {
                    let mut xs = results.split_off(results.len() - n - 1).into_iter();
                    let a = xs.next().expect("shown difference subject");
                    results.push(ShownType::Diff(Box::new(a), xs.collect()));
                }
                Task::Fn(names, err, env) => {
                    let mut xs = results
                        .split_off(
                            results.len() - names.len() - 1 - usize::from(err) - usize::from(env),
                        )
                        .into_iter();
                    let params = names
                        .into_iter()
                        .map(|n| (n, xs.next().expect("shown parameter")))
                        .collect();
                    let ret = xs.next().expect("shown return");
                    let err = if err {
                        Some(xs.next().expect("shown error"))
                    } else {
                        None
                    };
                    let env = if env {
                        Some(xs.next().expect("shown environment"))
                    } else {
                        None
                    };
                    results.push(ShownType::Fn(Box::new(ShownFn {
                        params,
                        ret,
                        err,
                        env,
                    })));
                }
            }
        }
        assert_eq!(results.len(), 1, "shown type copy result");
        results.pop().unwrap()
    }
}

impl PartialEq for ShownType {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some((a, b)) = pending.pop() {
            match (a, b) {
                (ShownType::Lit(a), ShownType::Lit(b)) | (ShownType::Var(a), ShownType::Var(b))
                    if a == b => {}
                (ShownType::Hole, ShownType::Hole) | (ShownType::Never, ShownType::Never) => {}
                (ShownType::Named(a, xs), ShownType::Named(b, ys))
                    if a == b && xs.len() == ys.len() =>
                {
                    pending.extend(xs.iter().zip(ys))
                }
                (ShownType::Tuple(xs), ShownType::Tuple(ys))
                | (ShownType::Union(xs), ShownType::Union(ys))
                | (ShownType::Inter(xs), ShownType::Inter(ys))
                    if xs.len() == ys.len() =>
                {
                    pending.extend(xs.iter().zip(ys))
                }
                (ShownType::Diff(a, xs), ShownType::Diff(b, ys)) if xs.len() == ys.len() => {
                    pending.push((a, b));
                    pending.extend(xs.iter().zip(ys));
                }
                (ShownType::Fn(a), ShownType::Fn(b))
                    if a.params.len() == b.params.len()
                        && a.err.is_some() == b.err.is_some()
                        && a.env.is_some() == b.env.is_some() =>
                {
                    for ((an, at), (bn, bt)) in a.params.iter().zip(&b.params) {
                        if an != bn {
                            return false;
                        }
                        pending.push((at, bt));
                    }
                    pending.push((&a.ret, &b.ret));
                    if let (Some(a), Some(b)) = (&a.err, &b.err) {
                        pending.push((a, b));
                    }
                    if let (Some(a), Some(b)) = (&a.env, &b.env) {
                        pending.push((a, b));
                    }
                }
                _ => return false,
            }
        }
        true
    }
}
impl Eq for ShownType {}
impl Hash for ShownType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut pending = vec![self];
        while let Some(t) = pending.pop() {
            std::mem::discriminant(t).hash(state);
            match t {
                ShownType::Lit(s) | ShownType::Var(s) => s.hash(state),
                ShownType::Hole | ShownType::Never => {}
                ShownType::Named(r, xs) => {
                    r.hash(state);
                    xs.len().hash(state);
                    pending.extend(xs.iter().rev());
                }
                ShownType::Tuple(xs) | ShownType::Union(xs) | ShownType::Inter(xs) => {
                    xs.len().hash(state);
                    pending.extend(xs.iter().rev());
                }
                ShownType::Diff(a, xs) => {
                    xs.len().hash(state);
                    pending.extend(xs.iter().rev());
                    pending.push(a);
                }
                ShownType::Fn(f) => {
                    f.params.len().hash(state);
                    for (label, _) in &f.params {
                        label.hash(state);
                    }
                    f.err.is_some().hash(state);
                    f.env.is_some().hash(state);
                    if let Some(t) = &f.env {
                        pending.push(t);
                    }
                    if let Some(t) = &f.err {
                        pending.push(t);
                    }
                    pending.push(&f.ret);
                    pending.extend(f.params.iter().rev().map(|(_, t)| t));
                }
            }
        }
    }
}

impl std::fmt::Debug for ShownType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        enum Part<'a> {
            Type(&'a ShownType),
            Text(&'static str),
            Label(&'a Option<String>),
        }
        fn list<'a>(xs: &'a [ShownType], pending: &mut Vec<Part<'a>>) {
            for (i, t) in xs.iter().enumerate().rev() {
                pending.push(Part::Type(t));
                if i != 0 {
                    pending.push(Part::Text(", "));
                }
            }
        }
        let mut pending = vec![Part::Type(self)];
        while let Some(part) = pending.pop() {
            match part {
                Part::Text(s) => f.write_str(s)?,
                Part::Label(s) => write!(f, "{s:?}")?,
                Part::Type(t) => match t {
                    ShownType::Lit(s) => write!(f, "Lit({s:?})")?,
                    ShownType::Var(s) => write!(f, "Var({s:?})")?,
                    ShownType::Hole => f.write_str("Hole")?,
                    ShownType::Never => f.write_str("Never")?,
                    ShownType::Named(r, xs) => {
                        write!(f, "Named({r:?}, [")?;
                        pending.push(Part::Text("])"));
                        list(xs, &mut pending);
                    }
                    ShownType::Tuple(xs) | ShownType::Union(xs) | ShownType::Inter(xs) => {
                        f.write_str(match t {
                            ShownType::Tuple(_) => "Tuple([",
                            ShownType::Union(_) => "Union([",
                            _ => "Inter([",
                        })?;
                        pending.push(Part::Text("])"));
                        list(xs, &mut pending);
                    }
                    ShownType::Diff(a, xs) => {
                        f.write_str("Diff(")?;
                        pending.push(Part::Text("])"));
                        list(xs, &mut pending);
                        pending.push(Part::Text(", ["));
                        pending.push(Part::Type(a));
                    }
                    ShownType::Fn(fun) => {
                        f.write_str("Fn(ShownFn { params: [")?;
                        pending.push(Part::Text(" })"));
                        if let Some(t) = &fun.env {
                            pending.push(Part::Text(")"));
                            pending.push(Part::Type(t));
                            pending.push(Part::Text("Some("));
                        } else {
                            pending.push(Part::Text("None"));
                        }
                        pending.push(Part::Text(", env: "));
                        if let Some(t) = &fun.err {
                            pending.push(Part::Text(")"));
                            pending.push(Part::Type(t));
                            pending.push(Part::Text("Some("));
                        } else {
                            pending.push(Part::Text("None"));
                        }
                        pending.push(Part::Text(", err: "));
                        pending.push(Part::Type(&fun.ret));
                        pending.push(Part::Text("], ret: "));
                        for (i, (label, t)) in fun.params.iter().enumerate().rev() {
                            pending.push(Part::Text(")"));
                            pending.push(Part::Type(t));
                            pending.push(Part::Text(", "));
                            pending.push(Part::Label(label));
                            pending.push(Part::Text("("));
                            if i != 0 {
                                pending.push(Part::Text(", "));
                            }
                        }
                    }
                },
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::BuiltinType;
    use std::collections::hash_map::DefaultHasher;

    #[test]
    fn deep_shown_types_are_safe_on_small_stacks() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut t = ShownType::Var("A".into());
                for i in 0..10_000 {
                    t = if i % 2 == 0 {
                        ShownType::Named(TypeRef::Builtin(BuiltinType::List), vec![t])
                    } else {
                        ShownType::Fn(Box::new(ShownFn {
                            params: Vec::new(),
                            ret: t,
                            err: None,
                            env: None,
                        }))
                    };
                }
                let copy = t.clone();
                assert_eq!(t, copy);
                let mut a = DefaultHasher::new();
                let mut b = DefaultHasher::new();
                t.hash(&mut a);
                copy.hash(&mut b);
                assert_eq!(a.finish(), b.finish());
                assert!(t.text(&[]).contains('A'));
                let debug = format!("{t:?}");
                assert!(debug.contains("Var(\"A\")"));
                let mut refs = Vec::new();
                t.type_refs(&mut refs);
                assert_eq!(refs.len(), 5_000);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
