use super::*;
use std::sync::Mutex;

const FIXTURE: &[u8] = include_bytes!("../../../tests/browser/fixtures/host-abi.wasm");
const CUSTOM: &[u8] = include_bytes!("../../../tests/browser/fixtures/custom-imports.wasm");

type Log = Arc<Mutex<Vec<String>>>;

/// The host functions of host-abi.wasm. `print` writes to the log, `read`
/// answers from a small table, `later` answers its argument, `hypot` works.
fn imports(log: &Log) -> HostImports {
    let mut imports = HostImports::default();
    let printed = Arc::clone(log);
    imports
        .service("app:console/v1", "print", move |arguments| {
            let [Value::String(text)] = arguments else {
                bail!("print takes one String")
            };
            printed.lock().unwrap().push(text.clone());
            Ok(Ok(Value::Nil))
        })
        .unwrap();
    imports
        .service("app:storage/v1", "read", |arguments| {
            let [Value::String(key)] = arguments else {
                bail!("read takes one String")
            };
            Ok(match key.as_str() {
                "name" => Ok(Value::String("i".into())),
                "fail" => bail!("the storage is gone"),
                "int" => Err(Value::Int(BigInt::from(1))),
                "wrong" => Ok(Value::Float(1.0)),
                "nan" => Err(Value::Float(f64::NAN)),
                _ => Err(Value::String(format!("no {key}"))),
            })
        })
        .unwrap();
    imports
        .async_service("app:clock/v1", "later", |arguments| {
            let value = arguments[0].clone();
            async move {
                match value {
                    Value::Float(x) if x < 0.0 => Ok(Err(Value::String("negative".into()))),
                    Value::Float(x) if x > 100.0 => bail!("the clock is broken"),
                    value => Ok(Ok(value)),
                }
            }
        })
        .unwrap();
    imports
        .external("app:math/v1", "hypot", |arguments| {
            let [Value::Float(a), Value::Float(b)] = arguments else {
                bail!("hypot takes two Floats")
            };
            Ok(Value::Float(a.hypot(*b)))
        })
        .unwrap();
    imports
}

fn program() -> Program {
    Program::from_bytes(FIXTURE).unwrap()
}

fn fixture() -> (Instance, Log) {
    let log = Log::default();
    let instance = program()
        .instantiate(HostOptions::new(imports(&log)))
        .unwrap();
    (instance, log)
}

fn raw(instance: &mut Instance, name: &str) -> i32 {
    instance
        .instance
        .get_typed_func::<(), i32>(&mut instance.store, name)
        .unwrap()
        .call(&mut instance.store, ())
        .unwrap()
}

fn allocations(instance: &mut Instance) -> i32 {
    raw(instance, "live_allocations")
}

fn ok(outcome: Outcome) -> Value {
    match outcome {
        Outcome::Ok(value) => value,
        other => panic!("expected a value, got {other:?}"),
    }
}

fn stop(outcome: Outcome) -> Stop {
    match outcome {
        Outcome::Stop(stop) => stop,
        other => panic!("expected a stop, got {other:?}"),
    }
}

#[test]
fn metadata_is_read_from_the_custom_section() {
    let abi = program().abi().clone();
    assert_eq!(abi.version, 2);
    assert_eq!(abi.entry, Some(EntryPoint::new("app/main.main")));
    assert_eq!(abi.tests.len(), 6);
    assert!(abi.tests[5].is_async);
    assert_eq!(
        abi.exports["choose"],
        Signature::new([AbiType::Float], AbiType::String).with_errors([
            AbiType::Nil,
            AbiType::Bool,
            AbiType::Int,
            AbiType::Float,
            AbiType::String,
            AbiType::BitArray,
        ])
    );
    assert!(abi.exports["app/async.later"].is_async);
    assert!(abi.services["app:clock/v1"]["later"].is_async);
    assert_eq!(
        abi.externals["app:math/v1"]["hypot"],
        Signature::new([AbiType::Float, AbiType::Float], AbiType::Float)
    );
    assert!(
        Program::from_bytes(FIXTURE)
            .unwrap()
            .with_abi(Abi::default())
            .is_err()
    );
}

