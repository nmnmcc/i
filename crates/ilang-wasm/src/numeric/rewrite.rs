//! Exhaustive operand renaming shared by the numeric passes.
use crate::ir::*;

fn num(n: &mut Num, map: &mut impl FnMut(&mut Atom)) {
    if let Num::Local(l) = n {
        let mut a = Atom::Local(*l);
        map(&mut a);
        let Atom::Local(new) = a else {
            panic!("numeric local rename changed internal number kind")
        };
        *l = new;
    }
}
pub fn env(e: &mut Env, map: &mut impl FnMut(&mut Atom)) {
    if let Env::Local(l) = e {
        let mut a = Atom::Local(*l);
        map(&mut a);
        let Atom::Local(new) = a else {
            panic!("numeric local rename changed environment kind")
        };
        *l = new;
    }
}
pub fn callee(c: &mut Callee, map: &mut impl FnMut(&mut Atom)) {
    match c {
        Callee::Value(a) | Callee::Known { closure: a, .. } | Callee::Lazy { service: a, .. } => {
            map(a)
        }
        Callee::Direct(_) => {}
    }
}
pub fn value(v: &mut Value, map: &mut impl FnMut(&mut Atom)) {
    match v {
        Value::Atom(a)
        | Value::Field(a, _)
        | Value::RecGet(a, _)
        | Value::ListHead(a)
        | Value::ListTail(a)
        | Value::StringSuffix(a, _, _)
        | Value::BitsWhole(a)
        | Value::BitsFromFloat { value: a, .. }
        | Value::BitsFromUtf { value: a, .. } => map(a),
        Value::Capture(_) | Value::BitsEmpty | Value::HostReply(_) => {}
        Value::Call {
            callee: c,
            args,
            env: e,
        } => {
            callee(c, map);
            args.iter_mut().for_each(&mut *map);
            env(e, map);
        }
        Value::HostCall { args, .. }
        | Value::HostStart { args, .. }
        | Value::Op(_, args)
        | Value::Finite(_, args)
        | Value::Tuple(args) => args.iter_mut().for_each(map),
        Value::OpEnv(_, args, e) | Value::Light { args, env: e, .. } => {
            args.iter_mut().for_each(&mut *map);
            env(e, map);
        }
        Value::Service(e, _) | Value::ServiceRaw(e, _) => env(e, map),
        Value::Closure { captures, .. } => captures.iter_mut().for_each(map),
        Value::Record { fields, .. } => fields.iter_mut().for_each(map),
        Value::RecNew(fields) => fields.iter_mut().for_each(|(_, a)| map(a)),
        Value::RecWith(a, fields) => {
            map(a);
            fields.iter_mut().for_each(|(_, a)| map(a));
        }
        Value::Cons(a, b) | Value::BitsConcat(a, b) => {
            map(a);
            map(b);
        }
        Value::BitsFromInt { value, width, .. } => {
            map(value);
            num(width, map);
        }
        Value::BitsTake(a, n) => {
            map(a);
            num(n, map);
        }
        Value::BitsReadInt {
            bits, start, width, ..
        } => {
            map(bits);
            num(start, map);
            num(width, map);
        }
        Value::BitsReadFloat { bits, start, .. } => {
            map(bits);
            num(start, map);
        }
        Value::BitsSlice {
            bits,
            start,
            length,
        } => {
            map(bits);
            num(start, map);
            num(length, map);
        }
    }
}
pub fn cond(c: &mut Cond, map: &mut impl FnMut(&mut Atom)) {
    match c {
        Cond::True(a)
        | Cond::Tag(a, _)
        | Cond::IsRec(a)
        | Cond::Bool(a, _)
        | Cond::Ctor(a, _)
        | Cond::Type(a, _)
        | Cond::Tuple(a, _)
        | Cond::Empty(a)
        | Cond::IsInt(a, _)
        | Cond::IsFloat(a, _)
        | Cond::IsString(a, _) => map(a),
        Cond::Not(c) => cond(c, map),
        Cond::And(a, b) | Cond::Or(a, b) => {
            cond(a, map);
            cond(b, map);
        }
        Cond::StartsWith(a, b, c) => {
            map(a);
            map(b);
            map(c);
        }
        Cond::Equal(a, b) | Cond::BitsEqual(a, b) => {
            map(a);
            map(b);
        }
        Cond::I32(l) => {
            let mut a = Atom::Local(*l);
            map(&mut a);
            let Atom::Local(new) = a else {
                panic!("numeric rename changed condition kind")
            };
            *l = new;
        }
        Cond::NumLt(a, b) | Cond::NumEq(a, b) => {
            num(a, map);
            num(b, map);
        }
        Cond::Whole(n) => num(n, map),
        Cond::BitsHas(a, b, c) => {
            map(a);
            num(b, map);
            num(c, map);
        }
        Cond::BitsFloat { bits, start, .. } => {
            map(bits);
            num(start, map);
        }
    }
}
pub fn operands(s: &mut Stmt, map: &mut impl FnMut(&mut Atom)) {
    match s {
        Stmt::Let { value: v, .. } | Stmt::Suspendable { value: v, .. } => value(v, map),
        Stmt::LetNum { value, .. } => match value {
            NumValue::Num(n) => num(n, map),
            NumValue::Add(a, b) | NumValue::Sub(a, b) => {
                num(a, map);
                num(b, map);
            }
            NumValue::BitsLength(a)
            | NumValue::LengthOf(a, _)
            | NumValue::TakeLength(a, _)
            | NumValue::WidthOf(a, _) => map(a),
        },
        Stmt::ReadUtf { bits, start, .. } => {
            map(bits);
            num(start, map);
        }
        Stmt::If { cond: c, .. } => cond(c, map),
        Stmt::Return(a) | Stmt::SetGlobal(_, a) => map(a),
        Stmt::TailCall {
            callee: c,
            args,
            env: e,
            ..
        } => {
            callee(c, map);
            args.iter_mut().for_each(&mut *map);
            env(e, map);
        }
        Stmt::LetEnv { env: e, value, .. } => {
            env(e, map);
            map(value);
        }
        Stmt::SetCapture { closure, value, .. } => {
            let mut a = Atom::Local(*closure);
            map(&mut a);
            let Atom::Local(l) = a else {
                panic!("numeric rename changed patched closure")
            };
            *closure = l;
            map(value);
        }
        Stmt::Block { .. }
        | Stmt::Break(_)
        | Stmt::Unreachable
        | Stmt::Continue(_)
        | Stmt::Join(_)
        | Stmt::Logical(_) => {}
    }
}
pub fn walk(stmts: &mut [Stmt], visit: &mut impl FnMut(&mut Stmt)) {
    for s in stmts {
        match s {
            Stmt::If {
                then, otherwise, ..
            } => {
                walk(then, visit);
                walk(otherwise, visit);
            }
            Stmt::Block { body, .. } => walk(body, visit),
            _ => {}
        }
        visit(s);
    }
}
pub fn size(stmts: &[Stmt]) -> usize {
    stmts
        .iter()
        .map(|s| {
            1 + match s {
                Stmt::If {
                    then, otherwise, ..
                } => size(then) + size(otherwise),
                Stmt::Block { body, .. } => size(body),
                _ => 0,
            }
        })
        .sum()
}

