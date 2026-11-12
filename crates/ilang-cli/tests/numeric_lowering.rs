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
        ("{ a = x *. 0.5\n(a, a) |> { (b, c) -> b +. c } }", 0.5, 0.5),
        ("{ a = x *. 0.5 r = (v: a) r.v +. a }", 0.5, 0.5),
        (
            "{ a = x *. 0.5 { x <. 0.0 } |> { True -> a False -> a +. 0.25 } }",
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
        body: "{ a = { x <. 0.0 } |> { True -> x False -> \"s\" } core.inspect(a) }".into(),
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
    cases
}

#[test]
fn numeric_corpus_agrees_at_o0_o2_o3() {
    let cases = corpus();
    assert!(cases.len() >= 60);
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
        r#"
(i/core, i/control, i/resource)
@external("numeric:events", "record")
record: (String) -> Nil
apply = (f: (Float) -> Float, x: Float) -> Float { f(x) }
loop_int = (n: Int, sum: Int) -> Int { n |> { 0 -> sum _ -> loop_int(n - 1, sum + n) } }
fact = (n: Int, acc: Int) -> Int { n |> { 0 -> acc _ -> fact(n - 1, acc * n) } }
loop_float = (n: Int, sum: Float) -> Float { n |> { 0 -> sum _ -> loop_float(n - 1, sum +. 0.25) } }
resumed_g = (b, x: Float) -> Float {
 y = control.suspend(b, (k) { control.resume(k, 1.0) })
 x *. x +. y
}
resumed_f = (b, x: Float) -> Float { y = resumed_g(b, x) y +. 0.25 }
resumed_h = (x: Float) -> Float {
 control.boundary((b) { y = resumed_f(b, x) y +. 0.5 })
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
}

#[test]
fn nonnumeric_emission_and_optimization_are_byte_equal() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("i.toml"),
        "name = \"plain\"\n[wasm]\nexports = [\"plain/pass.pass\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/pass.i"),
        "(pass)\npass = (x: String) -> String { x }\n",
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
    let reference = ilang_wasm::compile_wat(&graph, program, typed, Options::default()).unwrap();
    let (bytes, _) =
        ilang_wasm::compile_with_bindings(&graph, program, typed, Options::default()).unwrap();
    for mode in [Numeric::Visible, Numeric::Scalars] {
        let text = ilang_wasm::compile_wat_numeric(
            &graph,
            program,
            typed,
            Options::default().numeric(mode),
        )
        .unwrap();
        assert_eq!(
            text, reference,
            "no numeric region must leave WAT unchanged"
        );
        let (candidate, _) = ilang_wasm::compile_with_bindings_numeric(
            &graph,
            program,
            typed,
            Options::default().numeric(mode),
        )
        .unwrap();
        for level in [ilang_cli::Level::O2, ilang_cli::Level::O3] {
            assert_eq!(
                ilang_cli::optimize(&bytes, level).unwrap(),
                ilang_cli::optimize(&candidate, level).unwrap()
            );
        }
    }
}

#[test]
fn representation_facts_follow_code_identity_not_debug_names() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"identity\"\n[wasm]\nexports = [\"identity/functions.floating\", \"identity/functions.integer\"]\n").unwrap();
    std::fs::write(dir.path().join("src/functions.i"), "(floating, integer)\nfloating = (x: Float) -> Float { x +. 1.0 }\ninteger = (x: Int) -> Int { x + 1 }\n").unwrap();
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