#[test]
fn metadata_that_does_not_follow_b6_is_refused() {
    let parse = |json: &str| Abi::from_json(json.as_bytes());
    let empty =
        r#"{"version":2,"entry":null,"tests":[],"exports":{},"services":{},"externals":{}}"#;
    assert!(parse(empty).is_ok());
    for json in [
        r#"{"version":1,"entry":null,"tests":[],"exports":{},"services":{},"externals":{}}"#,
        r#"{"version":2,"entry":null,"tests":[],"exports":{},"services":{}}"#,
        r#"{"version":2,"tests":[],"exports":{},"services":{},"externals":{}}"#,
        r#"{"version":2,"entry":null,"tests":[],"exports":{},"services":{},"externals":{},"imports":{}}"#,
        r#"{"version":2,"entry":"main","tests":[],"exports":{},"services":{},"externals":{}}"#,
        r#"{"version":2,"entry":null,"tests":[],"exports":{"f":{"params":[],"result":"nil","errors":["string","int"],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":2,"entry":null,"tests":[],"exports":{"f":{"params":[],"result":"nil","errors":["int","int"],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":2,"entry":null,"tests":[],"exports":{"f":{"params":[],"result":"nil","errors":["externref"],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":2,"entry":null,"tests":[],"exports":{"f":{"params":["list"],"result":"nil","errors":[],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":2,"entry":null,"tests":[],"exports":{"ilang_start":{"params":[],"result":"nil","errors":[],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":2,"entry":{"name":"f","async":false},"tests":[],"exports":{"f":{"params":[],"result":"nil","errors":[],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":2,"entry":null,"tests":[],"exports":{},"services":{"ilang:host/v1":{}},"externals":{}}"#,
        r#"{"version":2,"entry":null,"tests":[],"exports":{},"services":{},"externals":{"m":{"f":{"params":[],"result":"nil","errors":["nil"]}}}}"#,
        r#"{"version":2,"entry":null,"tests":[],"exports":{},"services":{"m":{"f":{"params":[],"result":"nil"}}},"externals":{"m":{"f":{"params":[],"result":"nil"}}}}"#,
    ] {
        assert!(parse(json).is_err(), "{json}");
    }
}

#[test]
fn start_runs_once_before_anything_and_may_call_imports() {
    let (mut instance, log) = fixture();
    assert_eq!(raw(&mut instance, "initialized"), 1);
    assert_eq!(*log.lock().unwrap(), ["start\n"]);
    assert_eq!(allocations(&mut instance), 0);
}

#[test]
fn values_round_trip_and_every_buffer_is_freed() {
    let (mut instance, _) = fixture();
    for text in ["", "i 🦀 e\u{301}", "one\0two"] {
        let value = ok(instance
            .call("string_roundtrip", &[Value::String(text.into())])
            .unwrap());
        assert!(matches!(value, Value::String(actual) if actual == text));
    }
    let huge = BigInt::from(1_u32) << 4096_u32;
    for number in [BigInt::from(0), BigInt::from(-1), huge.clone(), -huge] {
        let value = ok(instance
            .call("int_roundtrip", &[Value::Int(number.clone())])
            .unwrap());
        assert!(matches!(value, Value::Int(actual) if actual == number));
    }
    for bits in [
        BitArray::new(vec![], 0).unwrap(),
        BitArray::new(vec![0b1010_0000], 3).unwrap(),
        BitArray::new(vec![0xff, 0x80], 9).unwrap(),
    ] {
        let value = ok(instance
            .call("bit_array_roundtrip", &[Value::BitArray(bits.clone())])
            .unwrap());
        assert!(matches!(value, Value::BitArray(actual) if actual == bits));
    }
    let value = ok(instance
        .call("float_roundtrip", &[Value::Float(-0.0)])
        .unwrap());
    assert!(matches!(value, Value::Float(x) if x.to_bits() == (-0.0_f64).to_bits()));
    assert!(matches!(
        ok(instance
            .call("bool_roundtrip", &[Value::Bool(true)])
            .unwrap()),
        Value::Bool(true)
    ));
    assert!(matches!(
        ok(instance.call("nil_roundtrip", &[]).unwrap()),
        Value::Nil
    ));
    assert!(matches!(
        ok(instance
            .call("opaque_roundtrip", &[Value::NullExternref])
            .unwrap()),
        Value::NullExternref
    ));
    assert!(matches!(
        ok(instance.call("exceptions", &[]).unwrap()),
        Value::Float(42.0)
    ));
    assert_eq!(allocations(&mut instance), 0);
    assert!(BitArray::new(vec![0xff], 3).is_err());
    assert!(BitArray::new(vec![0], 9).is_err());
}

