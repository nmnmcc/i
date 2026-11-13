//! Stack-safe ownership and structural operations for control-flow IR.
//!
//! Like Binaryen's `Walker` (wasm-traversal.h), traversals keep pending
//! operations on an explicit stack. Child order remains the source order.
use super::{Cond, Stmt};

impl Clone for Cond {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a Cond),
            Not,
            And,
            Or,
        }
        let mut todo = vec![Work::Visit(self)];
        let mut done = Vec::new();
        while let Some(item) = todo.pop() {
            match item {
                Work::Visit(c) => match c {
                    Cond::Not(a) => {
                        todo.push(Work::Not);
                        todo.push(Work::Visit(a));
                    }
                    Cond::And(a, b) => {
                        todo.push(Work::And);
                        todo.push(Work::Visit(b));
                        todo.push(Work::Visit(a));
                    }
                    Cond::Or(a, b) => {
                        todo.push(Work::Or);
                        todo.push(Work::Visit(b));
                        todo.push(Work::Visit(a));
                    }
                    Cond::IsRec(a) => done.push(Cond::IsRec(a.clone())),
                    Cond::True(a) => done.push(Cond::True(a.clone())),
                    Cond::Tag(a, n) => done.push(Cond::Tag(a.clone(), *n)),
                    Cond::Bool(a, b) => done.push(Cond::Bool(a.clone(), *b)),
                    Cond::Ctor(a, n) => done.push(Cond::Ctor(a.clone(), *n)),
                    Cond::Type(a, n) => done.push(Cond::Type(a.clone(), *n)),
                    Cond::Tuple(a, n) => done.push(Cond::Tuple(a.clone(), *n)),
                    Cond::Empty(a) => done.push(Cond::Empty(a.clone())),
                    Cond::Equal(a, b) => done.push(Cond::Equal(a.clone(), b.clone())),
                    Cond::IsInt(a, n) => done.push(Cond::IsInt(a.clone(), n.clone())),
                    Cond::IsFloat(a, n) => done.push(Cond::IsFloat(a.clone(), *n)),
                    Cond::IsString(a, n) => done.push(Cond::IsString(a.clone(), n.clone())),
                    Cond::StartsWith(a, b, c) => {
                        done.push(Cond::StartsWith(a.clone(), b.clone(), c.clone()))
                    }
                    Cond::I32(n) => done.push(Cond::I32(*n)),
                    Cond::NumLt(a, b) => done.push(Cond::NumLt(a.clone(), b.clone())),
                    Cond::NumEq(a, b) => done.push(Cond::NumEq(a.clone(), b.clone())),
                    Cond::Whole(n) => done.push(Cond::Whole(n.clone())),
                    Cond::BitsHas(a, b, c) => {
                        done.push(Cond::BitsHas(a.clone(), b.clone(), c.clone()))
                    }
                    Cond::BitsFloat {
                        bits,
                        start,
                        width,
                        little,
                    } => done.push(Cond::BitsFloat {
                        bits: bits.clone(),
                        start: start.clone(),
                        width: *width,
                        little: *little,
                    }),
                    Cond::BitsEqual(a, b) => done.push(Cond::BitsEqual(a.clone(), b.clone())),
                },
                Work::Not => {
                    let a = done.pop().expect("condition clone child");
                    done.push(Cond::Not(Box::new(a)));
                }
                Work::And | Work::Or => {
                    let b = done.pop().expect("condition clone right");
                    let a = done.pop().expect("condition clone left");
                    done.push(if matches!(item, Work::And) {
                        Cond::And(Box::new(a), Box::new(b))
                    } else {
                        Cond::Or(Box::new(a), Box::new(b))
                    });
                }
            }
        }
        assert_eq!(done.len(), 1, "condition clone result count");
        done.pop().unwrap()
    }
}

