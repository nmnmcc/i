//! Splitting the functions that may suspend (§9.4), after lowering.
//!
//! Frames are captured lazily (see `ir.rs`): nothing is saved until
//! `suspend` throws `$capture`. This pass finds the functions that may
//! suspend with a fixpoint over the whole program, and turns each call in
//! them that may suspend into a [`Stmt::Suspendable`] with a [`Point`]:
//!
//! - the **live** locals after the call, from a backward liveness pass over
//!   the structured code, are what the frame record keeps;
//! - the environment locals that the rest of the function uses are not
//!   kept: the frame record keeps the values of their environment
//!   statements, and they are made again from the environment of the place
//!   of `resume` (§9.4 rule 3);
//! - the **segment** is the rest of the function after the call: the rest
//!   of its statement list, then the rest of each enclosing list, with the
//!   enclosing blocks around them so that `Break` still leaves them. In a
//!   segment, each later `Suspendable` is followed by a `Continue` to its own
//!   segment, so every part of the code is in few segments.
//!
//! A function may suspend also when it only makes a tail call to a closure
//! or to a function that may suspend: its caller's frame is on the stack
//! when it suspends. But tail calls leave no frame, so they are not split.
//! The function itself is not changed otherwise: the normal path runs the
//! rest of it in place.

use crate::ir::*;
use std::collections::{BTreeSet, HashMap};

/// Splits every function of the program that may suspend.
pub fn split(p: &mut Program) {
    let may = may_suspend(p);
    for i in 0..p.funcs.len() {
        if !may[i] {
            continue;
        }
        let FuncKind::Code(f) = &mut p.funcs[i] else {
            continue;
        };
        let first = p.points.len() as PointId;
        let mut next = first;
        mark(&mut f.body, &may, &mut next);
        if next == first {
            continue;
        }
        let f = &*f;
        let mut defs = HashMap::new();
        env_defs(&f.body, &mut defs);
        let mut live = HashMap::new();
        live_stmts(&f.body, BTreeSet::new(), &mut HashMap::new(), &mut live);
        let mut paths = HashMap::new();
        find(&f.body, &mut Vec::new(), &mut paths);
        for point in first..next {
            let path = &paths[&point];
            let (list, at) = *path.last().expect("a point is in its function");
            let Stmt::Suspendable {
                dst, value, site, ..
            } = &list[at]
            else {
                unreachable!()
            };
            let env = match value {
                Value::Call { env, .. } | Value::OpEnv(_, _, env) => *env,
                _ => Env::Param,
            };
            let after = &live[&point];
            // Every environment that is used, and the ones it is made from.
            let mut needed: BTreeSet<Local> = after
                .iter()
                .copied()
                .filter(|&l| f.locals[l as usize] == LocalType::Env)
                .collect();
            if let Env::Local(l) = env {
                needed.insert(l);
            }
            let mut todo: Vec<Local> = needed.iter().copied().collect();
            while let Some(l) = todo.pop() {
                if let Some(&(Env::Local(from), _)) = defs.get(&l)
                    && needed.insert(from)
                {
                    todo.push(from);
                }
            }
            let envs: Vec<EnvLink> = needed
                .iter()
                .map(|&l| {
                    let (from, service) = defs[&l];
                    EnvLink {
                        dst: l,
                        from,
                        service,
                    }
                })
                .collect();
            let params: Vec<Local> = after
                .iter()
                .copied()
                .filter(|&l| l != *dst && f.locals[l as usize] != LocalType::Env)
                .chain(needed.iter().copied())
                .collect();
            p.points.push(Point {
                func: FuncId(i as u32),
                site: *site,
                env,
                dst: *dst,
                params,
                envs,
                segment: rest(path),
                locals: Vec::new(),
            });
        }
        // The locals that each segment declares: those it mentions, and
        // those that the later points it goes on with need.
        for point in first..next {
            let pt = &p.points[point as usize];
            let mut set: BTreeSet<Local> = pt.params.iter().copied().collect();
            set.insert(pt.dst);
            mentioned(&pt.segment, &mut set, &mut |q, set| {
                set.extend(p.points[q as usize].params.iter().copied());
                set.insert(p.points[q as usize].dst);
            });
            p.points[point as usize].locals = set.into_iter().collect();
        }
    }
}