#[test]
fn errors_use_the_tag_and_the_canonical_order() {
    let (mut instance, _) = fixture();
    let expected = [
        "Ok(String(\"ok\"))",
        "Err(Nil)",
        "Err(Bool(true))",
        "Err(Int(-7))",
        "Err(Float(2.5))",
        "Err(String(\"bad\"))",
        "Err(BitArray(BitArray { bytes: [160], bit_length: 3 }))",
    ];
    for (selector, expected) in expected.iter().enumerate() {
        let outcome = instance
            .call("choose", &[Value::Float(selector as f64)])
            .unwrap();
        let text = match outcome {
            Outcome::Ok(value) => format!("Ok({value:?})"),
            Outcome::Err(GuestError(value)) => format!("Err({value:?})"),
            Outcome::Stop(stop) => panic!("stopped: {stop}"),
        };
        assert_eq!(text, *expected);
        assert_eq!(allocations(&mut instance), 0);
    }
    let error = instance.call("choose", &[Value::Float(7.0)]).unwrap_err();
    assert!(format!("{error:#}").contains("error tag 7"), "{error:#}");
    assert!(instance.unusable().is_some());
}

#[test]
fn host_services_and_externals_use_the_same_encoding() {
    let (mut instance, log) = fixture();
    let value = ok(instance
        .call("read_through", &[Value::String("name".into())])
        .unwrap());
    assert!(matches!(value, Value::String(text) if text == "i"));
    let Outcome::Err(GuestError(Value::String(error))) = instance
        .call("read_through", &[Value::String("other".into())])
        .unwrap()
    else {
        panic!("expected an error")
    };
    assert_eq!(error, "no other");
    let value = ok(instance
        .call("hypot", &[Value::Float(3.0), Value::Float(4.0)])
        .unwrap());
    assert!(matches!(value, Value::Float(5.0)));
    ok(instance
        .call("print", &[Value::String("hello\n".into())])
        .unwrap());
    ok(instance
        .call("print", &[Value::String(String::new())])
        .unwrap());
    assert_eq!(*log.lock().unwrap(), ["start\n", "hello\n", ""]);
    assert_eq!(allocations(&mut instance), 0);
}

#[test]
fn incorrect_host_values_stop_the_program() {
    for key in ["int", "wrong", "nan"] {
        let (mut instance, _) = fixture();
        let stop = stop(
            instance
                .call("read_through", &[Value::String(key.into())])
                .unwrap(),
        );
        assert_eq!(stop.message.as_deref(), Some(HOST_VALUE_REASON), "{key}");
        assert!(instance.call("nil_roundtrip", &[]).is_err());
    }
    for (name, argument) in [
        ("float_roundtrip", Value::Float(f64::NAN)),
        ("float_roundtrip", Value::Float(f64::INFINITY)),
        ("bool_roundtrip", Value::String("bad".into())),
        (
            "bit_array_roundtrip",
            Value::BitArray(BitArray {
                bytes: vec![1],
                bit_length: 1,
            }),
        ),
    ] {
        let (mut instance, _) = fixture();
        let stop = stop(instance.call(name, &[argument]).unwrap());
        assert_eq!(stop.message.as_deref(), Some(HOST_VALUE_REASON));
        assert!(instance.unusable().is_some());
    }
    let (mut instance, _) = fixture();
    assert!(instance.call("bool_roundtrip", &[]).is_err());
    assert!(instance.call("absent", &[]).is_err());
    assert!(instance.unusable().is_none());
}

#[test]
fn incorrect_guest_values_are_errors_of_the_module() {
    for name in ["bad_bool", "bad_utf8", "bad_tag"] {
        let (mut instance, _) = fixture();
        assert!(instance.call(name, &[]).is_err(), "{name}");
        assert!(instance.unusable().is_some());
    }
}

