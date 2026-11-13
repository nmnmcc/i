//! O0, O2 and O3 must agree. O0 is the reference lowering oracle.
//! Compare full language stop messages
//! (including every logical source position), values, bits and final effects.
use ilang_host::{HostImports, HostOptions, Outcome, Program, Value};
use ilang_wasm::{Numeric, Options};
use num_bigint::BigInt;
use std::sync::{Arc, Mutex};

struct Case {
    body: String,
    input: Value,
    expected: String,
    stop: bool,
    events: Vec<String>,
}

fn int_case(body: impl Into<String>, input: BigInt, expected: BigInt) -> Case {
    Case {
        body: format!("core.inspect({})", body.into()),
        input: Value::Int(input),
        expected: expected.to_string(),
        stop: false,
        events: vec![],
    }
}
fn float_case(body: &str, input: f64, expected: f64) -> Case {
    Case {
        body: format!("core.inspect(core.float_to_bits({body}))"),
        input: Value::Float(input),
        expected: expected.to_bits().to_string(),
        stop: false,
        events: vec![],
    }
}

fn corpus() -> Vec<Case> {
    let mut cases = Vec::new();
    // Both i31 boundaries and both i64 boundaries, at every integer operator.
    // Remainder cannot overflow its operands, so include large inputs as well.
    for text in [
        "1073741823",
        "-1073741824",
        "9223372036854775807",
        "-9223372036854775808",
        "18446744073709551616",
        "-18446744073709551616",
    ] {
        let a: BigInt = text.parse().unwrap();
        for (op, b) in [
            ("+", 1),
            ("-", 1),
            ("*", -3),
            ("/", -1),
            ("%", 3),
            ("/", 0),
            ("%", 0),
        ] {
            let b = BigInt::from(b);
            let expected = match op {
                "+" => &a + &b,
                "-" => &a - &b,
                "*" => &a * &b,
                "/" if b != BigInt::from(0) => &a / &b,
                "%" if b != BigInt::from(0) => &a % &b,
                _ => BigInt::from(0),
            };
            cases.push(int_case(format!("x {op} {b}"), a.clone(), expected));
        }
        cases.push(int_case("-x", a.clone(), -&a));
        // Promotion and demotion happen with the current intermediate value.
        cases.push(int_case(
            "{ a = x * 3 b = a + 1 c = b - 1 c / 3 }",
            a.clone(),
            a,
        ));
    }
    for a in [-7, 7] {
        for b in [-3, 3] {
            cases.push(int_case(format!("x / {b}"), a.into(), (a / b).into()));
            cases.push(int_case(format!("x % {b}"), a.into(), (a % b).into()));
        }
    }
    cases.push(int_case(
        "{ a = x + 1073741823 b = a * 1073741823 c = b * 1073741823 c - c }",
        1.into(),
        0.into(),
    ));
    cases.push(int_case(
        "{ a = x * 1073741823 b = a % 7 b + 2 }",
        1073741823.into(),
        2.into(),
    ));
    for (source, x, expected) in [
        ("x +. 1.0 -. x", 9007199254740992.0_f64, 0.0),
        ("x *. 0.5", f64::from_bits(1), 0.0),
        ("x *. 0.5", f64::from_bits(3), f64::from_bits(2)),
        ("x +. 0.0", -0.0, 0.0),
        ("x -. 0.0", -0.0, -0.0),
        ("x *. -.1.0", 0.0, -0.0),
        ("x /. 0.0", -7.0, 0.0),
        ("x /. -.0.0", 7.0, 0.0),
        ("-.x", 0.0, -0.0),
        ("-.x", -0.0, 0.0),
        ("x /. 10.0", 1.0, 0.1),
        (
            "{ a = x *. 1.0000000000000002 a -. x }",
            1.0000000000000002,
            2.220446049250313e-16,
        ),
        ("{ a = x +. 0.25 b = a *. 2.0 a +. b }", 0.5, 2.25),
        (
            "{ a = x *. 0.5 f = (y: Float) { y +. a } f(a) }",
            -0.0,
            -0.0,
        ),
        (
            "{ a = x *. 0.5 f = (y: Float) { y +. a } apply(f, a) }",
            0.5,
            0.5,
        ),
        ("{ a = x *. 0.5\n(a, a) |> ((b, c)) { b +. c } }", 0.5, 0.5),
        ("{ a = x *. 0.5\nr = (v = a)\nr.v +. a }", 0.5, 0.5),
        (
            "{ a = x *. 0.5\n{ x <. 0.0 } |> (True) { a } | (False) { a +. 0.25 } }",
            0.5,
            0.5,
        ),
        ("{ a = x *. 0.5 core.floor(a) }", 3.5, 1.0),
        ("{ a = x *. 0.5 core.ceiling(a) }", 3.5, 2.0),
        ("{ a = x *. 0.5 core.sqrt(a) }", 8.0, 2.0),
    ] {
        cases.push(float_case(source, x, expected));
    }
    cases.push(Case {
        body: "{ a = { x <. 0.0 } |> (True) { x } | (False) { \"s\" }\ncore.inspect(a) }".into(),
        input: Value::Float(1.0),
        expected: "\"s\"".into(),
        stop: false,
        events: vec![],
    });
    cases.push(float_case(
        "core.float_from_bits(9223372036854775808)",
        1.0,
        -0.0,
    ));
    for bits in [
        "9218868437227405312",
        "18442240474082181120",
        "9221120237041090560",
    ] {
        cases.push(Case {
            body: format!("core.inspect(core.float_from_bits({bits}))"),
            input: Value::Float(1.0),
            expected: "float: result is not finite".into(),
            stop: true,
            events: vec![],
        });
    }
    cases.push(Case {
        body: "core.inspect({ a = x *. 0.5 a })".into(),
        input: Value::Float(-0.0),
        expected: "-0.0".into(),
        stop: false,
        events: vec![],
    });
    cases.push(int_case("loop_int(10000, x)", 1.into(), 50005001.into()));
    cases.push(int_case(
        "fact(30, x)",
        1.into(),
        "265252859812191058636308480000000".parse().unwrap(),
    ));
    cases.push(float_case("loop_float(10000, x)", -0.0, 2500.0));
    cases.push(float_case("control.boundary((b) { a = x *. 0.5 y = control.suspend(b, (k) { control.resume(k, 1.0) }) a +. y })", 5.0, 3.5));
    cases.push(float_case("control.boundary((b) { a = x *. 0.5 control.suspend(b, (k) { control.resume(k, Nil) }) control.suspend(b, (k) { control.resume(k, Nil) }) a })", -0.0, -0.0));
    cases.push(int_case("control.boundary((b) { a = x * 1073741823 control.suspend(b, (k) { control.resume(k, Nil) }) a + 1 })", 1073741823.into(), "1152921502459363330".parse().unwrap()));
    cases.push(Case { body: "resource.ensure(() { a = x *. 2.0 a +. 1.0 }, () { record(core.inspect(x)) }) |> core.inspect".into(), input: Value::Float(-0.0), expected: "1.0".into(), stop: false, events: vec!["-0.0".into()] });
    cases.push(Case { body: "control.boundary((b) { resource.ensure(() { a = x *. 0.5 control.suspend(b, (k) { control.cancel(k) 2.0 }) a }, () { record(core.inspect(x)) }) }) |> core.inspect".into(), input: Value::Float(-0.0), expected: "2.0".into(), stop: false, events: vec!["-0.0".into()] });
    cases.push(Case {
        body: "resource.ensure(() { x * 3 }, () { record(core.inspect(x)) }) |> core.inspect"
            .into(),
        input: Value::Int("9223372036854775808".parse().unwrap()),
        expected: "27670116110564327424".into(),
        stop: false,
        events: vec!["9223372036854775808".into()],
    });
    for body in [
        "x *. x",
        "x +. x",
        "x -. { -.x }",
        "x /. 1.0e-308",
        "{ unused = x *. x 0.0 }",
        "{ pair = (0.0, x *. x) pair.0 }",
        "control.boundary((b) { a = x control.suspend(b, (k) { control.resume(k, Nil) }) a *. a })",
        "resumed_h(x)",
        "resource.ensure(() { x *. x }, () { record(\"must not run\") })",
    ] {
        cases.push(Case {
            body: format!("core.inspect({body})"),
            input: Value::Float(1e308),
            expected: "float: result is not finite".into(),
            stop: true,
            events: vec![],
        });
    }
    cases.push(Case {
        body: "core.inspect(core.sqrt(-.x))".into(),
        input: Value::Float(1.0),
        expected: "float: result is not finite".into(),
        stop: true,
        events: vec![],
    });
    cases.push(Case {
        body: "{ record(\"before\") panic(\"before\") x *. x } |> core.inspect".into(),
        input: Value::Float(1e308),
        expected: "panic: before".into(),
        stop: true,
        events: vec!["before".into()],
    });
    cases.push(float_case("{ f = { x <. 0.0 } |> (True) { (v: Float) -> Float { v } } | (False) { (v: Float) -> Float { v +. 0.0 } }\nrelay(f, 1000000, x) }", 0.5, 0.5));
    cases.push(Case {
        body: "{ f = make_float(x) g = make_float(x) r = (value = x, callback = f) s = (value = x, callback = f) t = (value = x, callback = g) core.inspect(f == f && f != g && r == s && r != t) }".into(),
        input: Value::Float(-0.0), expected: "True".into(), stop: false, events: vec![],
    });
    for (x, expected) in [(-1.0, -0.5), (1.0, 0.0)] {
        cases.push(float_case("{ a = { x <. 0.0 } |> (True) { x } | (False) { \"s\" }\na |> (v: Float) { v +. 0.5 } | (_) { 0.0 } }", x, expected));
    }
    // N5: exact rounded chains under dominating bounds, and chains whose
    // middle result overflows even though the final expression would be finite.
    for n in 0..24 {
        let x = n as f64 * 0.125 - 1.5;
        let t = x + n as f64 / 32.0;
        let expected = if (-1.0..=1.0).contains(&t) {
            let a = t * 0.5 + 0.25;
            a * 0.5 - 0.125
        } else {
            t
        };
        cases.push(float_case(
            &format!("bounded_float(x +. {n}.0 /. 32.0)"),
            x,
            expected,
        ));
        cases.push(Case {
            body: format!("core.inspect({{ a = x *. 2.0 b = a /. 0.0 b +. {n}.0 }})"),
            input: Value::Float(1.0e308),
            expected: "float: result is not finite".into(),
            stop: true,
            events: vec![],
        });
    }
    for x in [
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MAX,
        -f64::MAX,
        -0.0,
    ] {
        let expected = if (-1.0..=1.0).contains(&x) {
            let a = x * 0.5 + 0.25;
            a * 0.5 - 0.125
        } else {
            x
        };
        cases.push(float_case("bounded_float(x)", x, expected));
    }
    // N4: carry the current accumulator through promotion and demotion. The
    // effect trace checks that leaving a guarded worker never replays a step.
    for n in 0..24 {
        let x = BigInt::from(1073741823_i64 - n);
        let count = n + 1;
        let sum = count * (count + 1) / 2;
        cases.push(int_case(
            format!("loop_int({count}, x)"),
            x.clone(),
            &x + sum,
        ));
        cases.push(int_case(format!("loop_down({count}, x)"), -&x, -&x - sum));
    }
    let start = BigInt::from(1073741821_i64);
    let mut acc = start.clone();
    let mut events = Vec::new();
    for n in (1..=7).rev() {
        events.push(acc.to_string());
        acc += n;
    }
    cases.push(Case {
        body: "core.inspect(loop_effect(7, x))".into(),
        input: Value::Int(start),
        expected: acc.to_string(),
        stop: false,
        events,
    });
    cases.push(int_case(
        "control.boundary((b) { suspend_int(b, x) })",
        1073741822.into(),
        1073741825.into(),
    ));
    // N3: cross-call pairs, structural and nominal numeric records, including
    // joins and repeated projections. Shapes carry actual labels/constructors.
    for n in 0..24 {
        let x = n as f64 * 0.125 - 1.5;
        cases.push(float_case(
            &format!("{{ p = Left(x = x, y = {n}.0) p.x +. p.x }}"),
            x,
            x + x,
        ));
        cases.push(float_case(
            &format!("{{ p = Right(y = {n}.0, x = x) p.x +. p.x }}"),
            x,
            x + x,
        ));
        cases.push(float_case(
            &format!("{{ (a, b) = numeric_pair(x +. {n}.0) a +. b }}"),
            x,
            ((x + n as f64) + 0.25) + (x + n as f64) * 0.5,
        ));
        cases.push(float_case(
            &format!("{{ r = (a = x, b = x *. {n}.0) r.a +. r.a +. r.b }}"),
            x,
            (x + x) + x * n as f64,
        ));
        cases.push(float_case(&format!("{{ r = {{ x <. 0.0 }} |> (True) {{ (a = x, b = {n}.0) }} | (False) {{ (a = x, b = {}.0) }}\nr.a +. r.b }}", n + 1), x, x + if x < 0.0 { n as f64 } else { (n + 1) as f64 }));
    }
    // N2: cross-call scalar chains and closures with different captures share
    // code without sharing identity or the capture values.
    for n in 0..32 {
        let x = n as f64 * 0.125 - 2.0;
        cases.push(float_case(
            &format!("step_float(step_float(x +. {n}.0))"),
            x,
            ((x + n as f64) * 0.5 + 0.25) * 0.5 + 0.25,
        ));
        cases.push(float_case(
            &format!("{{ f = (v: Float) -> Float {{ v *. 0.5 +. x }} apply(f, {n}.0) }}"),
            x,
            n as f64 * 0.5 + x,
        ));
    }
    // Inlined helpers whose operations stop must keep every pending logical
    // frame; an inlined tail call keeps its tail site. Mixed Int/Float workers
    // leave the i31 region mid-loop. Range proofs at the exact overflow tie.
    let stop = |body: String, input: Value| Case {
        body: format!("core.inspect({body})"),
        input,
        expected: "float: result is not finite".into(),
        stop: true,
        events: vec![],
    };
    for (i, x) in [3.0_f64, 1.0e200, 1.0e154, 1.5e154].into_iter().enumerate() {
        let y = x * x;
        let finite = y.is_finite() && (y + 1.0).is_finite();
        let body = format!("helper_g(x) +. {i}.0");
        if finite {
            cases.push(float_case(&body, x, y + 1.0 + i as f64));
        } else {
            cases.push(stop(body, Value::Float(x)));
        }
        let deep = format!("helper_deep(x *. {i}.0 +. 1.0)");
        let z = x * i as f64 + 1.0;
        let w = ((z * z) * (z * z)) * ((z * z) * (z * z));
        if w.is_finite() {
            cases.push(float_case(&deep, x, w));
        } else {
            cases.push(stop(deep, Value::Float(x)));
        }
    }
    for n in 0..8 {
        let x = 10f64.powi(n * 40 - 300);
        let mut y = x;
        let mut ok = true;
        for _ in 0..5 {
            y *= 1.0e100;
            ok &= y.is_finite();
        }
        let body = format!("helper_outer(x) -. {n}.0");
        if ok && (y + 2.0).is_finite() {
            cases.push(float_case(&body, x, (y + 2.0) - n as f64));
        } else {
            cases.push(stop(body, Value::Float(x)));
        }
    }
    for n in 0..8 {
        // acc crosses the i31 boundary after the first step; f overflows
        // after six steps, while the Int part has already been promoted.
        let body = format!("mixed_worker({n}, 1073741000, x)");
        let mut f = 1.0f64;
        let mut acc = 1073741000i64;
        let mut ok = true;
        for _ in 0..n {
            acc += 1073741000;
            f *= 1.0e60;
            ok &= f.is_finite();
        }
        if ok {
            cases.push(float_case(&body, 1.0, f + acc as f64));
        } else {
            cases.push(stop(body, Value::Float(1.0)));
        }
        cases.push(int_case(
            format!("mixed_int({n}, x)"),
            BigInt::from(-1073741824 + n),
            BigInt::from(-1073741824 + n) + BigInt::from(1073741000) * n,
        ));
        cases.push(int_case(
            format!("halve_int({n}, x)"),
            BigInt::from(1) << (25 + n),
            BigInt::from(1) << 25,
        ));
    }
    for (i, (text, k)) in [
        ("2.0", 2.0_f64),
        ("3.0", 3.0),
        ("0.5", 0.5),
        ("1.0e200", 1.0e200),
    ]
    .into_iter()
    .enumerate()
    {
        let body = format!("twice(scale_by({text}), x) +. {i}.0");
        let y = 1.5 * k * k;
        if y.is_finite() {
            cases.push(float_case(&body, 1.5, y + i as f64));
        } else {
            cases.push(stop(body, Value::Float(1.5)));
        }
    }
    for (i, x) in [2.0_f64, 1.0e200, -0.0].into_iter().enumerate() {
        let body = format!("unused_field(x) +. {i}.0");
        if (x * x).is_finite() {
            cases.push(float_case(&body, x, 1.0 + i as f64));
        } else {
            cases.push(stop(body, Value::Float(x)));
        }
    }
    let two970 = 2f64.powi(970);
    for (i, x) in [two970, f64::from_bits(two970.to_bits() - 1), 0.0, 1.0]
        .into_iter()
        .enumerate()
    {
        let body = format!("tie_add(x) +. {i}.0 *. 0.0");
        let y = if (0.0..=1.0e292).contains(&x) {
            x + f64::MAX
        } else {
            0.0
        };
        if y.is_finite() {
            cases.push(float_case(&body, x, y));
        } else {
            cases.push(stop(body, Value::Float(x)));
        }
    }
    for (i, y) in [0.0_f64, -0.0, 1.0e-300, -1.0e-300, 1.0e-310]
        .into_iter()
        .enumerate()
    {
        let body = format!("bounded_div(0.5 +. {i}.0 *. 0.0, x)");
        let q = if y == 0.0 { 0.0 } else { 0.5 / y };
        if q.is_finite() {
            cases.push(float_case(&body, y, q));
        } else {
            cases.push(stop(body, Value::Float(y)));
        }
    }
    // Scalar truncate takes the i31 path only below 2^30 in magnitude, and
    // scalar to_float only for i31 values; both edges and the runtime paths.
    for (x, expected) in [
        (1073741823.9_f64, "1073741823"),
        (1073741824.0, "1073741824"),
        (-1073741823.5, "-1073741823"),
        (-1073741824.0, "-1073741824"),
        (-0.0, "0"),
        (-2.5, "-2"),
        (1.0e20, "100000000000000000000"),
    ] {
        cases.push(Case {
            body: "core.inspect(core.truncate(x *. 1.0))".into(),
            input: Value::Float(x),
            expected: expected.into(),
            stop: false,
            events: vec![],
        });
    }
    for text in [
        "1073741822",
        "1073741823",
        "-1073741825",
        "9007199254740993",
    ] {
        let a: BigInt = text.parse().unwrap();
        let sum = &a + BigInt::from(1);
        let value: f64 = sum.to_string().parse().unwrap();
        cases.push(Case {
            body: "core.inspect(core.float_to_bits(core.to_float(x + 1)))".into(),
            input: Value::Int(a),
            expected: value.to_bits().to_string(),
            stop: false,
            events: vec![],
        });
    }
    // Self tail loops whose stop is in an inlined helper, a list loop, and
    // a fold whose callback captures the input: a stop in a later iteration
    // must print the same trace as the reference.
    for x in [1.0e10_f64, 1.5, -2.0e9] {
        let mut acc = Some(x);
        for _ in 0..10 {
            acc = acc
                .map(|a| a * a)
                .filter(|a| a.is_finite())
                .map(|a| a + 1.0)
                .filter(|a| a.is_finite());
        }
        match acc {
            Some(y) => cases.push(float_case("sq_loop(10, x)", x, y)),
            None => cases.push(stop("sq_loop(10, x)".into(), Value::Float(x))),
        }
    }
    for x in [1.0_f64, 1.0e-300] {
        let body = "list_product([1.0e200, 3.0, 1.0e200], x)";
        let y = x * 1.0e200 * 3.0 * 1.0e200;
        if y.is_finite() {
            cases.push(float_case(body, x, y));
        } else {
            cases.push(stop(body.into(), Value::Float(x)));
        }
    }
    for x in [1.0e10_f64, 0.5] {
        let body = "fold_float([1.0, 2.0, 1.0e300], 1.0, (acc: Float, v: Float) -> Float { acc +. v *. x })";
        let y = [1.0, 2.0, 1.0e300].iter().try_fold(1.0_f64, |acc, v| {
            let p = v * x;
            let s = acc + p;
            (p.is_finite() && s.is_finite()).then_some(s)
        });
        match y {
            Some(y) => cases.push(float_case(body, x, y)),
            None => cases.push(stop(body.into(), Value::Float(x))),
        }
    }
    // List matching must agree through failed literal patterns, short lists,
    // non-list union members, and self-tail iterations with new cells.
    for (items, expected) in [
        ("[]", 0.5),
        ("[2.0]", 1.0),
        ("[2.0, 3.0]", 5.5),
        ("[0.0, 2.0]", 2.5),
        ("[0.0, 2.0, 3.0]", 7.5),
        ("[2.0, 3.0, 4.0]", 8.0),
    ] {
        cases.push(float_case(
            &format!("list_match({items}, x)"),
            0.5,
            expected,
        ));
    }
    for (value, expected) in [("[]", 0.5), ("[2.0]", 2.5), ("x", 1.0)] {
        cases.push(float_case(
            &format!("list_or_float({value}, x)"),
            0.5,
            expected,
        ));
    }
    cases.push(float_case(
        "list_dot([2.0, 3.0, 4.0], [5.0, 6.0], x)",
        0.5,
        28.5,
    ));
    cases.push(float_case("list_dot([2.0], [5.0, 6.0], x)", 0.5, 10.5));
    cases.push(stop(
        "list_match([0.0, 2.0, 3.0], x)".into(),
        Value::Float(f64::MAX),
    ));
    cases.push(stop("list_or_float(x, x)".into(), Value::Float(f64::MAX)));
    let huge: BigInt = format!("1{}", "0".repeat(400)).parse().unwrap();
    cases.push(stop("core.to_float(x + 1)".into(), Value::Int(huge)));
    cases
}

