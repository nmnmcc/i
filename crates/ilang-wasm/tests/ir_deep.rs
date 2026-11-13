//! IR ownership and structural comparison must not use the host call stack.
use ilang_wasm::ir::{Atom, Callee, Cond, Env, FuncId, GlobalId, Num, NumValue, Op, Stmt, Value};
use num_bigint::BigInt;

fn small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

fn condition(depth: usize, leaf: bool) -> Cond {
    let mut value = Cond::True(Atom::Bool(leaf));
    for i in 0..depth {
        value = match i % 3 {
            0 => Cond::Not(Box::new(value)),
            1 => Cond::And(Box::new(value), Box::new(Cond::I32(i as u32))),
            _ => Cond::Or(Box::new(Cond::I32(i as u32)), Box::new(value)),
        };
    }
    value
}

fn statement(depth: usize, leaf: bool) -> Stmt {
    let mut value = Stmt::Return(Atom::Bool(leaf));
    for i in 0..depth {
        value = if i % 2 == 0 {
            Stmt::Block {
                label: i as u32,
                body: vec![Stmt::Break(i as u32), value],
            }
        } else {
            Stmt::If {
                cond: Cond::Bool(Atom::Local(i as u32), true),
                then: vec![value],
                otherwise: vec![Stmt::Unreachable],
            }
        };
    }
    value
}

#[test]
fn deep_condition_clone_equality_and_drop() {
    small_stack(|| {
        let a = condition(10_000, true);
        let copy = a.clone();
        assert!(a == copy);
        assert!(a != condition(10_000, false));
        assert!(a != condition(9_999, true));
        drop(a);
        drop(copy);
    });
}

#[test]
fn deep_statement_clone_equality_and_drop() {
    small_stack(|| {
        let a = statement(10_000, true);
        let copy = a.clone();
        assert!(a == copy);
        assert!(a != statement(10_000, false));
        assert!(a != statement(9_999, true));
        drop(a);
        drop(copy);
    });
}

#[test]
fn child_order_and_scalar_equality_are_preserved() {
    let a = Stmt::If {
        cond: Cond::And(Box::new(Cond::I32(1)), Box::new(Cond::I32(2))),
        then: vec![Stmt::Break(1), Stmt::Return(Atom::Nil)],
        otherwise: vec![Stmt::Unreachable],
    };
    assert!(a == a.clone());
    let b = Stmt::If {
        cond: Cond::And(Box::new(Cond::I32(2)), Box::new(Cond::I32(1))),
        then: vec![Stmt::Break(1), Stmt::Return(Atom::Nil)],
        otherwise: vec![Stmt::Unreachable],
    };
    assert!(a != b);
    assert!(Cond::IsFloat(Atom::Nil, f64::NAN) != Cond::IsFloat(Atom::Nil, f64::NAN));
    assert!(Cond::IsFloat(Atom::Nil, -0.0) == Cond::IsFloat(Atom::Nil, 0.0));
    let values = vec![
        Stmt::Let {
            dst: 4,
            value: Value::Op(Op::IntAdd, vec![Atom::Int(BigInt::from(1))]),
            site: Some(3),
        },
        Stmt::LetNum {
            dst: 5,
            value: NumValue::Add(Num::Const(1), Num::Local(2)),
            site: None,
        },
        Stmt::ReadUtf {
            codepoint: 1,
            width: 2,
            ok: 3,
            bits: Atom::EmptyList,
            start: Num::Const(0),
            encoding: 4,
            little: true,
        },
        Stmt::TailCall {
            callee: Callee::Direct(FuncId(7)),
            args: vec![Atom::Nil],
            env: Env::Local(8),
            site: Some(9),
        },
        Stmt::LetEnv {
            dst: 4,
            env: Env::Param,
            service: 2,
            value: Atom::Bool(true),
            in_place: false,
        },
        Stmt::SetCapture {
            closure: 1,
            index: 2,
            value: Atom::Nil,
        },
        Stmt::SetGlobal(GlobalId(3), Atom::String("value".into())),
        Stmt::Suspendable {
            dst: 2,
            value: Value::Atom(Atom::Nil),
            site: Some(1),
            point: 7,
        },
        Stmt::Continue(3),
        Stmt::Join(4),
    ];
    assert!(values == values.clone());
    let mut changed = values.clone();
    changed.swap(0, 1);
    assert!(values != changed);
}

#[test]
fn condition_leaf_forms_remain_distinct() {
    let a = Atom::String("a".into());
    let b = Atom::String("b".into());
    let n = Num::Const(3);
    let m = Num::Local(4);
    let forms = vec![
        Cond::True(a.clone()),
        Cond::Tag(a.clone(), 2),
        Cond::Bool(a.clone(), true),
        Cond::Ctor(a.clone(), 2),
        Cond::Type(a.clone(), 2),
        Cond::Tuple(a.clone(), 2),
        Cond::Empty(a.clone()),
        Cond::Equal(a.clone(), b.clone()),
        Cond::IsInt(a.clone(), BigInt::from(4)),
        Cond::IsFloat(a.clone(), 2.5),
        Cond::IsString(a.clone(), "x".into()),
        Cond::StartsWith(a.clone(), b.clone(), Atom::String(String::new())),
        Cond::I32(2),
        Cond::NumLt(n.clone(), m.clone()),
        Cond::NumEq(n.clone(), m.clone()),
        Cond::Whole(n.clone()),
        Cond::BitsHas(a.clone(), n.clone(), m.clone()),
        Cond::BitsFloat {
            bits: a.clone(),
            start: n,
            width: 32,
            little: true,
        },
        Cond::BitsEqual(a, b),
    ];
    for (i, a) in forms.iter().enumerate() {
        assert!(a == &a.clone());
        for (j, b) in forms.iter().enumerate() {
            assert_eq!(a == b, i == j, "condition variant {i} versus {j}");
        }
    }
}