/// The locals that statements set or use, and those of the points they
/// have.
fn mentioned(
    stmts: &[Stmt],
    set: &mut BTreeSet<Local>,
    point: &mut dyn FnMut(PointId, &mut BTreeSet<Local>),
) {
    for s in stmts {
        let mut uses = Vec::new();
        let mut add = |l| uses.push(l);
        match s {
            Stmt::Let { dst, value, .. } => {
                add(*dst);
                value_uses(value, &mut add);
            }
            Stmt::Suspendable {
                dst,
                value,
                point: q,
                ..
            } => {
                add(*dst);
                value_uses(value, &mut add);
                point(*q, set);
            }
            Stmt::Continue(q) => point(*q, set),
            Stmt::LetNum { dst, value, .. } => {
                add(*dst);
                num_value_uses(value, &mut add);
            }
            Stmt::ReadUtf {
                codepoint,
                width,
                ok,
                bits,
                start,
                ..
            } => {
                add(*codepoint);
                add(*width);
                add(*ok);
                atom_uses(bits, &mut add);
                num_uses(start, &mut add);
            }
            Stmt::If {
                cond,
                then,
                otherwise,
            } => {
                cond_uses(cond, &mut add);
                mentioned(then, set, point);
                mentioned(otherwise, set, point);
            }
            Stmt::Block { body, .. } => mentioned(body, set, point),
            Stmt::Break(_) | Stmt::Unreachable => {}
            Stmt::Return(a) | Stmt::SetGlobal(_, a) => atom_uses(a, &mut add),
            Stmt::TailCall { callee, args, env } => call_uses(callee, args, *env, &mut add),
            Stmt::LetEnv {
                dst, env, value, ..
            } => {
                add(*dst);
                env_uses(*env, &mut add);
                atom_uses(value, &mut add);
            }
            Stmt::SetCapture { closure, value, .. } => {
                add(*closure);
                atom_uses(value, &mut add);
            }
        }
        set.extend(uses);
    }
}

/// Whether each function may suspend: it calls a closure, an operation that
/// may suspend, or a function that may suspend. Tail calls count here: the
/// frame of the caller of such a function is on the stack when it suspends.
fn may_suspend(p: &Program) -> Vec<bool> {
    let mut may: Vec<bool> = p
        .funcs
        .iter()
        .map(|f| matches!(f, FuncKind::Runtime { op, .. } if op.suspends()))
        .collect();
    loop {
        let mut changed = false;
        for (i, f) in p.funcs.iter().enumerate() {
            if may[i] {
                continue;
            }
            if let FuncKind::Code(f) = f
                && any_suspends(&f.body, &may)
            {
                may[i] = true;
                changed = true;
            }
        }
        if !changed {
            return may;
        }
    }
}

fn suspends(v: &Value, may: &[bool]) -> bool {
    match v {
        Value::Call {
            callee: Callee::Value(_),
            ..
        } => true,
        Value::Call {
            callee: Callee::Direct(f),
            ..
        } => may[f.0 as usize],
        Value::Op(op, _) | Value::OpEnv(op, _, _) => op.suspends(),
        _ => false,
    }
}

fn any_suspends(stmts: &[Stmt], may: &[bool]) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::Let { value, .. } => suspends(value, may),
        Stmt::TailCall {
            callee: Callee::Value(_),
            ..
        } => true,
        Stmt::TailCall {
            callee: Callee::Direct(f),
            ..
        } => may[f.0 as usize],
        Stmt::If {
            then, otherwise, ..
        } => any_suspends(then, may) || any_suspends(otherwise, may),
        Stmt::Block { body, .. } => any_suspends(body, may),
        Stmt::Suspendable { .. } => true,
        _ => false,
    })
}

/// Turns every call that may suspend into a `Suspendable` with the next
/// point id.
fn mark(stmts: &mut [Stmt], may: &[bool], next: &mut PointId) {
    for s in stmts {
        match s {
            Stmt::Let { dst, value, site } if suspends(value, may) => {
                *s = Stmt::Suspendable {
                    dst: *dst,
                    value: value.clone(),
                    site: *site,
                    point: *next,
                };
                *next += 1;
            }
            Stmt::If {
                then, otherwise, ..
            } => {
                mark(then, may, next);
                mark(otherwise, may, next);
            }
            Stmt::Block { body, .. } => mark(body, may, next),
            _ => {}
        }
    }
}

