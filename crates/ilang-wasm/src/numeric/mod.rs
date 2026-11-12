//! Bounded known-code workers and non-suspending helper expansion, before split.
//! Like Flambda's inliner, bind actual parameters once and rename all locals.
use crate::ir::*;
use ilang_check::types::NumericFact;
use std::collections::{BTreeMap, BTreeSet, HashMap};
mod aggregate;
mod float_range;
mod integer;
pub(crate) mod rewrite;

#[derive(Clone, Debug)]
pub struct Worker {
    pub origin: FuncId,
    pub floats: Vec<bool>,
    pub ints: Vec<bool>,
    pub float_result: bool,
}

fn code(c: &Callee) -> Option<FuncId> {
    match c {
        Callee::Direct(f) | Callee::Known { func: f, .. } => Some(*f),
        Callee::Value(Atom::Func(f)) => Some(*f),
        _ => None,
    }
}
fn known(f: &mut Func) {
    // Only single definitions establish code identity; recursive capture patches
    // never alter the closure's code pointer.
    let mut defs: BTreeMap<Local, Vec<Value>> = BTreeMap::new();
    rewrite::walk(&mut f.body, &mut |s| {
        if let Stmt::Let { dst, value, .. } = s {
            defs.entry(*dst).or_default().push(value.clone());
        }
    });
    let mut ids = BTreeMap::new();
    loop {
        let before = ids.len();
        for (&l, vs) in &defs {
            if vs.len() != 1 {
                continue;
            }
            let id = match &vs[0] {
                Value::Closure { func, .. } | Value::Atom(Atom::Func(func)) => Some(*func),
                Value::Atom(Atom::Local(source)) => ids.get(source).copied(),
                _ => None,
            };
            if let Some(id) = id {
                ids.insert(l, id);
            }
        }
        if before == ids.len() {
            break;
        }
    }
    rewrite::walk(&mut f.body, &mut |s| {
        let c = match s {
            Stmt::Let {
                value: Value::Call { callee, .. },
                ..
            }
            | Stmt::TailCall { callee, .. } => callee,
            _ => return,
        };
        if let Callee::Value(a) = c {
            let id = match a {
                Atom::Func(id) => Some(*id),
                Atom::Local(l) => ids.get(l).copied(),
                _ => None,
            };
            if let Some(func) = id {
                *c = Callee::Known {
                    func,
                    closure: a.clone(),
                };
            }
        }
    });
}
fn float_atom(a: &Atom, locals: &BTreeSet<Local>) -> bool {
    matches!(a, Atom::Float(_)) || matches!(a, Atom::Local(l) if locals.contains(l))
}
fn float_locals(
    f: &Func,
    facts: Option<&HashMap<Local, NumericFact>>,
    results: &[bool],
) -> BTreeSet<Local> {
    let mut locals = BTreeSet::new();
    let mut defs: BTreeMap<Local, Vec<Value>> = BTreeMap::new();
    let mut body = f.body.clone();
    rewrite::walk(&mut body, &mut |s| {
        if let Stmt::Let { dst, value, .. } = s {
            defs.entry(*dst).or_default().push(value.clone());
        }
    });
    if let Some(facts) = facts {
        for (&l, fact) in facts {
            if *fact == NumericFact::FiniteF64
                && (l < f.arity || defs.get(&l).is_some_and(|v| v.len() == 1))
            {
                locals.insert(l);
            }
        }
    }
    loop {
        let before = locals.len();
        for (&l, vs) in &defs {
            if vs.iter().all(|v| match v {
                Value::Atom(a) => float_atom(a, &locals),
                Value::Op(
                    Op::FloatAdd | Op::FloatSub | Op::FloatMul | Op::FloatDiv | Op::FloatNeg,
                    _,
                ) => true,
                Value::Call { callee, .. } => code(callee).is_some_and(|f| results[f.0 as usize]),
                _ => false,
            }) {
                locals.insert(l);
            }
        }
        if locals.len() == before {
            return locals;
        }
    }
}