/// ANF arguments are already evaluated. Forward fresh, single-definition copies
/// inside an expansion, retaining its checks, captures and logical actions.
/// Joins are excluded on both sides: neither the copy nor its source may have
/// more than one definition in the expansion, so the source still holds the
/// copied value at every use of the copy.
pub fn copies(body: &mut Vec<Stmt>, first: Local) {
    let mut definitions = std::collections::BTreeMap::<Local, Vec<Option<Value>>>::new();
    walk(body, &mut |s| match s {
        Stmt::Let { dst, value, site } => definitions
            .entry(*dst)
            .or_default()
            .push(site.is_none().then(|| value.clone())),
        Stmt::LetNum { dst, .. } | Stmt::LetEnv { dst, .. } => {
            definitions.entry(*dst).or_default().push(None)
        }
        Stmt::ReadUtf {
            codepoint,
            width,
            ok,
            ..
        } => {
            for l in [*codepoint, *width, *ok] {
                definitions.entry(l).or_default().push(None);
            }
        }
        _ => {}
    });
    let single = |l: &Local| definitions.get(l).is_none_or(|v| v.len() == 1);
    let aliases = definitions
        .iter()
        .filter_map(|(&dst, values)| {
            if dst >= first
                && values.len() == 1
                && let Some(Value::Atom(a)) = &values[0]
                && *a != Atom::Local(dst)
                && !matches!(a, Atom::Local(source) if !single(source))
            {
                Some((dst, a.clone()))
            } else {
                None
            }
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    walk(body, &mut |s| {
        operands(s, &mut |a| {
            while let Atom::Local(l) = a
                && let Some(source) = aliases.get(l)
            {
                *a = source.clone();
            }
        })
    });
    fn prune(body: &mut Vec<Stmt>, aliases: &std::collections::BTreeMap<Local, Atom>) {
        for s in body.iter_mut() {
            match s {
                Stmt::Block { body, .. } => prune(body, aliases),
                Stmt::If {
                    then, otherwise, ..
                } => {
                    prune(then, aliases);
                    prune(otherwise, aliases);
                }
                _ => {}
            }
        }
        body.retain(|s| !matches!(s, Stmt::Let { dst, .. } if aliases.contains_key(dst)));
    }
    prune(body, &aliases);
}