#[test]
fn a_stop_gives_its_message_and_the_instance_is_unusable() {
    let (mut instance, _) = fixture();
    let message = "panic: boom\napp/users:12:5";
    let stop = stop(
        instance
            .call("stop", &[Value::String(message.into())])
            .unwrap(),
    );
    assert_eq!(stop.message.as_deref(), Some(message));
    assert!(stop.trap.contains("unreachable"), "{}", stop.trap);
    let refused = instance.call("nil_roundtrip", &[]).unwrap_err();
    assert!(format!("{refused}").contains("make a new instance"));
    assert!(instance.run("app/test.passing").is_err());
}

#[test]
fn a_failing_host_function_traps_and_keeps_its_error() {
    let (mut instance, _) = fixture();
    let error = instance
        .call("read_through", &[Value::String("fail".into())])
        .unwrap_err();
    assert_eq!(format!("{error}"), "the storage is gone");
    assert!(instance.unusable().is_some());
    let missing = program().instantiate(HostOptions::default()).err().unwrap();
    assert!(format!("{missing:#}").contains("missing host function"));
}

#[test]
fn entry_and_tests_run_by_name_each_test_in_a_new_instance() {
    let log = Log::default();
    let program = program();
    let options = HostOptions::new(imports(&log));
    assert!(matches!(
        program.run_entry(options.clone()).unwrap(),
        Outcome::Ok(Value::Nil)
    ));
    assert_eq!(*log.lock().unwrap(), ["start\n", "fixture main\n"]);
    let reports = program.run_tests(&options);
    let summary: Vec<_> = reports
        .iter()
        .map(|report| match &report.result {
            TestResult::Passed => format!("{} passed", report.name),
            TestResult::Stopped(stop) => format!("{} stopped: {stop}", report.name),
            TestResult::Failed(error) => format!("{} failed: {error}", report.name),
        })
        .collect();
    assert_eq!(
        summary,
        [
            "app/test.passing passed".to_owned(),
            "app/test.failing stopped: assertion failed: fixture\napp/test:3:5".to_owned(),
            "app/test.trapping stopped: trap: wasm trap: wasm `unreachable` instruction executed"
                .to_owned(),
            "app/test.fresh passed".to_owned(),
            "app/test.fresh passed".to_owned(),
            "app/async_test.waits passed".to_owned(),
        ]
    );
}

#[test]
fn async_exports_wait_for_host_functions() {
    let (mut instance, _) = fixture();
    let value = ok(block_on(instance.call_async("app/async.later", &[Value::Float(1.5)])).unwrap());
    assert!(matches!(value, Value::Float(2.5)));
    let outcome = block_on(instance.call_async("app/async.later", &[Value::Float(-1.0)])).unwrap();
    assert!(matches!(outcome, Outcome::Err(GuestError(Value::String(text))) if text == "negative"));
    assert_eq!(allocations(&mut instance), 0);
    assert!(
        instance
            .call("app/async.later", &[Value::Float(1.0)])
            .is_err()
    );
    assert!(
        block_on(instance.call_async("hypot", &[Value::Float(1.0), Value::Float(1.0)])).is_err()
    );
    let error =
        block_on(instance.call_async("app/async.later", &[Value::Float(500.0)])).unwrap_err();
    assert_eq!(format!("{error}"), "the clock is broken");
    assert!(instance.unusable().is_some());
}

#[test]
fn references_keep_identity_across_collection_and_support_null() {
    let (mut instance, _) = fixture();
    let reference = HostReference::new(String::from("owned by the embedder"));
    for _ in 0..10 {
        let Value::Externref(returned) = ok(instance
            .call("opaque_roundtrip", &[Value::Externref(reference.clone())])
            .unwrap())
        else {
            panic!("expected reference")
        };
        instance.collect_garbage().unwrap();
        assert!(reference.ptr_eq(&returned));
        assert_eq!(
            returned.downcast_ref::<String>().unwrap(),
            "owned by the embedder"
        );
    }
    let other = HostReference::new(42);
    for (a, b, same) in [
        (
            Value::Externref(reference.clone()),
            Value::Externref(reference.clone()),
            true,
        ),
        (
            Value::Externref(reference.clone()),
            Value::Externref(other.clone()),
            false,
        ),
        (Value::NullExternref, Value::NullExternref, true),
        (Value::NullExternref, Value::Externref(other), false),
    ] {
        assert!(matches!(
            ok(instance.call("same_reference", &[a, b]).unwrap()),
            Value::Bool(actual) if actual == same
        ));
    }
}