#[test]
fn numeric_corpus_agrees_at_o0_o2_o3() {
    let cases = corpus();
    assert!(cases.len() >= 200);
    assert!(
        cases
            .iter()
            .map(|case| &case.body)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            >= 200,
        "corpus must contain at least 200 different source programs"
    );
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    let exports = (0..cases.len())
        .map(|i| format!("numeric/cases.case_{i}"))
        .collect::<Vec<_>>();
    std::fs::write(
        dir.path().join("i.toml"),
        format!("name = \"numeric\"\n[wasm]\nexports = {exports:?}\n"),
    )
    .unwrap();
    let mut source = String::from(
        r#"(
	i/core
	i/control
	i/resource
)
@external("numeric:events", "record")
record : (String) -> Nil
make_float = (x : Float) { (v : Float) -> Float { v +. x } }
bounded_float = (x : Float) -> Float {
	{ x >=. -.1.0 } |>
		(False) { x }
		| (True) {
			{ x <=. 1.0 } |>
				(False) { x }
				| (True) {
					a = x *. 0.5 +. 0.25
					a *. 0.5 -. 0.125
				}
		}
}
Left = (x = Float, y = Float)
Right = (y = Float, x = Float)
Pair = Left | Right
relay = (f : (Float) -> Float, n : Int, x : Float) -> Float {
	n |> (0) { f(x) } | (_) { relay(f, n - 1, x) }
}
numeric_pair = (x : Float) -> (Float, Float) { (x +. 0.25, x *. 0.5) }
step_float = (x : Float) -> Float { x *. 0.5 +. 0.25 }
apply = (f : (Float) -> Float, x : Float) -> Float { f(x) }
suspend_int = (b, x : Int) -> Int {
	a = x + 1
	control.suspend(b, (k) { control.resume(k, Nil) })
	a + 2
}
loop_down = (n : Int, acc : Int) -> Int {
	n |> (0) { acc } | (_) { loop_down(n - 1, acc - n) }
}
loop_effect = (n : Int, acc : Int) -> Int {
	n |>
		(0) { acc }
		| (_) {
			record(core.inspect(acc))
			loop_effect(n - 1, acc + n)
		}
}
loop_int = (n : Int, sum : Int) -> Int {
	n |> (0) { sum } | (_) { loop_int(n - 1, sum + n) }
}
fact = (n : Int, acc : Int) -> Int {
	n |> (0) { acc } | (_) { fact(n - 1, acc * n) }
}
loop_float = (n : Int, sum : Float) -> Float {
	n |> (0) { sum } | (_) { loop_float(n - 1, sum +. 0.25) }
}
resumed_g = (b, x : Float) -> Float {
	y = control.suspend(b, (k) { control.resume(k, 1.0) })
	x *. x +. y
}
resumed_f = (b, x : Float) -> Float {
	y = resumed_g(b, x)
	y +. 0.25
}
resumed_h = (x : Float) -> Float {
	control.boundary((b) {
		y = resumed_f(b, x)
		y +. 0.5
	})
}
helper_sq = (x : Float) -> Float { x *. x }
sq_loop = (n : Int, acc : Float) -> Float { n |> (0) { acc } | (_) { sq_loop(n - 1, helper_sq(acc) +. 1.0) } }
list_product = (xs : List(Float), acc : Float) -> Float { xs |> ([]) { acc } | ([a, ..rest]) { list_product(rest, acc *. a) } }
fold_float = <A>(items : List(A), acc : Float, f : (Float, A) -> Float) -> Float { items |> ([]) { acc } | ([a, ..rest]) { fold_float(rest, f(acc, a), f) } }
list_match = (xs : List(Float), x : Float) -> Float {
	xs |>
		([0.0, a, ..rest]) { list_match(rest, a +. x) }
		| ([a, b]) { a +. b +. x }
		| ([]) { x }
		| ([a, ..rest]) { list_match(rest, a *. x) }
}
list_or_float = (v : { List(Float) | Float }, x : Float) -> Float {
	v |>
		([]) { x }
		| ([a, ..rest]) { list_or_float(rest, a +. x) }
		| (_ : Float) { x *. 2.0 }
}
list_dot = (as : List(Float), bs : List(Float), acc : Float) -> Float {
	as |>
		([]) { acc }
		| ([a, ..ar]) {
			bs |> ([]) { acc } | ([b, ..br]) { list_dot(ar, br, acc +. a *. b) }
		}
}
helper_g = (x : Float) -> Float {
	y = helper_sq(x)
	y +. 1.0
}
helper_deep = (x : Float) -> Float {
	helper_sq(
		helper_sq(helper_sq(x)),
	)
}
helper_loop = (n : Int, x : Float) -> Float {
	n |> (0) { x } | (_) { helper_loop(n - 1, x *. 1.0e100) }
}
helper_tail = (x : Float) -> Float { helper_loop(5, x) }
helper_outer = (x : Float) -> Float {
	a = helper_tail(x)
	a +. 2.0
}
mixed_worker = (n : Int, acc : Int, f : Float) -> Float {
	n |>
		(0) { f +. core.to_float(acc) }
		| (_) { mixed_worker(n - 1, acc + 1073741000, f *. 1.0e60) }
}
mixed_int = (n : Int, acc : Int) -> Int {
	n |> (0) { acc } | (_) { mixed_int(n - 1, acc + 1073741000) }
}
halve_int = (n : Int, acc : Int) -> Int {
	n |> (0) { acc } | (_) { halve_int(n - 1, acc / 2) }
}
scale_by = (k : Float) -> (Float) -> Float { (v : Float) -> Float { v *. k } }
twice = (f : (Float) -> Float, x : Float) -> Float { f(f(x)) }
unused_field = (x : Float) -> Float {
	r = (a = x *. x, b = 1.0)
	r.b
}
tie_add = (x : Float) -> Float {
	{ x <=. 1.0e292 } |>
		(True) {
			{ x >=. 0.0 } |>
				(True) { x +. 1.7976931348623157e308 }
				| (False) { 0.0 }
		}
		| (False) { 0.0 }
}
bounded_div = (x : Float, y : Float) -> Float {
	{ y >=. -.1.0e-300 } |>
		(True) { { y <=. 1.0e-300 } |> (True) { x /. y } | (False) { 0.0 } }
		| (False) { 0.0 }
}
"#,
    );
    source = source.replacen(
        "\n@external",
        &format!(
            "\n({})\n@external",
            (0..cases.len())
                .map(|i| format!("case_{i}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        1,
    );
    for (i, case) in cases.iter().enumerate() {
        let ty = match case.input {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            _ => unreachable!(),
        };
        source.push_str(&format!(
            "case_{i} = (x: {ty}) -> String {{ {} }}\n",
            case.body
        ));
    }
    if let Some(path) = std::env::var_os("NUMERIC_SOURCE_DUMP") {
        std::fs::write(path, &source).unwrap();
    }
    std::fs::write(dir.path().join("src/cases.i"), source).unwrap();
    let (graph, diagnostics) = ilang_package::load(dir.path());
    assert!(
        !diagnostics.iter().any(ilang_package::Diagnostic::is_error),
        "{diagnostics:?}"
    );
    let checked = ilang_check::check(&graph);
    assert!(
        !checked
            .diags
            .iter()
            .any(ilang_package::Diagnostic::is_error),
        "{:?}",
        graph.sources.render_all(&checked.diags)
    );
    let program = checked.program.as_ref().unwrap();
    let typed = checked.types.as_ref().unwrap();
    assert!(
        typed
            .numeric
            .values()
            .any(|v| *v == ilang_check::types::NumericFact::FiniteF64)
    );
    let mut oracle = None;
    for (opt, mode) in [
        (0, Numeric::Reference),
        (2, Numeric::Visible),
        (2, Numeric::Scalars),
        (3, Numeric::Scalars),
    ] {
        let (wasm, _) = ilang_wasm::compile_with_bindings_numeric(
            &graph,
            program,
            typed,
            Options::default().numeric(mode),
        )
        .unwrap();
        if opt == 0 {
            assert_eq!(
                wasm,
                ilang_wasm::compile(&graph, program, typed, Options::default()).unwrap(),
                "O0 bytes changed"
            );
        }
        let wasm = ilang_cli::optimize(&wasm, ilang_cli::level(opt).unwrap()).unwrap();
        if let Some(path) = std::env::var_os("NUMERIC_WASM_DUMP") {
            let path = std::path::PathBuf::from(path);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join(format!("o{opt}-{mode:?}.wasm")), &wasm).unwrap();
        }
        let compiled = Program::from_bytes(&wasm).unwrap();
        let mut observations = Vec::new();
        for (i, case) in cases.iter().enumerate() {
            let events = Arc::new(Mutex::new(Vec::new()));
            let recorded = events.clone();
            let mut imports = HostImports::default();
            imports
                .external("numeric:events", "record", move |args| {
                    let [Value::String(message)] = args else {
                        panic!("record takes String")
                    };
                    recorded.lock().unwrap().push(message.clone());
                    Ok(Value::Nil)
                })
                .unwrap();
            // Stops invalidate an instance. Each case starts from a fresh one.
            let mut instance = compiled.instantiate(HostOptions::new(imports)).unwrap();
            let result = instance
                .call(&exports[i], std::slice::from_ref(&case.input))
                .unwrap();
            let value = match result {
                Outcome::Ok(Value::String(value)) => {
                    assert!(!case.stop, "case {i} O{opt} did not stop");
                    assert_eq!(value, case.expected, "case {i} O{opt}: {}", case.body);
                    value
                }
                Outcome::Stop(stop) => {
                    assert!(case.stop, "case {i} O{opt}: {stop}");
                    let message = stop
                        .message
                        .expect("language stop has a complete logical trace");
                    assert!(
                        message.starts_with(&case.expected),
                        "case {i} O{opt}: {message}"
                    );
                    assert!(
                        message.contains("numeric/cases:"),
                        "missing stop position: {message}"
                    );
                    message
                }
                other => panic!("case {i} O{opt}: {other:?}"),
            };
            let events = events.lock().unwrap().clone();
            assert_eq!(events, case.events, "case {i} O{opt} final/effect order");
            observations.push((value, events));
        }
        if let Some(oracle) = &oracle {
            assert_eq!(
                &observations, oracle,
                "O{opt} differs from O0, including complete logical stop traces"
            );
        } else {
            oracle = Some(observations);
        }
    }
    if let Some(path) = std::env::var_os("NUMERIC_WASM_DUMP") {
        let cases = cases
            .iter()
            .enumerate()
            .map(|(i, case)| {
                let input = match &case.input {
                    Value::Int(n) => serde_json::json!({ "kind": "int", "value": n.to_string() }),
                    Value::Float(x) => {
                        serde_json::json!({ "kind": "float", "bits": x.to_bits().to_string() })
                    }
                    _ => unreachable!(),
                };
                serde_json::json!({ "export": exports[i], "input": input, "stop": case.stop })
            })
            .collect::<Vec<_>>();
        std::fs::write(
            std::path::PathBuf::from(path).join("cases.json"),
            serde_json::to_vec(
                &serde_json::json!({ "cases": cases, "observations": oracle.unwrap() }),
            )
            .unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn nonnumeric_functions_are_emitted_byte_equal() {
    // Every program has the library code that the runtime calls
    // (`lib/i/float_text.i`), which has Int arithmetic, so no whole module is
    // free of numeric regions any more. A function with no numeric region
    // must still be emitted byte for byte the same at every level.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        "name = \"plain\"\n[wasm]\nexports = [\"plain/pass.pass\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/pass.i"),
        "(pass)\npass = (x : String) -> String { x }\n",
    )
    .unwrap();
    let (graph, diagnostics) = ilang_package::load(dir.path());
    assert!(diagnostics.is_empty());
    let checked = ilang_check::check(&graph);
    assert!(
        !checked
            .diags
            .iter()
            .any(ilang_package::Diagnostic::is_error)
    );
    let program = checked.program.as_ref().unwrap();
    let typed = checked.types.as_ref().unwrap();
    // The text of the function with this debug name, up to the next one.
    fn function<'t>(text: &'t str, name: &str) -> &'t str {
        let start = text
            .find(&format!("\n;; {name}\n"))
            .unwrap_or_else(|| panic!("no function {name}"));
        let end = text[start + 1..]
            .find("\n;; ")
            .map_or(text.len(), |end| start + 1 + end);
        &text[start..end]
    }
    let reference = ilang_wasm::compile_wat(&graph, program, typed, Options::default()).unwrap();
    for mode in [Numeric::Visible, Numeric::Scalars] {
        let text = ilang_wasm::compile_wat_numeric(
            &graph,
            program,
            typed,
            Options::default().numeric(mode),
        )
        .unwrap();
        assert_eq!(
            function(&text, "plain/pass.pass"),
            function(&reference, "plain/pass.pass"),
            "no numeric region must leave WAT unchanged"
        );
    }
}

#[test]
fn representation_facts_follow_code_identity_not_debug_names() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"identity\"\n[wasm]\nexports = [\"identity/functions.floating\", \"identity/functions.integer\"]\n").unwrap();
    std::fs::write(dir.path().join("src/functions.i"), "(floating, integer)\nfloating = (x : Float) -> Float { x +. 1.0 }\ninteger = (x : Int) -> Int { x + 1 }\n").unwrap();
    let (graph, diagnostics) = ilang_package::load(dir.path());
    assert!(diagnostics.is_empty());
    let checked = ilang_check::check(&graph);
    assert!(
        !checked
            .diags
            .iter()
            .any(ilang_package::Diagnostic::is_error)
    );
    let mut ir = ilang_wasm::lower::lower(
        &graph,
        checked.program.as_ref().unwrap(),
        checked.types.as_ref().unwrap(),
        Options::default(),
    )
    .unwrap();
    ir.numeric = Numeric::Scalars;
    let before = ilang_wasm::emit::wat(&ir);
    for function in &mut ir.funcs {
        if let ilang_wasm::ir::FuncKind::Code(function) = function {
            function.name = "same debug name".into();
        }
    }
    let after = ilang_wasm::emit::wat(&ir);
    let without_comments = |text: &str| {
        text.lines()
            .filter(|line| !line.trim_start().starts_with(";;"))
            .map(|line| format!("{line}\n"))
            .collect::<String>()
    };
    assert_eq!(
        without_comments(&before),
        without_comments(&after),
        "debug names must not decide parameter/local representations"
    );
}

