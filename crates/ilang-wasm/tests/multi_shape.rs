//! Multi-shape function values at run time (syntax-3.md §2.6.5, §6.4): a
//! `$multi` holds one ordinary closure per shape group, and every indirect
//! call site dispatches on the shape of its argument data when the callee is
//! not a closure of its arity. The programs here are written directly in the
//! intermediate form, the way the lowering makes them.

use ilang_wasm::control;
use ilang_wasm::emit;
use ilang_wasm::ir::*;
use num_bigint::BigInt;

fn int(n: i64) -> Atom {
    Atom::Int(BigInt::from(n))
}

/// A small builder of one function of the intermediate form.
struct F {
    arity: u32,
    locals: Vec<LocalType>,
    body: Vec<Stmt>,
}

impl F {
    fn new(arity: u32) -> Self {
        F {
            arity,
            locals: vec![LocalType::Value; arity as usize],
            body: Vec::new(),
        }
    }
    fn local(&mut self) -> Local {
        self.locals.push(LocalType::Value);
        self.locals.len() as Local - 1
    }
    fn let_(&mut self, value: Value) -> Atom {
        let dst = self.local();
        self.body.push(Stmt::Let {
            dst,
            value,
            site: None,
        });
        Atom::Local(dst)
    }
    fn call(&mut self, callee: Callee, args: Vec<Atom>) -> Atom {
        self.let_(Value::Call {
            callee,
            args,
            env: Env::Param,
        })
    }
    /// Returns `False` from the function unless `v == expected`.
    fn check(&mut self, v: Atom, expected: Atom) {
        let same = self.let_(Value::Op(Op::Equal, vec![v, expected]));
        self.body.push(Stmt::If {
            cond: Cond::True(same),
            then: vec![],
            otherwise: vec![Stmt::Return(Atom::Bool(false))],
        });
    }
    fn done(mut self, name: &str, returns: Option<Atom>) -> FuncKind {
        self.body
            .push(Stmt::Return(returns.unwrap_or(Atom::Bool(true))));
        FuncKind::Code(Func {
            name: name.into(),
            arity: self.arity,
            budget_frame: self.locals.len() as u32,
            locals: self.locals,
            body: self.body,
            frame: true,
        })
    }
}

/// The test program: its checks are exports of no parameters that give
/// `True`, in [`CHECKS`], and [`MISSES`] stop.
struct Built {
    program: Program,
}

const CHECKS: &[&str] = &[
    "call",
    "fast",
    "labelled",
    "tail",
    "tail_fast",
    "suspend",
    "suspend_handler",
    "boundary_body",
    "catch_ensure",
    "keep",
    "equal",
    "inspect",
];
const MISSES: &[&str] = &["miss", "miss_tail", "miss_single"];