#[test]
fn engine_features_include_gc_tail_calls_and_exceptions() {
    let (mut instance, _) = fixture();
    assert_eq!(raw(&mut instance, "feature_probe"), 42);
    let tail = instance
        .instance
        .get_typed_func::<(i32, i32), i32>(&mut instance.store, "tail_count")
        .unwrap()
        .call(&mut instance.store, (1_000_000, 0))
        .unwrap();
    assert_eq!(tail, 1_000_000);
    let probe = Program::from_bytes(include_bytes!(
        "../../../tests/browser/fixtures/required-features.wasm"
    ))
    .unwrap();
    let engine = &probe.engine;
    let mut store = Store::new(engine, ());
    let instance = Linker::new(engine)
        .instantiate(&mut store, &probe.module)
        .unwrap();
    let answer = instance
        .get_typed_func::<(), i32>(&mut store, "probe")
        .unwrap()
        .call(&mut store, ())
        .unwrap();
    assert_eq!(answer, 42);
}

fn custom_abi() -> Abi {
    let unary = |kind: AbiType| Signature::new([kind], kind);
    let mut abi = Abi::default();
    let mut externals = BTreeMap::new();
    for (name, kind) in [
        ("echo_string", AbiType::String),
        ("echo_int", AbiType::Int),
        ("echo_bits", AbiType::BitArray),
        ("echo_ref", AbiType::Externref),
        ("echo_bool", AbiType::Bool),
        ("echo_float", AbiType::Float),
    ] {
        abi.exports.insert(name.into(), unary(kind));
        externals.insert(name.to_owned(), unary(kind));
    }
    abi.exports
        .insert("echo_nil".into(), Signature::new([], AbiType::Nil));
    externals.insert("echo_nil".into(), Signature::new([], AbiType::Nil));
    abi.externals.insert("custom".into(), externals);
    let fallible = Signature::new([AbiType::Float], AbiType::String).with_errors([
        AbiType::Nil,
        AbiType::Bool,
        AbiType::Int,
        AbiType::Float,
        AbiType::String,
        AbiType::BitArray,
    ]);
    abi.exports.insert("fallible".into(), fallible.clone());
    abi.services.insert(
        "custom:fallible/v1".into(),
        BTreeMap::from([("fallible".to_owned(), fallible)]),
    );
    abi
}

fn custom() -> Instance {
    let mut imports = HostImports::default();
    for name in [
        "echo_string",
        "echo_int",
        "echo_bits",
        "echo_ref",
        "echo_bool",
        "echo_float",
        "echo_nil",
    ] {
        imports
            .external("custom", name, |arguments| {
                Ok(arguments.first().cloned().unwrap_or(Value::Nil))
            })
            .unwrap();
    }
    imports
        .service("custom:fallible/v1", "fallible", |arguments| {
            let [Value::Float(selector)] = arguments else {
                bail!("fallible takes a Float")
            };
            Ok(match *selector as i32 {
                0 => Ok(Value::String("fine".into())),
                1 => Err(Value::Nil),
                2 => Err(Value::Bool(false)),
                3 => Err(Value::Int(BigInt::from(1) << 100)),
                4 => Err(Value::Float(-0.5)),
                5 => Err(Value::String("oops".into())),
                _ => Err(Value::BitArray(BitArray::new(vec![0xc0], 2).unwrap())),
            })
        })
        .unwrap();
    Program::from_bytes(CUSTOM)
        .unwrap()
        .with_abi(custom_abi())
        .unwrap()
        .instantiate(HostOptions::new(imports))
        .unwrap()
}