/// The review's exact resumed-stop fixture. Lines and columns are normative.
#[test]
fn transitive_suspend_preserves_pending_logical_frames() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"trace\"\n").unwrap();
    std::fs::write(dir.path().join("src/main.i"), "(i/control)\ng = (b, x : Float) -> Float {\n y = control.suspend(b, (k) { control.resume(k, 1.0) })\n x *. x +. y\n}\nf = (b, x : Float) -> Float {\n y = g(b, x)\n y +. 0.25\n}\nh = (b, x : Float) -> Float {\n y = f(b, x)\n y +. 0.5\n}\n\nmain = () {\n control.boundary((b) { h(b, 1.0e308) })\n Nil\n}\n").unwrap();
    let (graph, diags) = ilang_package::load(dir.path());
    assert!(diags.is_empty());
    let checked = ilang_check::check(&graph);
    let wat = ilang_wasm::compile_wat_numeric(
        &graph,
        checked.program.as_ref().unwrap(),
        checked.types.as_ref().unwrap(),
        Options::default().numeric(Numeric::Scalars),
    )
    .unwrap();
    for name in ["f", "h"] {
        assert!(
            wat.lines().any(|line| line.contains("numeric: code")
                && line.contains(&format!(".{name})"))
                && line.contains("inline=false suspend=true")),
            "transitive exclusion for {name}: {}",
            wat.lines()
                .filter(|l| l.contains("numeric: code"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    for opt in ["0", "2", "3"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_i"))
            .args([
                "run",
                "--project",
                dir.path().to_str().unwrap(),
                "--opt",
                opt,
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "O{opt}");
        assert!(output.stdout.is_empty(), "O{opt}: {:?}", output.stdout);
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "float: result is not finite\ntrace/main:4:2\ntrace/main:7:6\ntrace/main:11:6\ntrace/main:16:2\n",
            "O{opt}"
        );
    }
}