fn build(numeric: ilang_wasm::Numeric) -> Built {
    let mut p = Program {
        numeric,
        services: 1,
        ..Program::default()
    };
    let port = p.shape(1, &["port".to_string()]);
    let mut funcs: Vec<FuncKind> = Vec::new();
    let mut add = |f: FuncKind| {
        funcs.push(f);
        FuncId(funcs.len() as u32 - 1)
    };

    // Groups of the multi-shape value `m` (global 0).
    // (x) { x + 1 }
    let mut f = F::new(1);
    let r = f.let_(Value::Op(Op::IntAdd, vec![Atom::Local(0), int(1)]));
    let one = add(f.done("one", Some(r)));
    // (x, y) { x * y + c }, c captured
    let mut f = F::new(2);
    let c = f.let_(Value::Capture(0));
    let r = f.let_(Value::Op(Op::IntMul, vec![Atom::Local(0), Atom::Local(1)]));
    let r = f.let_(Value::Op(Op::IntAdd, vec![r, c]));
    let two = add(f.done("two", Some(r)));
    // (x, port = p) { x - p }
    let mut f = F::new(2);
    let r = f.let_(Value::Op(Op::IntSub, vec![Atom::Local(0), Atom::Local(1)]));
    let labelled = add(f.done("labelled", Some(r)));
    // () { 7 }
    let zero = add(F::new(0).done("zero", Some(int(7))));
    // A tail loop through the multi-shape value of global 1:
    // (n, acc) { n == 0 => acc; g1(n - 1, acc + 2) }
    let mut f = F::new(2);
    let at_end = f.let_(Value::Op(Op::Equal, vec![Atom::Local(0), int(0)]));
    let n1 = f.let_(Value::Op(Op::IntSub, vec![Atom::Local(0), int(1)]));
    let a2 = f.let_(Value::Op(Op::IntAdd, vec![Atom::Local(1), int(2)]));
    f.body.push(Stmt::If {
        cond: Cond::True(at_end),
        then: vec![Stmt::Return(Atom::Local(1))],
        otherwise: vec![Stmt::TailCall {
            callee: Callee::Value(Atom::Global(GlobalId(1))),
            args: vec![n1, a2],
            env: Env::Param,
            site: None,
        }],
    });
    let looping = add(f.done("loop", None));
    // The same loop through an ordinary closure (global 2).
    let mut f = F::new(2);
    let at_end = f.let_(Value::Op(Op::Equal, vec![Atom::Local(0), int(0)]));
    let n1 = f.let_(Value::Op(Op::IntSub, vec![Atom::Local(0), int(1)]));
    let a2 = f.let_(Value::Op(Op::IntAdd, vec![Atom::Local(1), int(2)]));
    f.body.push(Stmt::If {
        cond: Cond::True(at_end),
        then: vec![Stmt::Return(Atom::Local(1))],
        otherwise: vec![Stmt::TailCall {
            callee: Callee::Value(Atom::Global(GlobalId(2))),
            args: vec![n1, a2],
            env: Env::Param,
            site: None,
        }],
    });
    let looping_fast = add(f.done("loop_fast", None));

    // Control: the handler `(k) { control.resume(k, 10) }`.
    let mut f = F::new(1);
    let r = f.let_(Value::OpEnv(
        Op::Resume,
        vec![Atom::Local(0), int(10)],
        Env::Param,
    ));
    let handler = add(f.done("handler", Some(r)));
    // The shape-1 group that suspends: `(b) { control.suspend(b, handler) + 1 }`,
    // with the handler as global 4 (a multi-shape value with it as its
    // one-position group) or as a constant closure.
    let suspender = |via_multi: bool, name: &str| {
        let mut f = F::new(1);
        let h = if via_multi {
            Atom::Global(GlobalId(4))
        } else {
            Atom::Func(handler)
        };
        let v = f.let_(Value::Op(Op::Suspend, vec![Atom::Local(0), h]));
        let r = f.let_(Value::Op(Op::IntAdd, vec![v, int(1)]));
        f.done(name, Some(r))
    };
    let suspends = add(suspender(false, "suspends"));
    let suspends_h = add(suspender(true, "suspends_via_multi_handler"));
    // The boundary body: `(b) { g3(b) * 2 }`, a call through the multi-shape
    // value of global 3 across the suspension.
    let mut f = F::new(1);
    let v = f.call(
        Callee::Value(Atom::Global(GlobalId(3))),
        vec![Atom::Local(0)],
    );
    let r = f.let_(Value::Op(Op::IntMul, vec![v, int(2)]));
    let body = add(f.done("body", Some(r)));

    // The start: make the values of the globals.
    let mut s = F::new(0);
    let c_one = s.let_(Value::Closure {
        func: one,
        captures: vec![],
        patched: false,
    });
    let c_two = s.let_(Value::Closure {
        func: two,
        captures: vec![int(100)],
        patched: false,
    });
    let c_lab = s.let_(Value::Closure {
        func: labelled,
        captures: vec![],
        patched: false,
    });
    let c_zero = s.let_(Value::Closure {
        func: zero,
        captures: vec![],
        patched: false,
    });
    let m = s.let_(Value::Multi(vec![
        (1, c_one.clone()),
        (2, c_two.clone()),
        (port, c_lab.clone()),
        (0, c_zero.clone()),
    ]));
    s.body.push(Stmt::SetGlobal(GlobalId(0), m));
    let m2 = s.let_(Value::Multi(vec![
        (1, c_one.clone()),
        (2, Atom::Func(looping)),
    ]));
    s.body.push(Stmt::SetGlobal(GlobalId(1), m2));
    s.body
        .push(Stmt::SetGlobal(GlobalId(2), Atom::Func(looping_fast)));
    let m3 = s.let_(Value::Multi(vec![
        (2, c_two.clone()),
        (1, Atom::Func(suspends)),
    ]));
    s.body.push(Stmt::SetGlobal(GlobalId(3), m3));
    let m4 = s.let_(Value::Multi(vec![
        (0, c_zero.clone()),
        (1, Atom::Func(handler)),
    ]));
    s.body.push(Stmt::SetGlobal(GlobalId(4), m4));
    let m5 = s.let_(Value::Multi(vec![
        (2, c_two.clone()),
        (1, Atom::Func(suspends_h)),
    ]));
    s.body.push(Stmt::SetGlobal(GlobalId(5), m5));
    let m6 = s.let_(Value::Multi(vec![
        (2, c_two.clone()),
        (1, Atom::Func(body)),
    ]));
    s.body.push(Stmt::SetGlobal(GlobalId(6), m6));
    // A second value with the same groups as global 0.
    let m7 = s.let_(Value::Multi(vec![
        (1, c_one.clone()),
        (2, c_two.clone()),
        (port, c_lab.clone()),
        (0, c_zero.clone()),
    ]));
    s.body.push(Stmt::SetGlobal(GlobalId(7), m7));
    s.body.push(Stmt::SetGlobal(GlobalId(8), c_lab));
    let FuncKind::Code(start) = s.done("start", Some(Atom::Nil)) else {
        unreachable!()
    };

    let g = |i: u32| Atom::Global(GlobalId(i));
    let mut checks: Vec<(&str, FuncKind)> = Vec::new();

    // Ordinary calls of every shape, including a labelled one.
    let mut f = F::new(0);
    let r = f.call(Callee::Value(g(0)), vec![int(41)]);
    f.check(r, int(42));
    let r = f.call(Callee::Value(g(0)), vec![int(6), int(7)]);
    f.check(r, int(142));
    let r = f.call(Callee::Shaped(g(0), port), vec![int(10), int(3)]);
    f.check(r, int(7));
    let r = f.call(Callee::Value(g(0)), vec![]);
    f.check(r, int(7));
    checks.push(("call", f.done("call", None)));

    // Single-shape closures still take the fast path.
    let mut f = F::new(0);
    let r = f.call(Callee::Value(Atom::Func(one)), vec![int(1)]);
    f.check(r, int(2));
    checks.push(("fast", f.done("fast", None)));

    // A single-shape closure of a labelled shape.
    let mut f = F::new(0);
    let r = f.call(Callee::Shaped(g(8), port), vec![int(10), int(4)]);
    f.check(r, int(6));
    checks.push(("labelled", f.done("labelled_check", None)));

    // 10^6 tail calls through a multi-shape value, and through a closure.
    for (name, global) in [("tail", 1), ("tail_fast", 2)] {
        let mut f = F::new(0);
        let r = f.call(Callee::Value(g(global)), vec![int(1_000_000), int(0)]);
        f.check(r, int(2_000_000));
        checks.push((name, f.done(name, None)));
    }

    // A call through a multi-shape value across `control.suspend`: the
    // boundary body calls global 3, whose shape-1 group suspends; the
    // handler resumes with 10, so the body gives (10 + 1) * 2.
    let mut f = F::new(0);
    let r = f.let_(Value::OpEnv(
        Op::Boundary,
        vec![Atom::Func(body)],
        Env::Param,
    ));
    f.check(r, int(22));
    checks.push(("suspend", f.done("suspend", None)));

    // The handler given to `suspend` is a multi-shape value (the
    // `return_call_ref` of `$boundary_core`); the body calls global 5.
    let mut f2 = F::new(1);
    let v = f2.call(Callee::Value(g(5)), vec![Atom::Local(0)]);
    let r = f2.let_(Value::Op(Op::IntMul, vec![v, int(2)]));
    let body5 = f2.done("body5", Some(r));
    checks.push(("body5", body5));
    // (the check itself is added below, once the id of body5 is known)

    // The boundary body itself is a multi-shape value (`$call1`).
    let mut f = F::new(0);
    let r = f.let_(Value::OpEnv(Op::Boundary, vec![g(6)], Env::Param));
    f.check(r, int(22));
    checks.push(("boundary_body", f.done("boundary_body", None)));

    // `fail.catch` and `resource.ensure` of multi-shape values (`$call0`).
    let mut f = F::new(0);
    let r = f.let_(Value::OpEnv(Op::Catch, vec![g(4)], Env::Param));
    let v = f.let_(Value::Field(r, 0));
    f.check(v, int(7));
    let r = f.let_(Value::OpEnv(Op::Ensure, vec![g(4), g(4)], Env::Param));
    f.check(r, int(7));
    checks.push(("catch_ensure", f.done("catch_ensure", None)));

    // A function kept with its environment, around a multi-shape value.
    let mut f = F::new(0);
    let k = f.let_(Value::OpEnv(Op::KeepEnv, vec![g(0)], Env::Param));
    let r = f.call(Callee::Value(k), vec![int(5)]);
    f.check(r, int(6));
    checks.push(("keep", f.done("keep", None)));

    // A function equals only itself.
    let mut f = F::new(0);
    let same = f.let_(Value::Op(Op::Equal, vec![g(0), g(0)]));
    f.check(same, Atom::Bool(true));
    let other = f.let_(Value::Op(Op::Equal, vec![g(0), g(7)]));
    f.check(other, Atom::Bool(false));
    let group = f.let_(Value::Op(Op::Equal, vec![g(0), Atom::Func(one)]));
    f.check(group, Atom::Bool(false));
    let differs = f.let_(Value::Op(Op::NotEqual, vec![g(7), g(0)]));
    f.check(differs, Atom::Bool(true));
    checks.push(("equal", f.done("equal", None)));

    // Text form.
    let mut f = F::new(0);
    let inspect = Op::Runtime {
        name: "value_inspect",
        arity: 1,
    };
    let t = f.let_(Value::Op(inspect, vec![g(0)]));
    f.check(
        t,
        Atom::String(
            "//fn(a) { ... } | (a, b) { ... } | (a, port = b) { ... } | () { ... }".into(),
        ),
    );
    let t = f.let_(Value::Op(inspect, vec![g(8)]));
    f.check(t, Atom::String("//fn(a, port = b) { ... }".into()));
    let t = f.let_(Value::Op(inspect, vec![Atom::Func(two)]));
    f.check(t, Atom::String("//fn(a, b) { ... }".into()));
    checks.push(("inspect", f.done("inspect", None)));

    // Stops: no group of the shape, in a call and a tail call, and a
    // single-shape closure of another arity.
    let mut f = F::new(0);
    f.call(Callee::Value(g(0)), vec![int(1), int(2), int(3)]);
    checks.push(("miss", f.done("miss", None)));
    let mut f = F::new(0);
    f.body.push(Stmt::TailCall {
        callee: Callee::Value(g(1)),
        args: vec![],
        env: Env::Param,
        site: None,
    });
    checks.push(("miss_tail", f.done("miss_tail", None)));
    let mut f = F::new(0);
    f.call(Callee::Value(Atom::Func(one)), vec![int(1), int(2)]);
    checks.push(("miss_single", f.done("miss_single", None)));

    let base = funcs.len() as u32;
    let mut body5 = None;
    for (i, (name, func)) in checks.into_iter().enumerate() {
        let id = FuncId(base + i as u32);
        funcs.push(func);
        if name == "body5" {
            body5 = Some(id);
            continue;
        }
        p.exports.push(Export {
            name: name.into(),
            callee: Callee::Direct(id),
            abi: Some(ExportAbi {
                params: vec![],
                result: AbiType::Bool,
                errors: vec![],
            }),
            site: None,
            async_body: None,
        });
    }
    let body5 = body5.unwrap();
    let mut f = F::new(0);
    let r = f.let_(Value::OpEnv(
        Op::Boundary,
        vec![Atom::Func(body5)],
        Env::Param,
    ));
    f.check(r, int(22));
    funcs.push(f.done("suspend_handler", None));
    p.exports.push(Export {
        name: "suspend_handler".into(),
        callee: Callee::Direct(FuncId(funcs.len() as u32 - 1)),
        abi: Some(ExportAbi {
            params: vec![],
            result: AbiType::Bool,
            errors: vec![],
        }),
        site: None,
        async_body: None,
    });

    p.funcs = funcs;
    p.func_shapes.insert(labelled, port);
    p.constants = vec![
        one,
        two,
        looping,
        looping_fast,
        handler,
        suspends,
        suspends_h,
        body,
        body5,
    ];
    p.globals = 9;
    p.start = Some(start);
    control::split(&mut p);
    Built { program: p }
}