/// Specialize known callbacks by code identity while retaining the callback's
/// actual closure parameter. Capture values never enter a variant key.
fn callbacks(p: &mut Program) -> (usize, BTreeMap<FuncId, FuncId>, BTreeMap<FuncId, usize>) {
    let originals = p.funcs.clone();
    let mut variants = BTreeMap::new();
    let mut roots = BTreeMap::new();
    let mut counts = BTreeMap::<FuncId, usize>::new();
    let total: usize = originals
        .iter()
        .filter_map(|f| {
            if let FuncKind::Code(f) = f {
                Some(rewrite::size(&f.body))
            } else {
                None
            }
        })
        .sum();
    let mut growth = 0;
    for i in 0..originals.len() {
        let FuncKind::Code(mut caller) = p.funcs[i].clone() else {
            continue;
        };
        let mut identities = BTreeMap::new();
        let mut assignments = BTreeMap::<Local, usize>::new();
        rewrite::walk(&mut caller.body, &mut |s| {
            if let Stmt::Let { dst, value, .. } = s {
                *assignments.entry(*dst).or_default() += 1;
                match value {
                    Value::Closure { func, .. } | Value::Atom(Atom::Func(func)) => {
                        identities.insert(*dst, *func);
                    }
                    _ => {}
                }
            }
        });
        identities.retain(|l, _| assignments[l] == 1);
        let caller_facts = p
            .numeric_locals
            .get(&FuncId(i as u32))
            .cloned()
            .unwrap_or_default();
        rewrite::walk(&mut caller.body, &mut |s| {
            let (callee, args) = match s {
                Stmt::Let {
                    value: Value::Call { callee, args, .. },
                    ..
                }
                | Stmt::TailCall { callee, args, .. } => (callee, args),
                _ => return,
            };
            let Some(origin) = code(callee) else { return };
            let Some(FuncKind::Code(source)) = originals.get(origin.0 as usize) else {
                return;
            };
            for (param, arg) in args.iter().enumerate() {
                let callback = match arg {
                    Atom::Func(f) => Some(*f),
                    Atom::Local(l) => identities.get(l).copied(),
                    _ => None,
                };
                let Some(callback) = callback else { continue };
                if !p
                    .numeric_locals
                    .get(&callback)
                    .is_some_and(|facts| !facts.is_empty())
                {
                    continue;
                }
                let mut clone = source.clone();
                let mut aliases = BTreeSet::from([param as u32]);
                let mut copies = BTreeMap::<Local, Vec<Atom>>::new();
                rewrite::walk(&mut clone.body, &mut |s| {
                    if let Stmt::Let { dst, value, .. } = s {
                        copies.entry(*dst).or_default().push(match value {
                            Value::Atom(a) => a.clone(),
                            _ => Atom::Nil,
                        });
                    }
                });
                loop {
                    let before = aliases.len();
                    for (&dst, values) in &copies {
                        if values.len() == 1
                            && matches!(&values[0], Atom::Local(l) if aliases.contains(l))
                        {
                            aliases.insert(dst);
                        }
                    }
                    if before == aliases.len() {
                        break;
                    }
                }
                let mut used_callback = false;
                rewrite::walk(&mut clone.body, &mut |s| {
                    let callee = match s {
                        Stmt::Let {
                            value: Value::Call { callee, .. },
                            ..
                        }
                        | Stmt::TailCall { callee, .. } => callee,
                        _ => return,
                    };
                    if matches!(callee, Callee::Value(Atom::Local(l)) if aliases.contains(l)) {
                        *callee = Callee::Known {
                            func: callback,
                            closure: Atom::Local(param as u32),
                        };
                        used_callback = true;
                    }
                });
                if !used_callback {
                    continue;
                }
                let floats = args.iter().map(|a| matches!(a, Atom::Float(_)) || matches!(a, Atom::Local(l) if caller_facts.get(l) == Some(&NumericFact::FiniteF64))).collect::<Vec<_>>();
                let key = (origin, param, callback, floats.clone());
                let id = if let Some(&id) = variants.get(&key) {
                    id
                } else {
                    let count = counts.entry(origin).or_default();
                    let cost = rewrite::size(&clone.body);
                    if *count == 4 || growth + cost > total {
                        p.numeric_log
                            .push(format!("code {} callback budget", origin.0));
                        continue;
                    }
                    growth += cost;
                    *count += 1;
                    let id = FuncId(p.funcs.len() as u32);
                    rewrite::walk(&mut clone.body, &mut |s| {
                        let callee = match s {
                            Stmt::Let {
                                value: Value::Call { callee, .. },
                                ..
                            }
                            | Stmt::TailCall { callee, .. } => callee,
                            _ => return,
                        };
                        match callee {
                            Callee::Direct(f) | Callee::Known { func: f, .. } if *f == origin => {
                                *f = id
                            }
                            _ => {}
                        }
                    });
                    clone.name = format!("{} [callback {}:{}]", clone.name, param, callback.0);
                    let mut facts = p.numeric_locals.get(&origin).cloned().unwrap_or_default();
                    for (l, float) in floats.iter().enumerate() {
                        if *float {
                            facts.insert(l as u32, NumericFact::FiniteF64);
                        }
                    }
                    p.numeric_locals.insert(id, facts);
                    p.funcs.push(FuncKind::Code(clone));
                    roots.insert(id, origin);
                    variants.insert(key, id);
                    p.numeric_log.push(format!(
                        "callback worker {} origin={} parameter={} code={}",
                        id.0, origin.0, param, callback.0
                    ));
                    id
                };
                match callee {
                    Callee::Direct(f) | Callee::Known { func: f, .. } => *f = id,
                    _ => unreachable!("resolved callback source"),
                }
                break;
            }
        });
        p.funcs[i] = FuncKind::Code(caller);
    }
    (growth, roots, counts)
}