impl PartialEq for Cond {
    fn eq(&self, other: &Self) -> bool {
        let mut todo = vec![(self, other)];
        while let Some((a, b)) = todo.pop() {
            match (a, b) {
                (Cond::StartsWith(a, b, c), Cond::StartsWith(x, y, z)) => {
                    if a != x || b != y || c != z {
                        return false;
                    }
                }
                (Cond::Not(a), Cond::Not(b)) => todo.push((a, b)),
                (Cond::And(a, b), Cond::And(c, d)) | (Cond::Or(a, b), Cond::Or(c, d)) => {
                    todo.push((b, d));
                    todo.push((a, c));
                }
                (Cond::True(a), Cond::True(b)) | (Cond::Empty(a), Cond::Empty(b)) => {
                    if a != b {
                        return false;
                    }
                }
                (Cond::Tag(a, n), Cond::Tag(b, m))
                | (Cond::Ctor(a, n), Cond::Ctor(b, m))
                | (Cond::Type(a, n), Cond::Type(b, m))
                | (Cond::Tuple(a, n), Cond::Tuple(b, m)) => {
                    if a != b || n != m {
                        return false;
                    }
                }
                (Cond::Bool(a, n), Cond::Bool(b, m)) => {
                    if a != b || n != m {
                        return false;
                    }
                }
                (Cond::Equal(a, b), Cond::Equal(c, d))
                | (Cond::BitsEqual(a, b), Cond::BitsEqual(c, d)) => {
                    if a != c || b != d {
                        return false;
                    }
                }
                (Cond::IsInt(a, n), Cond::IsInt(b, m)) => {
                    if a != b || n != m {
                        return false;
                    }
                }
                (Cond::IsFloat(a, n), Cond::IsFloat(b, m)) => {
                    if a != b || n != m {
                        return false;
                    }
                }
                (Cond::IsString(a, n), Cond::IsString(b, m)) => {
                    if a != b || n != m {
                        return false;
                    }
                }
                (Cond::I32(a), Cond::I32(b)) => {
                    if a != b {
                        return false;
                    }
                }
                (Cond::NumLt(a, b), Cond::NumLt(c, d)) | (Cond::NumEq(a, b), Cond::NumEq(c, d)) => {
                    if a != c || b != d {
                        return false;
                    }
                }
                (Cond::Whole(a), Cond::Whole(b)) => {
                    if a != b {
                        return false;
                    }
                }
                (Cond::BitsHas(a, b, c), Cond::BitsHas(d, e, f)) => {
                    if a != d || b != e || c != f {
                        return false;
                    }
                }
                (
                    Cond::BitsFloat {
                        bits: a,
                        start: b,
                        width: c,
                        little: d,
                    },
                    Cond::BitsFloat {
                        bits: e,
                        start: f,
                        width: g,
                        little: h,
                    },
                ) => {
                    if a != e || b != f || c != g || d != h {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }
}

impl Drop for Cond {
    fn drop(&mut self) {
        fn detach(node: &mut Cond, pending: &mut Vec<Cond>) {
            match node {
                Cond::Not(a) => pending.push(std::mem::replace(a.as_mut(), Cond::I32(0))),
                Cond::And(a, b) | Cond::Or(a, b) => {
                    pending.push(std::mem::replace(a.as_mut(), Cond::I32(0)));
                    pending.push(std::mem::replace(b.as_mut(), Cond::I32(0)));
                }
                _ => {}
            }
        }
        if !matches!(self, Cond::Not(_) | Cond::And(_, _) | Cond::Or(_, _)) {
            return;
        }
        let mut pending = Vec::new();
        detach(self, &mut pending);
        while let Some(mut child) = pending.pop() {
            detach(&mut child, &mut pending);
        }
    }
}

impl Clone for Stmt {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a Stmt),
            If(Cond, usize, usize),
            Block(u32, usize),
        }
        let mut todo = vec![Work::Visit(self)];
        let mut done = Vec::new();
        while let Some(item) = todo.pop() {
            match item {
                Work::Visit(s) => match s {
                    Stmt::If {
                        cond,
                        then,
                        otherwise,
                    } => {
                        todo.push(Work::If(cond.clone(), then.len(), otherwise.len()));
                        for s in otherwise.iter().rev() {
                            todo.push(Work::Visit(s));
                        }
                        for s in then.iter().rev() {
                            todo.push(Work::Visit(s));
                        }
                    }
                    Stmt::Block { label, body } => {
                        todo.push(Work::Block(*label, body.len()));
                        for s in body.iter().rev() {
                            todo.push(Work::Visit(s));
                        }
                    }
                    Stmt::Logical(action) => done.push(Stmt::Logical(*action)),
                    Stmt::Let { dst, value, site } => done.push(Stmt::Let {
                        dst: *dst,
                        value: value.clone(),
                        site: *site,
                    }),
                    Stmt::LetNum { dst, value, site } => done.push(Stmt::LetNum {
                        dst: *dst,
                        value: value.clone(),
                        site: *site,
                    }),
                    Stmt::ReadUtf {
                        codepoint,
                        width,
                        ok,
                        bits,
                        start,
                        encoding,
                        little,
                    } => done.push(Stmt::ReadUtf {
                        codepoint: *codepoint,
                        width: *width,
                        ok: *ok,
                        bits: bits.clone(),
                        start: start.clone(),
                        encoding: *encoding,
                        little: *little,
                    }),
                    Stmt::Break(n) => done.push(Stmt::Break(*n)),
                    Stmt::Return(a) => done.push(Stmt::Return(a.clone())),
                    Stmt::TailCall {
                        callee,
                        args,
                        env,
                        site,
                    } => done.push(Stmt::TailCall {
                        callee: callee.clone(),
                        args: args.clone(),
                        env: *env,
                        site: *site,
                    }),
                    Stmt::LetEnv {
                        dst,
                        env,
                        service,
                        value,
                        in_place,
                    } => done.push(Stmt::LetEnv {
                        dst: *dst,
                        env: *env,
                        service: *service,
                        value: value.clone(),
                        in_place: *in_place,
                    }),
                    Stmt::Unreachable => done.push(Stmt::Unreachable),
                    Stmt::SetCapture {
                        closure,
                        index,
                        value,
                    } => done.push(Stmt::SetCapture {
                        closure: *closure,
                        index: *index,
                        value: value.clone(),
                    }),
                    Stmt::SetGlobal(id, a) => done.push(Stmt::SetGlobal(*id, a.clone())),
                    Stmt::Suspendable {
                        dst,
                        value,
                        site,
                        point,
                    } => done.push(Stmt::Suspendable {
                        dst: *dst,
                        value: value.clone(),
                        site: *site,
                        point: *point,
                    }),
                    Stmt::Continue(n) => done.push(Stmt::Continue(*n)),
                    Stmt::Join(n) => done.push(Stmt::Join(*n)),
                },
                Work::If(cond, n, m) => {
                    let i = done.len().checked_sub(n + m).expect("if clone child count");
                    let otherwise = done.split_off(i + n);
                    let then = done.split_off(i);
                    done.push(Stmt::If {
                        cond,
                        then,
                        otherwise,
                    });
                }
                Work::Block(label, n) => {
                    let i = done.len().checked_sub(n).expect("block clone child count");
                    let body = done.split_off(i);
                    done.push(Stmt::Block { label, body });
                }
            }
        }
        assert_eq!(done.len(), 1, "statement clone result count");
        done.pop().unwrap()
    }
}

impl PartialEq for Stmt {
    fn eq(&self, other: &Self) -> bool {
        let mut todo = vec![(self, other)];
        while let Some((a, b)) = todo.pop() {
            match (a, b) {
                (
                    Stmt::If {
                        cond: a,
                        then: b,
                        otherwise: c,
                    },
                    Stmt::If {
                        cond: d,
                        then: e,
                        otherwise: f,
                    },
                ) => {
                    if a != d || b.len() != e.len() || c.len() != f.len() {
                        return false;
                    }
                    todo.extend(b.iter().zip(e));
                    todo.extend(c.iter().zip(f));
                }
                (Stmt::Block { label: a, body: b }, Stmt::Block { label: c, body: d }) => {
                    if a != c || b.len() != d.len() {
                        return false;
                    }
                    todo.extend(b.iter().zip(d));
                }
                (
                    Stmt::Let {
                        dst: a,
                        value: b,
                        site: c,
                    },
                    Stmt::Let {
                        dst: d,
                        value: e,
                        site: f,
                    },
                ) => {
                    if a != d || b != e || c != f {
                        return false;
                    }
                }
                (
                    Stmt::LetNum {
                        dst: a,
                        value: b,
                        site: c,
                    },
                    Stmt::LetNum {
                        dst: d,
                        value: e,
                        site: f,
                    },
                ) => {
                    if a != d || b != e || c != f {
                        return false;
                    }
                }
                (
                    Stmt::ReadUtf {
                        codepoint: a,
                        width: b,
                        ok: c,
                        bits: d,
                        start: e,
                        encoding: f,
                        little: g,
                    },
                    Stmt::ReadUtf {
                        codepoint: h,
                        width: i,
                        ok: j,
                        bits: k,
                        start: l,
                        encoding: m,
                        little: n,
                    },
                ) => {
                    if a != h || b != i || c != j || d != k || e != l || f != m || g != n {
                        return false;
                    }
                }
                (Stmt::Break(a), Stmt::Break(b))
                | (Stmt::Continue(a), Stmt::Continue(b))
                | (Stmt::Join(a), Stmt::Join(b)) => {
                    if a != b {
                        return false;
                    }
                }
                (Stmt::Return(a), Stmt::Return(b)) => {
                    if a != b {
                        return false;
                    }
                }
                (
                    Stmt::TailCall {
                        callee: a,
                        args: b,
                        env: c,
                        site: d,
                    },
                    Stmt::TailCall {
                        callee: e,
                        args: f,
                        env: g,
                        site: h,
                    },
                ) => {
                    if a != e || b != f || c != g || d != h {
                        return false;
                    }
                }
                (
                    Stmt::LetEnv {
                        dst: a,
                        env: b,
                        service: c,
                        value: d,
                        in_place: i,
                    },
                    Stmt::LetEnv {
                        dst: e,
                        env: f,
                        service: g,
                        value: h,
                        in_place: j,
                    },
                ) => {
                    if a != e || b != f || c != g || d != h || i != j {
                        return false;
                    }
                }
                (Stmt::Logical(a), Stmt::Logical(b)) => {
                    if a != b {
                        return false;
                    }
                }
                (Stmt::Unreachable, Stmt::Unreachable) => {}
                (
                    Stmt::SetCapture {
                        closure: a,
                        index: b,
                        value: c,
                    },
                    Stmt::SetCapture {
                        closure: d,
                        index: e,
                        value: f,
                    },
                ) => {
                    if a != d || b != e || c != f {
                        return false;
                    }
                }
                (Stmt::SetGlobal(a, b), Stmt::SetGlobal(c, d)) => {
                    if a != c || b != d {
                        return false;
                    }
                }
                (
                    Stmt::Suspendable {
                        dst: a,
                        value: b,
                        site: c,
                        point: d,
                    },
                    Stmt::Suspendable {
                        dst: e,
                        value: f,
                        site: g,
                        point: h,
                    },
                ) => {
                    if a != e || b != f || c != g || d != h {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }
}

impl Drop for Stmt {
    fn drop(&mut self) {
        fn detach(s: &mut Stmt, pending: &mut Vec<Stmt>) {
            match s {
                Stmt::If {
                    then, otherwise, ..
                } => {
                    pending.append(then);
                    pending.append(otherwise);
                }
                Stmt::Block { body, .. } => pending.append(body),
                _ => {}
            }
        }
        if !matches!(self, Stmt::If { .. } | Stmt::Block { .. }) {
            return;
        }
        let mut pending = Vec::new();
        detach(self, &mut pending);
        while let Some(mut child) = pending.pop() {
            detach(&mut child, &mut pending);
        }
    }
}