fn wasmtime() -> std::ffi::OsString {
    std::env::var_os("WASMTIME").unwrap_or_else(|| "wasmtime".into())
}

/// Runs `export` of the module with a driver that calls `ilang_start` and
/// stops unless the export gives 1.
fn run(guest: &[u8], export: &str) -> std::process::Output {
    let directory = tempfile::tempdir().unwrap();
    let guest_path = directory.path().join("guest.wasm");
    let host = directory.path().join("host.wasm");
    let driver = directory.path().join("driver.wasm");
    std::fs::write(&guest_path, guest).unwrap();
    let host_wat = r#"(module
        (func (export "panic") (param i32 i32) unreachable)
        (func (export "reference_id") (param externref) (result i64) (i64.const 0))
        (func (export "mc_publish") (param i32 i32 i32) (result i64) (i64.const 0))
        (func (export "mc_claim") (param i64) (result i32) (i32.const 1))
        (func (export "mc_adopt") (param i64 i64))
        (func (export "mc_send") (param i64 i32 i32))
        (func (export "mc_inbox") (result i64) (i64.const 0))
        (func (export "mc_message") (result i64) (i64.const 0))
        (func (export "mc_pending") (result i32) (i32.const 0))
        (func (export "mc_wait"))
        (func (export "mc_share_put") (param i64 i64 i64 i32 i32))
        (func (export "mc_share_get") (param i64 i64 i64) (result i64) (i64.const 0))
        (func (export "mc_share_drop") (param i64 i64 i64)))"#;
    std::fs::write(&host, wat::parse_str(host_wat).unwrap()).unwrap();
    let driver_wat = format!(
        r#"(module
        (import "guest" "ilang_start" (func $start))
        (import "guest" "{export}" (func $check (result i32)))
        (func (export "_start")
          (call $start)
          (if (i32.ne (call $check) (i32.const 1)) (then unreachable))))"#
    );
    std::fs::write(&driver, wat::parse_str(driver_wat).unwrap()).unwrap();
    std::process::Command::new(wasmtime())
        .args([
            "run",
            "-W",
            "gc=y",
            "-W",
            "exceptions=y",
            "-W",
            "tail-call=y",
            "--preload",
        ])
        .arg(format!("ilang:host/v1={}", host.display()))
        .arg("--preload")
        .arg(format!("guest={}", guest_path.display()))
        .arg(&driver)
        .output()
        .expect("wasmtime is required; run inside Devenv or set WASMTIME")
}