fn has_numeric(f: &Func) -> bool {
    let mut body = f.body.clone();
    let mut found = false;
    rewrite::walk(&mut body, &mut |s| {
        if matches!(
            s,
            Stmt::Let {
                value: Value::Op(
                    Op::IntAdd
                        | Op::IntSub
                        | Op::IntMul
                        | Op::IntDiv
                        | Op::IntRem
                        | Op::IntNeg
                        | Op::IntLt
                        | Op::IntLe
                        | Op::IntGt
                        | Op::IntGe
                        | Op::FloatAdd
                        | Op::FloatSub
                        | Op::FloatMul
                        | Op::FloatDiv
                        | Op::FloatNeg
                        | Op::FloatLt
                        | Op::FloatLe
                        | Op::FloatGt
                        | Op::FloatGe,
                    _
                ),
                ..
            }
        ) {
            found = true;
        }
    });
    found
}

pub fn run(p: &mut Program) {
    if p.numeric_locals.values().all(HashMap::is_empty)
        && !p
            .funcs
            .iter()
            .any(|f| matches!(f, FuncKind::Code(f) if has_numeric(f)))
    {
        return;
    }
    assert!(
        p.points.is_empty(),
        "numeric rewriting must precede capture splitting"
    );
    for f in &mut p.funcs {
        if let FuncKind::Code(f) = f {
            known(f);
        }
    }
    let source_count = p.funcs.len();
    let source_size: usize = p
        .funcs
        .iter()
        .filter_map(|f| {
            if let FuncKind::Code(f) = f {
                Some(rewrite::size(&f.body))
            } else {
                None
            }
        })
        .sum();
    let (mut growth, callback_roots, mut worker_counts) = callbacks(p);
    // Freeze this exclusion before any expansion. It is never weakened later.
    let may = crate::control::may_suspend(p);
    let mut edges = vec![BTreeSet::new(); p.funcs.len()];
    for (i, f) in p.funcs.iter_mut().enumerate() {
        if let FuncKind::Code(f) = f {
            rewrite::walk(&mut f.body, &mut |s| match s {
                Stmt::Let {
                    value: Value::Call { callee, .. },
                    ..
                }
                | Stmt::TailCall { callee, .. } => {
                    if let Some(id) = code(callee) {
                        edges[i].insert(id.0 as usize);
                    }
                }
                _ => {}
            });
        }
    }
    let recursive = (0..p.funcs.len())
        .map(|i| {
            let mut todo: Vec<_> = edges[i].iter().copied().collect();
            let mut seen = BTreeSet::new();
            while let Some(j) = todo.pop() {
                if j == i {
                    return true;
                }
                if seen.insert(j) {
                    todo.extend(edges[j].iter().copied());
                }
            }
            false
        })
        .collect::<Vec<_>>();
    let originals = p.funcs.clone();
    // Tail-result constraints propagate backward through acyclic predecessors
    // as well as recursive groups. Unknown tails and suspension anchor Value.
    let mut results: Vec<_> = originals
        .iter()
        .enumerate()
        .map(|(i, f)| !may[i] && matches!(f, FuncKind::Code(_)))
        .collect();
    loop {
        let before = results.clone();
        for (i, f) in originals.iter().enumerate() {
            let FuncKind::Code(f) = f else { continue };
            if !results[i] {
                continue;
            }
            let locals = float_locals(f, p.numeric_locals.get(&FuncId(i as u32)), &before);
            let mut body = f.body.clone();
            let mut valid = true;
            let mut exits = false;
            rewrite::walk(&mut body, &mut |s| match s {
                Stmt::Return(a) => {
                    exits = true;
                    valid &= float_atom(a, &locals);
                }
                Stmt::TailCall { callee, .. } => {
                    exits = true;
                    valid &= code(callee).is_some_and(|id| before[id.0 as usize]);
                }
                _ => {}
            });
            results[i] = valid && exits;
        }
        if results == before {
            break;
        }
    }
    let mut workers = BTreeMap::new();
    for (i, f) in originals.iter().enumerate() {
        let id = FuncId(i as u32);
        let FuncKind::Code(f) = f else { continue };
        p.numeric_log.push(format!(
            "code {} ({}) inline={} suspend={} recursive={}",
            i,
            f.name,
            !may[i] && !recursive[i] && rewrite::size(&f.body) <= 32,
            may[i],
            recursive[i]
        ));
        let floats = (0..f.arity)
            .map(|l| {
                p.numeric_locals.get(&id).and_then(|m| m.get(&l)) == Some(&NumericFact::FiniteF64)
            })
            .collect::<Vec<_>>();
        let mut ints = integer::parameters(f);
        for (int, float) in ints.iter_mut().zip(&floats) {
            if *float {
                *int = false;
            }
        }
        if !floats.iter().chain(&ints).any(|b| *b) && !results[i] {
            continue;
        }
        let cost = rewrite::size(&f.body);
        let root = callback_roots.get(&id).copied().unwrap_or(id);
        let count = worker_counts.entry(root).or_default();
        if *count == 4 || growth + cost > 2 * source_size {
            p.numeric_log.push(format!("code {i}: worker budget"));
            continue;
        }
        growth += cost;
        *count += 1;
        let worker = FuncId(p.funcs.len() as u32);
        let mut clone = f.clone();
        clone.name = format!("{} [numeric worker]", f.name);
        p.funcs.push(FuncKind::Code(clone));
        if let Some(facts) = p.numeric_locals.get(&id).cloned() {
            p.numeric_locals.insert(worker, facts);
        }
        workers.insert(id, worker);
        p.workers.insert(
            worker,
            Worker {
                origin: id,
                float_result: results[i] && !ints.iter().any(|b| *b),
                floats,
                ints,
            },
        );
        p.numeric_log.push(format!(
            "worker {} origin={i} float-result={}",
            worker.0, results[i]
        ));
    }
    // A scalar-result worker must have a worker for every tail successor.
    loop {
        let mut changed = false;
        let snapshot = p.workers.clone();
        for (&id, worker) in &mut p.workers {
            if !worker.float_result {
                continue;
            }
            let FuncKind::Code(f) = &originals[worker.origin.0 as usize] else {
                unreachable!()
            };
            let mut body = f.body.clone();
            rewrite::walk(&mut body, &mut |s| {
                if let Stmt::TailCall { callee, .. } = s
                    && !code(callee)
                        .and_then(|origin| workers.get(&origin))
                        .is_some_and(|id| snapshot[id].float_result)
                {
                    worker.float_result = false;
                    changed = true;
                }
            });
            let _ = id;
        }
        if !changed {
            break;
        }
    }
    // Include every appended scalar worker in the frozen pre-inlining set.
    let inline_may = crate::control::may_suspend(p);
    // Inline only original, nonrecursive, transitively non-suspending bodies.
    // Calls already evaluate actual arguments in written order in ANF.
    for i in 0..p.funcs.len() {
        let FuncKind::Code(mut f) = p.funcs[i].clone() else {
            continue;
        };
        let size = rewrite::size(&f.body);
        let limit = if i < source_count {
            size * 2
        } else {
            size + size.min(2 * source_size - growth)
        };
        let mut used = size;
        let mut labels = 0;
        rewrite::walk(&mut f.body, &mut |s| {
            if let Stmt::Block { label, .. } | Stmt::Break(label) = s {
                labels = labels.max(*label + 1);
            }
        });
        let mut facts = p
            .numeric_locals
            .remove(&FuncId(i as u32))
            .unwrap_or_default();
        let mut body = std::mem::take(&mut f.body);
        inline(
            &mut body,
            &mut f,
            &mut facts,
            &originals,
            &inline_may,
            &recursive,
            &mut labels,
            &mut used,
            limit,
            &p.numeric_locals,
        );
        f.body = body;
        if i >= source_count {
            growth += used - size;
        }
        p.numeric_locals.insert(FuncId(i as u32), facts);
        p.funcs[i] = FuncKind::Code(f);
    }
    for (i, function) in p.funcs.iter_mut().enumerate() {
        if let FuncKind::Code(f) = function
            && (has_numeric(f)
                || p.numeric_locals
                    .get(&FuncId(i as u32))
                    .is_some_and(|facts| !facts.is_empty()))
        {
            let mut candidate = f.clone();
            aggregate::run(&mut candidate);
            let before = rewrite::size(&f.body);
            let after = rewrite::size(&candidate.body);
            if i < source_count || growth + after <= 2 * source_size + before {
                if i >= source_count {
                    growth = growth + after - before;
                }
                *f = candidate;
            } else {
                p.numeric_log
                    .push(format!("code {i}: aggregate worker growth budget"));
            }
        }
    }
    // Select workers after inlining; tail conventions must agree exactly.
    for (i, function) in p.funcs.iter_mut().enumerate() {
        let FuncKind::Code(f) = function else {
            continue;
        };
        let scalar_result = p
            .workers
            .get(&FuncId(i as u32))
            .is_some_and(|w| w.float_result);
        rewrite::walk(&mut f.body, &mut |s| {
            let (callee, tail) = match s {
                Stmt::Let {
                    value: Value::Call { callee, .. },
                    ..
                } => (callee, false),
                Stmt::TailCall { callee, .. } => (callee, true),
                _ => return,
            };
            if let Some(origin) = code(callee)
                && let Some(&worker) = workers.get(&origin)
                && (!tail || scalar_result == p.workers[&worker].float_result)
            {
                match callee {
                    Callee::Direct(id) | Callee::Known { func: id, .. } => *id = worker,
                    Callee::Value(_) => {
                        unreachable!("known calls resolved before worker selection")
                    }
                }
            }
        });
    }
    for (i, function) in p.funcs.iter_mut().enumerate() {
        if let FuncKind::Code(f) = function {
            rewrite::walk(&mut f.body, &mut |s| {
                if let Stmt::Let {
                    dst,
                    value: Value::Call { callee, .. },
                    ..
                } = s
                    && code(callee)
                        .is_some_and(|id| p.workers.get(&id).is_some_and(|w| w.float_result))
                {
                    p.numeric_locals
                        .entry(FuncId(i as u32))
                        .or_default()
                        .insert(*dst, NumericFact::FiniteF64);
                }
            });
        }
    }
    // Reference entries adapt to workers without adding a logical frame. A
    // scalar-result worker has no Value/unknown tail successor, so the one
    // boxing adapter cannot accumulate recursively along source tail edges.
    for (&origin, &worker) in &workers {
        if p.workers[&worker].ints.iter().any(|b| *b) {
            continue;
        }
        let FuncKind::Code(original) = &originals[origin.0 as usize] else {
            unreachable!()
        };
        let call = Callee::Known {
            func: worker,
            closure: Atom::CurrentClosure,
        };
        let args = (0..original.arity).map(Atom::Local).collect();
        let body = if p.workers[&worker].float_result {
            vec![
                Stmt::Let {
                    dst: original.arity,
                    value: Value::Call {
                        callee: call,
                        args,
                        env: Env::Param,
                    },
                    site: None,
                },
                Stmt::Return(Atom::Local(original.arity)),
            ]
        } else {
            vec![Stmt::TailCall {
                callee: call,
                args,
                env: Env::Param,
                site: None,
            }]
        };
        p.funcs[origin.0 as usize] = FuncKind::Code(Func {
            name: format!("{} [reference entry]", original.name),
            arity: original.arity,
            budget_frame: original.budget_frame,
            locals: vec![LocalType::Value; original.arity as usize + 1],
            body,
            frame: false,
        });
        p.numeric_locals.remove(&origin);
    }
    for (i, function) in p.funcs.iter_mut().enumerate() {
        if let FuncKind::Code(f) = function
            && has_numeric(f)
        {
            let removed = float_range::run(f);
            if removed != 0 {
                p.numeric_log
                    .push(format!("finite proofs code={i} removed={removed}"));
            }
        }
    }
    let actual_extra: usize = p
        .funcs
        .iter()
        .skip(source_count)
        .filter_map(|f| {
            if let FuncKind::Code(f) = f {
                Some(rewrite::size(&f.body))
            } else {
                None
            }
        })
        .sum();
    assert!(
        actual_extra <= 2 * source_size,
        "numeric actual extra-statement budget"
    );
    assert!(growth <= 2 * source_size, "numeric extra-statement budget");
    assert!(
        worker_counts.values().all(|count| *count <= 4),
        "numeric per-binding worker budget"
    );
    p.numeric_log.push(format!(
        "budget extra-statements={growth}/{} workers-per-binding<=4",
        2 * source_size
    ));
    let after = crate::control::may_suspend(p);
    for (&id, worker) in &p.workers {
        assert!(
            !worker.float_result || !after[id.0 as usize],
            "suspending worker has scalar result"
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn inline(
    body: &mut [Stmt],
    caller: &mut Func,
    facts: &mut HashMap<Local, NumericFact>,
    funcs: &[FuncKind],
    may: &[bool],
    recursive: &[bool],
    labels: &mut u32,
    used: &mut usize,
    limit: usize,
    all_facts: &HashMap<FuncId, HashMap<Local, NumericFact>>,
) {
    for stmt in body.iter_mut() {
        match stmt {
            Stmt::If {
                then, otherwise, ..
            } => {
                inline(
                    then, caller, facts, funcs, may, recursive, labels, used, limit, all_facts,
                );
                inline(
                    otherwise, caller, facts, funcs, may, recursive, labels, used, limit, all_facts,
                );
            }
            Stmt::Block { body, .. } => inline(
                body, caller, facts, funcs, may, recursive, labels, used, limit, all_facts,
            ),
            _ => {}
        }
        let Stmt::Let {
            dst,
            value: Value::Call { callee, args, env },
            site,
        } = stmt
        else {
            continue;
        };
        let Some(id) = code(callee) else { continue };
        let FuncKind::Code(f) = &funcs[id.0 as usize] else {
            continue;
        };
        let size = rewrite::size(&f.body);
        if may[id.0 as usize] || recursive[id.0 as usize] || size > 32 || *used + size > limit {
            continue;
        }
        // Keep expansion within numeric regions; no-op programs retain bytes.
        if !all_facts.get(&id).is_some_and(|m| !m.is_empty()) && !has_numeric(f) {
            continue;
        }
        let closure = match callee {
            Callee::Known { closure, .. } => Some(closure.clone()),
            _ => None,
        };
        let mut needs_closure = false;
        let mut clone = f.body.clone();
        rewrite::walk(&mut clone, &mut |s| {
            if matches!(
                s,
                Stmt::Let {
                    value: Value::Capture(_),
                    ..
                }
            ) {
                needs_closure = true;
            }
        });
        if needs_closure && closure.is_none() {
            continue;
        }
        let offset = caller.locals.len() as u32;
        caller.locals.extend(&f.locals);
        let label_offset = *labels;
        let done = label_offset;
        *labels += 1;
        let mut max_label = 0;
        rewrite::walk(&mut clone, &mut |s| {
            if let Stmt::Block { label, .. } | Stmt::Break(label) = s {
                max_label = max_label.max(*label + 1);
            }
        });
        *labels += max_label;
        let mut expansion = Vec::new();
        for (l, a) in args.iter().enumerate() {
            expansion.push(Stmt::Let {
                dst: offset + l as u32,
                value: Value::Atom(a.clone()),
                site: None,
            });
        }
        if f.frame {
            expansion.push(Stmt::Logical(Logical::Enter(*site)));
        }
        let call_env = *env;
        let dest = *dst;
        rewrite::walk(&mut clone, &mut |s| {
            rewrite::operands(s, &mut |a| {
                if let Atom::Local(l) = a {
                    *l += offset;
                }
            });
            match s {
                Stmt::Let { dst, value, .. } => {
                    *dst += offset;
                    if let Value::Capture(index) = value {
                        *value = Value::Field(
                            closure
                                .clone()
                                .expect("inline capture has original closure"),
                            *index,
                        );
                    }
                }
                Stmt::LetNum { dst, .. } | Stmt::LetEnv { dst, .. } => *dst += offset,
                Stmt::ReadUtf {
                    codepoint,
                    width,
                    ok,
                    ..
                } => {
                    *codepoint += offset;
                    *width += offset;
                    *ok += offset;
                }
                Stmt::Block { label, .. } | Stmt::Break(label) => *label += label_offset + 1,
                _ => {}
            }
            match s {
                Stmt::Let {
                    value:
                        Value::Call { env, .. } | Value::OpEnv(_, _, env) | Value::Service(env, _),
                    ..
                }
                | Stmt::TailCall { env, .. }
                | Stmt::LetEnv { env, .. }
                    if *env == Env::Param =>
                {
                    *env = call_env
                }
                _ => {}
            }
            match s {
                Stmt::Return(a) => {
                    let mut body = vec![Stmt::Let {
                        dst: dest,
                        value: Value::Atom(a.clone()),
                        site: None,
                    }];
                    if f.frame {
                        body.push(Stmt::Logical(Logical::Leave));
                    }
                    body.push(Stmt::Break(done));
                    *s = Stmt::Block {
                        label: *labels,
                        body,
                    };
                    *labels += 1;
                }
                Stmt::TailCall {
                    callee,
                    args,
                    env,
                    site,
                } => {
                    let mut body = Vec::new();
                    if f.frame {
                        body.push(Stmt::Logical(Logical::Tail(*site)));
                    }
                    body.push(Stmt::Let {
                        dst: dest,
                        value: Value::Call {
                            callee: callee.clone(),
                            args: args.clone(),
                            env: *env,
                        },
                        site: None,
                    });
                    body.push(Stmt::Break(done));
                    *s = Stmt::Block {
                        label: *labels,
                        body,
                    };
                    *labels += 1;
                }
                _ => {}
            }
        });
        expansion.extend(clone);
        let added = rewrite::size(&expansion);
        if *used + added > limit {
            caller.locals.truncate(offset as usize);
            continue;
        }
        *used += added;
        // Fresh definition facts are renamed with the exact cloned code.
        if let Some(source) = all_facts.get(&id) {
            for (&l, &fact) in source {
                facts.insert(offset + l, fact);
            }
        }
        *stmt = Stmt::Block {
            label: done,
            body: expansion,
        };
    }
}
