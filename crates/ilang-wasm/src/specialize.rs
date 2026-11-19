//! Specializes a function for a known function argument.
//!
//! A function value is called with `call_ref`. When a direct call passes
//! `Atom::Func(g)` — the one closure of a top-level function, a constructor,
//! or a library function used as a value — the callee can be cloned with that
//! parameter bound to `g`, and every call through the parameter becomes a
//! direct `call $f{g}`. Calls that forward the parameter to another direct
//! call are specialized too, recursively, so `list.sort(xs, int.compare)`
//! turns the comparator call of the whole merge chain into a direct call.
//!
//! The pass is sound because `Atom::Func(g)` is the unique closure of `g`:
//! calling the parameter is calling `g`. A clone is only reached from call
//! sites that pass exactly that value. Parameters that escape keep their
//! value; only the calls through them are rewritten. Lowering copies a
//! parameter into a fresh local before using it, so a chain of local copies
//! is followed: a local that is defined exactly once from the parameter (or
//! from such a local) holds the same value.
//!
//! It runs before [`crate::control::split`], so the existing `may_suspend`
//! fixpoint sees the specialized call graph and decides on its own whether a
//! clone still needs suspension points. The known function must not suspend
//! (that is [`crate::control::may_suspend`] of the unspecialized program);
//! a function that does is left indirect. `--opt 0` never runs this pass: the
//! reference lowering stays independent of the optimized one.
//!
//! The clone budget bounds code growth: each successful clone is one copy of
//! a function body, and the number of distinct (callee, parameter, function)
//! triples is bounded by the call sites.

use crate::ir::*;
use std::collections::{HashMap, HashSet};

/// The most clones one program may gain, as a multiple of its functions.
const CLONE_BUDGET: usize = 8;

pub fn run(p: &mut Program) {
    // The existing fixpoint decides which functions really may suspend.
    let may = crate::control::may_suspend(p);
    let originals = p.funcs.len();
    let mut state = State {
        memo: HashMap::new(),
        may,
        budget: CLONE_BUDGET.saturating_mul(originals).max(16),
        clones: 0,
    };
    // Calls in the functions that already exist may pass a known function.
    // Rewrite them in place; the clones they ask for are appended and get
    // the same treatment from `specialize`.
    for i in 0..originals {
        let FuncKind::Code(f) = &p.funcs[i] else {
            continue;
        };
        let mut body = f.body.clone();
        rewrite(&mut body, &None, p, &mut state);
        if let FuncKind::Code(f) = &mut p.funcs[i] {
            f.body = body;
        }
    }
}