/// The environment statements of a function: each environment local is
/// made once.
fn env_defs(stmts: &[Stmt], out: &mut HashMap<Local, (Env, u32)>) {
    for s in stmts {
        match s {
            Stmt::LetEnv {
                dst, env, service, ..
            } => {
                out.insert(*dst, (*env, *service));
            }
            Stmt::If {
                then, otherwise, ..
            } => {
                env_defs(then, out);
                env_defs(otherwise, out);
            }
            Stmt::Block { body, .. } => env_defs(body, out),
            _ => {}
        }
    }
}

type Path<'a> = Vec<(&'a [Stmt], usize)>;

/// For each point, the statement lists from the body to it, with the index
/// in each.
fn find<'a>(stmts: &'a [Stmt], path: &mut Path<'a>, out: &mut HashMap<PointId, Path<'a>>) {
    for (i, s) in stmts.iter().enumerate() {
        path.push((stmts, i));
        match s {
            Stmt::Suspendable { point, .. } => {
                out.insert(*point, path.clone());
            }
            Stmt::If {
                then, otherwise, ..
            } => {
                find(then, path, out);
                find(otherwise, path, out);
            }
            Stmt::Block { body, .. } => find(body, path, out),
            _ => {}
        }
        path.pop();
    }
}

/// The segment of the point at the end of `path`: the rest of the function
/// after it. The rest of an `If` branch goes on after the `If`; the rest of
/// a block body is in the block, for its `Break`s.
fn rest(path: &[(&[Stmt], usize)]) -> Vec<Stmt> {
    let (list, at) = path[path.len() - 1];
    let mut out = cut(&list[at + 1..]);
    for &(list, at) in path[..path.len() - 1].iter().rev() {
        match &list[at] {
            Stmt::Block { label, .. } => {
                out = vec![Stmt::Block {
                    label: *label,
                    body: out,
                }];
            }
            // A branch that goes on with a segment does not end.
            Stmt::If { .. } if matches!(out.last(), Some(Stmt::Continue(_))) => continue,
            Stmt::If { .. } => {}
            _ => unreachable!("only blocks and ifs have statements"),
        }
        out.extend(cut(&list[at + 1..]));
    }
    out
}

/// A copy of the statements in which each `Suspendable` goes on with the
/// segment of its point, so that the code after it is not copied.
fn cut(stmts: &[Stmt]) -> Vec<Stmt> {
    let mut out = Vec::new();
    for s in stmts {
        match s {
            Stmt::Suspendable { point, .. } => {
                out.push(s.clone());
                out.push(Stmt::Continue(*point));
                return out;
            }
            Stmt::If {
                cond,
                then,
                otherwise,
            } => out.push(Stmt::If {
                cond: cond.clone(),
                then: cut(then),
                otherwise: cut(otherwise),
            }),
            Stmt::Block { label, body } => out.push(Stmt::Block {
                label: *label,
                body: cut(body),
            }),
            s => out.push(s.clone()),
        }
    }
    out
}

// ---- Liveness ----

type Live = BTreeSet<Local>;

/// The locals that are used before they are set, from the start of
/// `stmts`, when `after` is live at their end. `breaks` has what is live
/// after each enclosing block. The live locals after each `Suspendable` go
/// to `points`.
fn live_stmts(
    stmts: &[Stmt],
    after: Live,
    breaks: &mut HashMap<Label, Live>,
    points: &mut HashMap<PointId, Live>,
) -> Live {
    let mut live = after;
    for s in stmts.iter().rev() {
        live = live_stmt(s, live, breaks, points);
    }
    live
}