#[test]
fn externals_and_service_errors_round_trip_without_leaks() {
    let mut instance = custom();
    let huge: BigInt = (BigInt::from(1) << 1024) + 19;
    for value in [
        Value::String("host 👩🏽‍🚀 value".into()),
        Value::String(String::new()),
        Value::Int(-huge),
        Value::Int(BigInt::from(0)),
        Value::BitArray(BitArray::new(vec![0xc0], 2).unwrap()),
        Value::Bool(true),
        Value::Float(0.25),
        Value::Nil,
    ] {
        let name = match value.abi_type() {
            AbiType::String => "echo_string",
            AbiType::Int => "echo_int",
            AbiType::BitArray => "echo_bits",
            AbiType::Bool => "echo_bool",
            AbiType::Float => "echo_float",
            _ => "echo_nil",
        };
        let arguments = if name == "echo_nil" {
            vec![]
        } else {
            vec![value.clone()]
        };
        let returned = ok(instance.call(name, &arguments).unwrap());
        assert_eq!(format!("{returned:?}"), format!("{value:?}"));
        assert_eq!(allocations(&mut instance), 0);
    }
    let reference = HostReference::new(());
    let Value::Externref(returned) = ok(instance
        .call("echo_ref", &[Value::Externref(reference.clone())])
        .unwrap())
    else {
        panic!("expected a reference")
    };
    assert!(returned.ptr_eq(&reference));
    let expected = [
        "Ok(String(\"fine\"))",
        "Err(Nil)",
        "Err(Bool(false))",
        "Err(Int(1267650600228229401496703205376))",
        "Err(Float(-0.5))",
        "Err(String(\"oops\"))",
        "Err(BitArray(BitArray { bytes: [192], bit_length: 2 }))",
    ];
    for (selector, expected) in expected.iter().enumerate() {
        let text = match instance
            .call("fallible", &[Value::Float(selector as f64)])
            .unwrap()
        {
            Outcome::Ok(value) => format!("Ok({value:?})"),
            Outcome::Err(GuestError(value)) => format!("Err({value:?})"),
            Outcome::Stop(stop) => panic!("stopped: {stop}"),
        };
        assert_eq!(text, *expected);
        assert_eq!(allocations(&mut instance), 0);
    }
}

#[test]
fn identical_host_arcs_get_identical_ids_even_with_separate_externrefs() {
    let bytes = wat::parse_str(
        r#"(module
        (import "ilang:host/v1" "reference_id" (func $id (param externref) (result i64)))
        (func (export "ilang_start"))
        (func (export "same") (param externref externref) (result i32)
            (i64.eq (call $id (local.get 0)) (call $id (local.get 1)))))"#,
    )
    .unwrap();
    let mut abi = Abi::default();
    abi.exports.insert(
        "same".into(),
        Signature::new([AbiType::Externref, AbiType::Externref], AbiType::Bool),
    );
    let mut instance = Program::from_bytes(&bytes)
        .unwrap()
        .with_abi(abi)
        .unwrap()
        .instantiate(HostOptions::default())
        .unwrap();
    let reference = HostReference::new(42);
    let value = ok(instance
        .call(
            "same",
            &[
                Value::Externref(reference.clone()),
                Value::Externref(reference.clone()),
            ],
        )
        .unwrap());
    assert!(matches!(value, Value::Bool(true)));
}

#[test]
fn a_stop_in_ilang_start_is_reported_and_the_start_is_required() {
    let stopping = wat::parse_str(
        r#"(module
        (import "ilang:host/v1" "panic" (func $panic (param i32 i32)))
        (memory (export "memory") 1)
        (data (i32.const 0) "panic: early")
        (func (export "ilang_start") (call $panic (i32.const 0) (i32.const 12)) unreachable))"#,
    )
    .unwrap();
    let program = Program::from_bytes(&stopping).unwrap();
    let error = program.instantiate(HostOptions::default()).err().unwrap();
    let stop = error.downcast_ref::<Stop>().unwrap();
    assert_eq!(stop.message.as_deref(), Some("panic: early"));
    let none = wat::parse_str("(module)").unwrap();
    let error = Program::from_bytes(&none)
        .unwrap()
        .instantiate(HostOptions::default())
        .err()
        .unwrap();
    assert!(format!("{error:#}").contains("ilang_start"));
}