struct State {
    memo: HashMap<Key, Option<FuncId>>,
    may: Vec<bool>,
    budget: usize,
    clones: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Key {
    func: FuncId,
    /// The source parameter that is called, as its local index (the first
    /// `arity` locals are the parameters).
    param: u32,
    known: FuncId,
}

/// What a clone's body knows: which locals hold the parameter, and the
/// function it holds.
struct Bind {
    known: FuncId,
    aliases: HashSet<u32>,
}

/// Rewrites every call in these statements. With `bind`, calls through the
/// parameter's aliases become direct calls to `known`, and forwarding calls
/// are specialized recursively.
fn rewrite(stmts: &mut [Stmt], bind: &Option<Bind>, p: &mut Program, st: &mut State) {
    for s in stmts {
        for body in s.bodies_mut() {
            rewrite(body, bind, p, st);
        }
        match s {
            Stmt::Let {
                value: Value::Call { callee, args, .. },
                ..
            }
            | Stmt::Suspendable {
                value: Value::Call { callee, args, .. },
                ..
            } => rewrite_call(callee, args, bind, p, st),
            Stmt::TailCall { callee, args, .. } => rewrite_call(callee, args, bind, p, st),
            _ => {}
        }
    }
}

fn rewrite_call(
    callee: &mut Callee,
    args: &mut [Atom],
    bind: &Option<Bind>,
    p: &mut Program,
    st: &mut State,
) {
    // A call through the specialized parameter.
    if let Some(bind) = bind {
        let called = match callee {
            Callee::Value(Atom::Local(l)) => bind.aliases.contains(l),
            // A labelled call keeps its shape only when the target's own
            // shape agrees, which `Callee::Known` looks up itself.
            Callee::Shaped(Atom::Local(l), shape) => {
                bind.aliases.contains(l) && p.func_shapes.get(&bind.known) == Some(shape)
            }
            _ => false,
        };
        if called {
            *callee = Callee::Known {
                func: bind.known,
                closure: Atom::Func(bind.known),
            };
            return;
        }
    }
    // A direct call that forwards the parameter, or that passes a known
    // function: specialize its callee at that argument.
    let func = match callee {
        Callee::Direct(f) => *f,
        Callee::Known { func, .. } => *func,
        _ => return,
    };
    for (i, arg) in args.iter_mut().enumerate() {
        let known = match arg {
            Atom::Func(g) => *g,
            Atom::Local(l) => match bind {
                Some(bind) if bind.aliases.contains(l) => bind.known,
                _ => continue,
            },
            _ => continue,
        };
        if let Some(new) = specialize(p, st, func, i as u32, known) {
            *callee = match callee {
                Callee::Direct(_) => Callee::Direct(new),
                Callee::Known { closure, .. } => Callee::Known {
                    func: new,
                    closure: closure.clone(),
                },
                _ => unreachable!(),
            };
            *arg = Atom::Func(known);
        }
    }
}

fn specialize(
    p: &mut Program,
    st: &mut State,
    func: FuncId,
    param: u32,
    known: FuncId,
) -> Option<FuncId> {
    let key = Key {
        func,
        param,
        known,
    };
    if let Some(found) = st.memo.get(&key) {
        return *found;
    }
    // The known function must not suspend: a real suspension inside it would
    // be lost if it were treated as a plain direct call. This uses the same
    // fixpoint as `control::split`.
    if st.may.get(known.0 as usize).copied().unwrap_or(true) {
        st.memo.insert(key, None);
        return None;
    }
    let FuncKind::Code(base) = &p.funcs.get(func.0 as usize)? else {
        st.memo.insert(key, None);
        return None;
    };
    if param >= base.arity || st.clones >= st.budget {
        st.memo.insert(key, None);
        return None;
    }
    let aliases = aliases(&base.body, param);
    // A parameter that is never called here and never reaches a direct call
    // (only returned, stored or captured) has nothing to devirtualize; a
    // clone would only copy the body. `list.sort` forwards its comparator
    // instead of calling it, which still reaches the calls deeper down.
    if !parameter_reaches_call(&base.body, &aliases) {
        st.memo.insert(key, None);
        return None;
    }
    let mut clone = base.clone();
    let id = FuncId(p.funcs.len() as u32);
    clone.name = format!("{}#spec{param}_{}", clone.name, known.0);
    st.memo.insert(key, Some(id));
    st.clones += 1;
    p.funcs.push(FuncKind::Code(clone));
    // The tables keyed by function identity. A clone has the same body
    // shape, so the numeric plan, the labels and the worker signature are
    // the same.
    if let Some(v) = p.numeric_locals.get(&func).cloned() {
        p.numeric_locals.insert(id, v);
    }
    if let Some(v) = p.func_shapes.get(&func).copied() {
        p.func_shapes.insert(id, v);
    }
    if let Some(v) = p.workers.get(&func).cloned() {
        p.workers.insert(id, v);
    }
    if let Some(v) = p.lazy_fields.get(&func).cloned() {
        p.lazy_fields.insert(id, v);
    }
    let mut body = match &p.funcs[id.0 as usize] {
        FuncKind::Code(f) => f.body.clone(),
        _ => unreachable!(),
    };
    rewrite(&mut body, &Some(Bind { known, aliases }), p, st);
    if let FuncKind::Code(f) = &mut p.funcs[id.0 as usize] {
        f.body = body;
    }
    Some(id)
}

/// The locals that hold the parameter: the parameter itself plus each local
/// defined exactly once, at the top level of the function, as a copy of one
/// of them. Only top-level copies are followed: a copy under an `If` may not
/// run before the call, and then the original would call a null value where
/// the rewrite would call the function. Lowering puts its parameter copies
/// at the top level.
fn aliases(stmts: &[Stmt], param: u32) -> HashSet<u32> {
    let mut defs: HashMap<u32, Vec<Option<u32>>> = HashMap::new();
    for s in stmts {
        match s {
            Stmt::Let { dst, value, .. } => {
                let src = match value {
                    Value::Atom(Atom::Local(l)) => Some(*l),
                    _ => None,
                };
                defs.entry(*dst).or_default().push(src);
            }
            Stmt::LetNum { dst, .. } | Stmt::LetEnv { dst, .. } | Stmt::Suspendable { dst, .. } => {
                defs.entry(*dst).or_default().push(None)
            }
            Stmt::ReadUtf {
                codepoint,
                width,
                ok,
                ..
            } => {
                for dst in [codepoint, width, ok] {
                    defs.entry(*dst).or_default().push(None);
                }
            }
            _ => {}
        }
    }
    let mut set = HashSet::from([param]);
    loop {
        let mut changed = false;
        for (dst, sources) in &defs {
            if set.contains(dst) || sources.len() != 1 {
                continue;
            }
            if sources[0].is_some_and(|src| set.contains(&src)) {
                set.insert(*dst);
                changed = true;
            }
        }
        if !changed {
            return set;
        }
    }
}

/// Whether the parameter is called, or forwarded to a direct call that can
/// be specialized in turn. A parameter that only escapes (returned, stored,
/// captured by another closure) does not qualify.
fn parameter_reaches_call(stmts: &[Stmt], aliases: &HashSet<u32>) -> bool {
    stmts.iter().any(|s| {
        let call = match s {
            Stmt::Let {
                value: Value::Call { callee, args, .. },
                ..
            }
            | Stmt::Suspendable {
                value: Value::Call { callee, args, .. },
                ..
            }
            | Stmt::TailCall { callee, args, .. } => Some((callee, args)),
            _ => None,
        };
        if let Some((callee, args)) = call {
            match callee {
                Callee::Value(Atom::Local(l)) | Callee::Shaped(Atom::Local(l), _)
                    if aliases.contains(l) =>
                {
                    return true;
                }
                Callee::Direct(_) | Callee::Known { .. }
                    if args
                        .iter()
                        .any(|a| matches!(a, Atom::Local(l) if aliases.contains(l))) =>
                {
                    return true;
                }
                _ => {}
            }
        }
        s.bodies().any(|b| parameter_reaches_call(b, aliases))
    })
}