fn live_stmt(
    s: &Stmt,
    mut live: Live,
    breaks: &mut HashMap<Label, Live>,
    points: &mut HashMap<PointId, Live>,
) -> Live {
    match s {
        Stmt::Let { dst, value, .. } => {
            live.remove(dst);
            value_uses(value, &mut |l| {
                live.insert(l);
            });
        }
        Stmt::Suspendable {
            dst, value, point, ..
        } => {
            points.insert(*point, live.clone());
            live.remove(dst);
            value_uses(value, &mut |l| {
                live.insert(l);
            });
        }
        Stmt::LetNum { dst, value, .. } => {
            live.remove(dst);
            num_value_uses(value, &mut |l| {
                live.insert(l);
            });
        }
        Stmt::ReadUtf {
            codepoint,
            width,
            ok,
            bits,
            start,
            ..
        } => {
            live.remove(codepoint);
            live.remove(width);
            live.remove(ok);
            atom_uses(bits, &mut |l| {
                live.insert(l);
            });
            num_uses(start, &mut |l| {
                live.insert(l);
            });
        }
        Stmt::If {
            cond,
            then,
            otherwise,
        } => {
            let a = live_stmts(then, live.clone(), breaks, points);
            let b = live_stmts(otherwise, live, breaks, points);
            live = a;
            live.extend(b);
            cond_uses(cond, &mut |l| {
                live.insert(l);
            });
        }
        Stmt::Block { label, body } => {
            let saved = breaks.insert(*label, live.clone());
            live = live_stmts(body, live, breaks, points);
            match saved {
                Some(s) => breaks.insert(*label, s),
                None => breaks.remove(label),
            };
        }
        Stmt::Break(label) => live = breaks.get(label).cloned().unwrap_or_default(),
        Stmt::Return(a) => {
            live = Live::new();
            atom_uses(a, &mut |l| {
                live.insert(l);
            });
        }
        Stmt::TailCall { callee, args, env } => {
            live = Live::new();
            call_uses(callee, args, *env, &mut |l| {
                live.insert(l);
            });
        }
        Stmt::LetEnv {
            dst, env, value, ..
        } => {
            live.remove(dst);
            env_uses(*env, &mut |l| {
                live.insert(l);
            });
            atom_uses(value, &mut |l| {
                live.insert(l);
            });
        }
        Stmt::Unreachable => live = Live::new(),
        Stmt::SetCapture { closure, value, .. } => {
            live.insert(*closure);
            atom_uses(value, &mut |l| {
                live.insert(l);
            });
        }
        Stmt::SetGlobal(_, a) => atom_uses(a, &mut |l| {
            live.insert(l);
        }),
        Stmt::Continue(_) => unreachable!("liveness is worked out before segments"),
    }
    live
}

fn atom_uses(a: &Atom, f: &mut dyn FnMut(Local)) {
    if let Atom::Local(l) = a {
        f(*l);
    }
}

fn num_uses(n: &Num, f: &mut dyn FnMut(Local)) {
    if let Num::Local(l) = n {
        f(*l);
    }
}

fn env_uses(e: Env, f: &mut dyn FnMut(Local)) {
    if let Env::Local(l) = e {
        f(l);
    }
}

fn call_uses(callee: &Callee, args: &[Atom], env: Env, f: &mut dyn FnMut(Local)) {
    if let Callee::Value(a) = callee {
        atom_uses(a, f);
    }
    args.iter().for_each(|a| atom_uses(a, f));
    env_uses(env, f);
}

fn value_uses(v: &Value, f: &mut dyn FnMut(Local)) {
    match v {
        Value::Atom(a)
        | Value::Field(a, _)
        | Value::ListHead(a)
        | Value::ListTail(a)
        | Value::StringSuffix(a, _)
        | Value::BitsWhole(a)
        | Value::BitsFromFloat { value: a, .. }
        | Value::BitsFromUtf { value: a, .. } => atom_uses(a, f),
        Value::Capture(_) | Value::BitsEmpty => {}
        Value::Call { callee, args, env } => call_uses(callee, args, *env, f),
        Value::Op(_, args) | Value::Tuple(args) => args.iter().for_each(|a| atom_uses(a, f)),
        Value::OpEnv(_, args, env) => {
            args.iter().for_each(|a| atom_uses(a, f));
            env_uses(*env, f);
        }
        Value::Service(env, _) => env_uses(*env, f),
        Value::Closure { captures, .. } => captures.iter().for_each(|a| atom_uses(a, f)),
        Value::Record { fields, .. } => fields.iter().for_each(|a| atom_uses(a, f)),
        Value::Cons(a, b) | Value::BitsConcat(a, b) => {
            atom_uses(a, f);
            atom_uses(b, f);
        }
        Value::BitsFromInt { value, width, .. } => {
            atom_uses(value, f);
            num_uses(width, f);
        }
        Value::BitsTake(a, n) => {
            atom_uses(a, f);
            num_uses(n, f);
        }
        Value::BitsReadInt {
            bits, start, width, ..
        } => {
            atom_uses(bits, f);
            num_uses(start, f);
            num_uses(width, f);
        }
        Value::BitsReadFloat { bits, start, .. } => {
            atom_uses(bits, f);
            num_uses(start, f);
        }
        Value::BitsSlice {
            bits,
            start,
            length,
        } => {
            atom_uses(bits, f);
            num_uses(start, f);
            num_uses(length, f);
        }
    }
}