fn module(numeric: ilang_wasm::Numeric) -> (String, Vec<u8>) {
    let built = build(numeric);
    let text = emit::wat(&built.program);
    let wasm = wat::parse_str(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    ilang_wasm::validate(&wasm).unwrap_or_else(|e| panic!("{e}"));
    (text, wasm)
}

#[test]
fn multi_shape_values_run_at_every_call_site() {
    for numeric in [ilang_wasm::Numeric::Reference, ilang_wasm::Numeric::Scalars] {
        let (_, wasm) = module(numeric);
        for check in CHECKS {
            let out = run(&wasm, check);
            assert!(
                out.status.success(),
                "{numeric:?} {check}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        for miss in MISSES {
            let out = run(&wasm, miss);
            assert!(!out.status.success(), "{numeric:?} {miss} should stop");
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                stderr.contains("unreachable") || stderr.contains("cast"),
                "{numeric:?} {miss}: {stderr}"
            );
        }
    }
}

/// The fast path of a closure call is one type test: the closure is cast
/// once, and the dispatcher is reached only when that test fails.
#[test]
fn closure_calls_test_the_closure_once() {
    let (text, _) = module(ilang_wasm::Numeric::Reference);
    let fast = text
        .split("\n;; fast\n")
        .nth(1)
        .and_then(|rest| rest.split("\n;; ").next())
        .expect("the function `fast`");
    assert_eq!(fast.matches("br_on_cast_fail").count(), 1, "{fast}");
    assert_eq!(fast.matches("ref.cast (ref $clo1)").count(), 0, "{fast}");
    assert!(fast.contains("call $dispatch1"), "{fast}");
    // Tail sites keep no frame: the slow path is a `return_call` of the
    // dispatcher, which ends in `return_call_ref`.
    let tail = text
        .split("\n;; loop\n")
        .nth(1)
        .and_then(|rest| rest.split("\n;; ").next())
        .expect("the function `loop`");
    assert!(tail.contains("return_call $dispatch2"), "{tail}");
    assert!(tail.contains("return_call_ref $fn2"), "{tail}");
    let dispatch = text
        .split("(func $dispatch2 ")
        .nth(1)
        .expect("the dispatcher of arity 2");
    let dispatch = &dispatch[..dispatch.find("\n(func ").unwrap_or(dispatch.len())];
    assert!(dispatch.contains("return_call_ref $fn2"), "{dispatch}");
    assert!(!dispatch.contains("(call_ref"), "{dispatch}");
}

/// Worker copies preserve both function identity and every group's full shape.
#[test]
fn worker_codec_preserves_multi_shape_values() {
    let mut p = build(ilang_wasm::Numeric::Reference).program;
    p.funcs.push(FuncKind::Runtime {
        op: Op::Runtime { name: "worker_enabled", arity: 0 },
        arity: 0,
    });
    let mut text = emit::wat(&p);
    let end = text.rfind(')').unwrap();
    text.insert_str(end, r#"
      (func (export "codec_roundtrip") (result i32)
        (local $copy eqref)
        (if (i32.eqz (call $codec_encode (global.get $g0))) (then unreachable))
        (local.set $copy (call $codec_decode (global.get $enc_ptr) (global.get $enc_len)))
        (if (i32.eqz (call $closure_same (global.get $g0) (local.get $copy))) (then unreachable))
        (if (i32.eqz (call $equal
          (call $dispatch1 (local.get $copy) (i64.const 1) (global.get $root_env) (ref.i31 (i32.const 41)))
          (ref.i31 (i32.const 42)))) (then unreachable))
        (if (i32.eqz (call $equal
          (call $dispatch2 (local.get $copy) (i64.const 2) (global.get $root_env)
            (ref.i31 (i32.const 6)) (ref.i31 (i32.const 7)))
          (ref.i31 (i32.const 142)))) (then unreachable))
        (if (i32.eqz (call $equal
          (call $dispatch2 (local.get $copy)
            (struct.get $obj 2 (ref.cast (ref $obj) (global.get $g8))) (global.get $root_env)
            (ref.i31 (i32.const 10)) (ref.i31 (i32.const 3)))
          (ref.i31 (i32.const 7)))) (then unreachable))
        (if (i32.eqz (call $equal (call $inspect (local.get $copy)) (call $inspect (global.get $g0))))
          (then unreachable))
        (i32.const 1))
    "#);
    let wasm = wat::parse_str(&text).unwrap();
    ilang_wasm::validate(&wasm).unwrap();
    let out = run(&wasm, "codec_roundtrip");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