fn num_value_uses(v: &NumValue, f: &mut dyn FnMut(Local)) {
    match v {
        NumValue::Num(n) => num_uses(n, f),
        NumValue::Add(a, b) | NumValue::Sub(a, b) => {
            num_uses(a, f);
            num_uses(b, f);
        }
        NumValue::BitsLength(a) | NumValue::LengthOf(a, _) | NumValue::WidthOf(a, _) => {
            atom_uses(a, f)
        }
    }
}

fn cond_uses(c: &Cond, f: &mut dyn FnMut(Local)) {
    match c {
        Cond::True(a)
        | Cond::Tag(a, _)
        | Cond::Bool(a, _)
        | Cond::Ctor(a, _)
        | Cond::Type(a, _)
        | Cond::Tuple(a, _)
        | Cond::Empty(a)
        | Cond::IsInt(a, _)
        | Cond::IsFloat(a, _)
        | Cond::IsString(a, _) => atom_uses(a, f),
        Cond::Not(c) => cond_uses(c, f),
        Cond::And(a, b) | Cond::Or(a, b) => {
            cond_uses(a, f);
            cond_uses(b, f);
        }
        Cond::Equal(a, b) | Cond::StartsWith(a, b) | Cond::BitsEqual(a, b) => {
            atom_uses(a, f);
            atom_uses(b, f);
        }
        Cond::I32(l) => f(*l),
        Cond::NumLt(a, b) | Cond::NumEq(a, b) => {
            num_uses(a, f);
            num_uses(b, f);
        }
        Cond::Whole(n) => num_uses(n, f),
        Cond::BitsHas(a, s, w) => {
            atom_uses(a, f);
            num_uses(s, f);
            num_uses(w, f);
        }
        Cond::BitsFloat { bits, start, .. } => {
            atom_uses(bits, f);
            num_uses(start, f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A function with an `i64` local and an environment that are live after
    /// a closure call: the frame record boxes the number, keeps the value of
    /// the environment statement, and the module is valid.
    #[test]
    fn numbers_and_environments_are_kept() {
        let body = vec![
            Stmt::LetNum {
                dst: 1,
                value: NumValue::Num(Num::Const(5)),
                site: None,
            },
            Stmt::LetEnv {
                dst: 2,
                env: Env::Param,
                service: 1,
                value: Atom::Nil,
            },
            Stmt::Let {
                dst: 3,
                value: Value::Call {
                    callee: Callee::Value(Atom::Local(0)),
                    args: vec![],
                    env: Env::Local(2),
                },
                site: None,
            },
            Stmt::If {
                cond: Cond::NumLt(Num::Local(1), Num::Const(10)),
                then: vec![Stmt::TailCall {
                    callee: Callee::Value(Atom::Local(3)),
                    args: vec![],
                    env: Env::Local(2),
                }],
                otherwise: vec![Stmt::Return(Atom::Nil)],
            },
        ];
        let mut p = Program {
            funcs: vec![FuncKind::Code(Func {
                name: "f".into(),
                arity: 1,
                locals: vec![
                    LocalType::Value,
                    LocalType::I64,
                    LocalType::Env,
                    LocalType::Value,
                ],
                body,
                frame: true,
            })],
            services: 2,
            ..Program::default()
        };
        split(&mut p);
        assert_eq!(p.points.len(), 1);
        let point = &p.points[0];
        assert_eq!(point.params, vec![1, 2]);
        assert_eq!(
            point.envs,
            vec![EnvLink {
                dst: 2,
                from: Env::Param,
                service: 1
            }]
        );
        let text = crate::emit::wat(&p);
        assert!(text.contains("(struct.new $box (local.get $l1))"), "{text}");
        let wasm = wat::parse_str(&text).unwrap();
        crate::validate(&wasm).unwrap();
    }
}
