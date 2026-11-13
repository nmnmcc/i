use super::*;
use std::sync::Mutex;

const FIXTURE: &[u8] = include_bytes!("../../../tests/browser/fixtures/host-abi.wasm");
const CUSTOM: &[u8] = include_bytes!("../../../tests/browser/fixtures/custom-imports.wasm");
const ONE_THREAD: &[u8] = include_bytes!("../../../tests/browser/fixtures/one-thread.wasm");

type Log = Arc<Mutex<Vec<String>>>;

/// The host functions of host-abi.wasm. `print` writes to the log, `read`
/// answers from a small table, `later` answers its argument, `hypot` works.
fn imports(log: &Log) -> HostImports {
    let printed = Arc::clone(log);
    imports_with(
        move |text| printed.lock().unwrap().push(text.to_owned()),
        |key| {
            Ok(match key {
                "name" => Ok(Value::String("i".into())),
                "fail" => bail!("the storage is gone"),
                "int" => Err(Value::Int(BigInt::from(1))),
                "wrong" => Ok(Value::Float(1.0)),
                "nan" => Err(Value::Float(f64::NAN)),
                _ => Err(Value::String(format!("no {key}"))),
            })
        },
        |arguments| {
            let value = arguments[0].clone();
            Box::pin(async move {
                match value {
                    Value::Float(x) if x < 0.0 => Ok(Err(Value::String("negative".into()))),
                    Value::Float(x) if x > 100.0 => bail!("the clock is broken"),
                    value => Ok(Ok(value)),
                }
            })
        },
    )
}

/// The host functions of host-abi.wasm, with these `print`, `read` and
/// `later`.
fn imports_with(
    print: impl Fn(&str) + Send + Sync + 'static,
    read: impl Fn(&str) -> Result<std::result::Result<Value, Value>> + Send + Sync + 'static,
    later: impl Fn(&[Value]) -> ServiceFuture + Send + Sync + 'static,
) -> HostImports {
    let mut imports = HostImports::default();
    imports
        .service("app:console/v1", "print", move |arguments| {
            let [Value::String(text)] = arguments else {
                bail!("print takes one String")
            };
            print(text);
            Ok(Ok(Value::Nil))
        })
        .unwrap();
    imports
        .service("app:storage/v1", "read", move |arguments| {
            let [Value::String(key)] = arguments else {
                bail!("read takes one String")
            };
            read(key)
        })
        .unwrap();
    imports
        .async_service("app:clock/v1", "later", later)
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
    // Keep legacy metadata coverage when the shared browser fixture is updated.
    let text = include_str!("../../../tests/browser/fixtures/host-abi.wat").replacen(
        r#"{\"version\":4,"#,
        r#"{\"version\":3,"#,
        1,
    );
    let legacy = wat::parse_str(text).unwrap();
    let abi = Program::from_bytes(&legacy).unwrap().abi().clone();
    assert_eq!(abi.version, ABI_OLDEST);
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
        r#"{"version":3,"entry":null,"tests":[],"exports":{},"services":{},"externals":{}}"#;
    assert!(parse(empty).is_ok());
    for json in [
        r#"{"version":1,"entry":null,"tests":[],"exports":{},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{},"services":{}}"#,
        r#"{"version":3,"tests":[],"exports":{},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{},"services":{},"externals":{},"imports":{}}"#,
        r#"{"version":3,"entry":"main","tests":[],"exports":{},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{"f":{"params":[],"result":"nil","errors":["string","int"],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{"f":{"params":[],"result":"nil","errors":["int","int"],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{"f":{"params":[],"result":"nil","errors":["externref"],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{"f":{"params":["list"],"result":"nil","errors":[],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{"ilang_start":{"params":[],"result":"nil","errors":[],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{"ilang_continue":{"params":[],"result":"nil","errors":[],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":{"name":"ilang_continue","async":true},"tests":[],"exports":{},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":{"name":"f","async":false},"tests":[],"exports":{"f":{"params":[],"result":"nil","errors":[],"async":false}},"services":{},"externals":{}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{},"services":{"ilang:host/v1":{}},"externals":{}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{},"services":{},"externals":{"m":{"f":{"params":[],"result":"nil","errors":["nil"]}}}}"#,
        r#"{"version":3,"entry":null,"tests":[],"exports":{},"services":{"m":{"f":{"params":[],"result":"nil"}}},"externals":{"m":{"f":{"params":[],"result":"nil"}}}}"#,
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

/// The runtime of `block_on` has IO when tokio is built with it (the dev
/// dependency turns on `net`), so an async service may use tokio IO.
#[test]
fn block_on_gives_services_tokio_io() {
    let later = |arguments: &[Value]| -> ServiceFuture {
        let value = arguments[0].clone();
        Box::pin(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let _client = tokio::net::TcpStream::connect(address).await?;
            listener.accept().await?;
            Ok(Ok(value))
        })
    };
    let mut instance = program()
        .instantiate(HostOptions::new(imports_with(
            |_| {},
            |_| Ok(Ok(Value::Nil)),
            later,
        )))
        .unwrap();
    let value = ok(block_on(instance.call_async("app/async.later", &[Value::Float(1.5)])).unwrap());
    assert!(matches!(value, Value::Float(2.5)));
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
    // Multicore engines are epoch-instrumented (native N12).
    store.set_epoch_deadline(1 << 62);
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

/// Deep enough for any recursion of `app/deep` to pass [`WASM_STACK`].
const TOO_DEEP: f64 = 100_000_000.0;

#[test]
fn deep_recursion_after_a_resume_stops_like_a_sync_call() {
    let depth = 100_000.0;
    let (mut instance, _) = fixture();
    let value = ok(instance.call("app/deep", &[Value::Float(depth)]).unwrap());
    assert!(matches!(value, Value::Float(x) if x == depth));
    let value =
        ok(block_on(instance.call_async("app/async.deep", &[Value::Float(depth)])).unwrap());
    assert!(matches!(value, Value::Float(x) if x == depth));

    let (mut instance, _) = fixture();
    let on_its_own_thread = stop(
        instance
            .call("app/deep", &[Value::Float(TOO_DEEP)])
            .unwrap(),
    );
    let (mut instance, _) = fixture();
    let on_ilang =
        stop(block_on(async { instance.call("app/deep", &[Value::Float(TOO_DEEP)]) }).unwrap());
    let (mut instance, _) = fixture();
    let resumed =
        stop(block_on(instance.call_async("app/async.deep", &[Value::Float(TOO_DEEP)])).unwrap());
    assert!(
        on_its_own_thread.trap.contains("call stack exhausted"),
        "{}",
        on_its_own_thread.trap
    );
    assert_eq!(on_its_own_thread.message, None);
    for other in [&on_ilang, &resumed] {
        assert_eq!(other.trap, on_its_own_thread.trap);
        assert_eq!(other.message, None);
    }
    assert!(instance.unusable().is_some());
}

/// `later` that answers its argument after a tokio timer of 1 ms. The timer
/// is made in the start closure, so it needs a tokio runtime there.
fn timer_later(arguments: &[Value]) -> ServiceFuture {
    let value = arguments[0].clone();
    let sleep = tokio::time::sleep(std::time::Duration::from_millis(1));
    Box::pin(async move {
        sleep.await;
        Ok(Ok(value))
    })
}

#[test]
fn block_on_works_inside_a_tokio_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let (mut instance, _) = fixture();
    let value = runtime
        .block_on(async { block_on(instance.call_async("app/async.later", &[Value::Float(1.5)])) });
    assert!(matches!(ok(value.unwrap()), Value::Float(2.5)));
    // Awaited on the caller's own runtime: every Wasm call takes the
    // fallback path.
    let value = runtime.block_on(instance.call_async("app/async.later", &[Value::Float(3.0)]));
    assert!(matches!(ok(value.unwrap()), Value::Float(4.0)));

    let mut instance = program()
        .instantiate(HostOptions::new(imports_with(
            |_| {},
            |_| Ok(Ok(Value::Nil)),
            timer_later,
        )))
        .unwrap();
    let value = runtime.block_on(instance.call_async("app/async.later", &[Value::Float(1.5)]));
    assert!(matches!(ok(value.unwrap()), Value::Float(2.5)));
    let value = block_on(instance.call_async("app/async.later", &[Value::Float(5.0)]));
    assert!(matches!(ok(value.unwrap()), Value::Float(6.0)));

    // Many timers on the fallback path: the reactor of the caller's runtime
    // answers them, the shortest first. Its clock is paused, so the order
    // does not depend on how fast the threads of the fallback start.
    let order = Order::default();
    let mut instance = program()
        .instantiate(HostOptions::new(imports_with(
            |_| {},
            |_| Ok(Ok(Value::Nil)),
            timers(4, &order),
        )))
        .unwrap();
    let value = runtime.block_on(async {
        tokio::time::pause();
        instance
            .call_async("app/async.fan", &[Value::Float(4.0)])
            .await
    });
    assert_eq!(*order.lock().unwrap(), [4, 3, 2, 1]);
    assert!(matches!(ok(value.unwrap()), Value::Float(h) if h == fold(&[4, 3, 2, 1])));
}

type Threads = Arc<Mutex<Vec<(&'static str, std::thread::ThreadId)>>>;

/// `print` and the start of `later` record the thread they run on.
fn thread_imports(threads: &Threads) -> HostImports {
    let mut imports = HostImports::default();
    let printed = Arc::clone(threads);
    imports
        .service("app:console/v1", "print", move |_| {
            printed
                .lock()
                .unwrap()
                .push(("print", std::thread::current().id()));
            Ok(Ok(Value::Nil))
        })
        .unwrap();
    let started = Arc::clone(threads);
    imports
        .async_service("app:clock/v1", "later", move |arguments| {
            started
                .lock()
                .unwrap()
                .push(("later", std::thread::current().id()));
            let value = arguments[0].clone();
            async move { Ok(Ok(value)) }
        })
        .unwrap();
    imports
}

#[test]
fn run_entry_uses_one_thread() {
    let threads = Threads::default();
    let options = HostOptions::new(thread_imports(&threads));
    let program = Program::from_bytes(ONE_THREAD).unwrap();
    let check = |threads: &Threads| {
        let recorded = std::mem::take(&mut *threads.lock().unwrap());
        let count = |name| recorded.iter().filter(|(kind, _)| *kind == name).count();
        assert_eq!((count("print"), count("later")), (4, 3), "{recorded:?}");
        let first = recorded[0].1;
        assert_ne!(first, std::thread::current().id());
        assert!(recorded.iter().all(|(_, id)| *id == first), "{recorded:?}");
    };
    assert!(matches!(
        program.run_entry(options.clone()).unwrap(),
        Outcome::Ok(Value::Nil)
    ));
    check(&threads);
    assert!(matches!(
        program.run_test_with("app/test.waits", options.clone()),
        TestResult::Passed
    ));
    check(&threads);
    let reports = program.run_tests(&options);
    assert_eq!(reports.len(), 1);
    assert!(reports[0].passed());
    check(&threads);
}

#[test]
fn futures_are_send() {
    fn send<T: Send>(_: &T) {}
    fn send_type<T: Send>() {}
    let (mut instance, _) = fixture();
    {
        let future = instance.run_async("app/async_test.waits");
        send(&future);
    }
    {
        let future = instance.call_async("app/async.later", &[Value::Float(1.0)]);
        send(&future);
    }
    send_type::<ServiceFuture>();
    send_type::<Outcome>();
    send_type::<TestResult>();
    send_type::<Value>();
    send_type::<Instance>();
}

#[test]
fn a_nested_store_gets_its_own_thread() {
    let (mut top, _) = fixture();
    let expected = stop(top.call("app/deep", &[Value::Float(TOO_DEEP)]).unwrap());

    let inner_threads = Threads::default();
    let recorded = Arc::clone(&inner_threads);
    let inner = program()
        .instantiate(HostOptions::new(imports_with(
            move |_| {
                recorded
                    .lock()
                    .unwrap()
                    .push(("print", std::thread::current().id()));
            },
            |_| Ok(Ok(Value::Nil)),
            |_| Box::pin(async { Ok(Ok(Value::Nil)) }),
        )))
        .unwrap();
    inner_threads.lock().unwrap().clear();
    let inner = Mutex::new(inner);
    let outer_thread = Arc::new(Mutex::new(None));
    let inner_trap = Arc::new(Mutex::new(None));
    let (outer_seen, trap_seen) = (Arc::clone(&outer_thread), Arc::clone(&inner_trap));
    let mut outer = program()
        .instantiate(HostOptions::new(imports_with(
            |_| {},
            move |_| {
                let current = std::thread::current();
                *outer_seen.lock().unwrap() =
                    Some((current.name().map(str::to_owned), current.id()));
                let mut inner = inner.lock().unwrap();
                ensure!(matches!(inner.run("app/main.main")?, Outcome::Ok(_)));
                let Outcome::Stop(stop) = inner.call("app/deep", &[Value::Float(TOO_DEEP)])? else {
                    bail!("the inner call did not stop")
                };
                *trap_seen.lock().unwrap() = Some(stop);
                Ok(Ok(Value::String("i".into())))
            },
            |_| Box::pin(async { Ok(Ok(Value::Nil)) }),
        )))
        .unwrap();
    let value = ok(
        block_on(async { outer.call("read_through", &[Value::String("name".into())]) }).unwrap(),
    );
    assert!(matches!(value, Value::String(text) if text == "i"));
    let (name, outer_id) = outer_thread.lock().unwrap().clone().unwrap();
    assert_eq!(name.as_deref(), Some("ilang"));
    let inner_prints = inner_threads.lock().unwrap().clone();
    assert_eq!(inner_prints.len(), 1);
    assert_ne!(inner_prints[0].1, outer_id);
    let trap = inner_trap.lock().unwrap().take().unwrap();
    assert!(
        expected.trap.contains("call stack exhausted"),
        "{}",
        expected.trap
    );
    assert_eq!(trap.trap, expected.trap);
    assert_eq!(trap.message, None);
}

const NO_CONTINUE: &[u8] = include_bytes!("../../../tests/browser/fixtures/no-continue.wasm");
const WRONG_TYPES: &[u8] = include_bytes!("../../../tests/browser/fixtures/wrong-types.wasm");
const COMPILED: &[u8] = include_bytes!("../../../tests/browser/fixtures/compiled.wasm");

/// The hash that host-abi.wasm folds over the arguments of the answers, in
/// the order they came: h = (h * 31 + x) mod (2^31 - 1).
fn fold(arguments: &[i64]) -> f64 {
    let mut h: i64 = 0;
    for x in arguments {
        h = (h * 31 + x) % 2_147_483_647;
    }
    h as f64
}

/// The arguments of `later`, in the order their answers were given.
type Order = Arc<Mutex<Vec<i64>>>;

/// `later` that answers 2x after a tokio timer of (n + 1 - x) ms, made in
/// the start closure, and records x when it answers.
fn timers(n: i64, order: &Order) -> impl Fn(&[Value]) -> ServiceFuture + Send + Sync + 'static {
    let order = Arc::clone(order);
    move |arguments| {
        let [Value::Float(x)] = arguments else {
            panic!("later takes one Float")
        };
        let x = *x;
        let delay = (n + 1) as f64 - x;
        assert!(delay >= 1.0, "a delay of at least 1 ms");
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(delay as u64);
        let order = Arc::clone(&order);
        Box::pin(async move {
            tokio::time::sleep_until(deadline).await;
            order.lock().unwrap().push(x as i64);
            Ok(Ok(Value::Float(2.0 * x)))
        })
    }
}

/// A test service for `later` whose order of answers a seeded random choice
/// decides, with no timers. Each request is a gate. When a gate is polled
/// and no gate is released, one gate (two with `burst`) among the live ones
/// is released; a released gate answers 2x when it is polled. A gate whose
/// argument is 0 is held: it is never released.
#[derive(Default)]
struct GateState {
    rng: u64,
    burst: bool,
    /// Releases nothing at all.
    hold_all: bool,
    /// The argument whose gate fails in the host.
    fail: Option<i64>,
    /// The first answer is a String, not a Float.
    wrong_type_first: bool,
    next: u64,
    live: Vec<u64>,
    released: Vec<u64>,
    wakers: HashMap<u64, std::task::Waker>,
    starts: usize,
    /// The arguments of the gates that answered, in order.
    completed: Vec<i64>,
    drops: usize,
    /// The gates dropped before they answered: the argument, and how many
    /// gates had answered then.
    unfinished: Vec<(i64, usize)>,
    /// Gates dropped while they were released and had not answered.
    dropped_released: usize,
}

impl GateState {
    fn random(&mut self) -> usize {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng as usize
    }

    fn wake_all(&mut self) {
        for (_, waker) in self.wakers.drain() {
            waker.wake();
        }
    }
}

type Gates = Arc<Mutex<GateState>>;

fn gates(seed: u64) -> Gates {
    Arc::new(Mutex::new(GateState {
        rng: seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1,
        ..GateState::default()
    }))
}

struct Gate {
    key: u64,
    x: i64,
    state: Gates,
    done: bool,
}

impl Future for Gate {
    type Output = Result<std::result::Result<Value, Value>>;

    fn poll(
        mut self: Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let state = Arc::clone(&self.state);
        let mut state = state.lock().unwrap();
        if state.released.is_empty() && !state.live.is_empty() && !state.hold_all {
            let count = if state.burst { 2 } else { 1 }.min(state.live.len());
            for _ in 0..count {
                let index = state.random() % state.live.len();
                let key = state.live.remove(index);
                state.released.push(key);
                if let Some(waker) = state.wakers.remove(&key) {
                    waker.wake();
                }
            }
        }
        let Some(index) = state.released.iter().position(|key| *key == self.key) else {
            state.wakers.insert(self.key, context.waker().clone());
            return std::task::Poll::Pending;
        };
        state.released.remove(index);
        self.done = true;
        let first = state.completed.is_empty();
        state.completed.push(self.x);
        if state.released.is_empty() {
            state.wake_all();
        }
        std::task::Poll::Ready(if state.fail == Some(self.x) {
            Err(anyhow!("the gate failed"))
        } else if first && state.wrong_type_first {
            Ok(Ok(Value::String("wrong".into())))
        } else {
            Ok(Ok(Value::Float(2.0 * self.x as f64)))
        })
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.drops += 1;
        state.wakers.remove(&self.key);
        if self.done {
            return;
        }
        let answered = state.completed.len();
        state.unfinished.push((self.x, answered));
        state.live.retain(|key| *key != self.key);
        if let Some(index) = state.released.iter().position(|key| *key == self.key) {
            state.released.remove(index);
            state.dropped_released += 1;
            if state.released.is_empty() {
                state.wake_all();
            }
        }
    }
}

/// The host functions of host-abi.wasm, with gates for `later`.
fn gate_imports(gates: &Gates) -> HostImports {
    let state = Arc::clone(gates);
    imports_with(
        |_| {},
        |key| {
            Ok(match key {
                "name" => Ok(Value::String("i".into())),
                _ => Err(Value::String(format!("no {key}"))),
            })
        },
        move |arguments| {
            let x = match arguments {
                [Value::Float(x)] => *x as i64,
                _ => -1,
            };
            let mut locked = state.lock().unwrap();
            locked.starts += 1;
            let key = locked.next;
            locked.next += 1;
            if x != 0 {
                locked.live.push(key);
            }
            Box::pin(Gate {
                key,
                x,
                state: Arc::clone(&state),
                done: false,
            })
        },
    )
}

fn gated(program: &Program, gates: &Gates) -> Instance {
    program
        .instantiate(HostOptions::new(gate_imports(gates)))
        .unwrap()
}

fn float(outcome: Outcome) -> f64 {
    match ok(outcome) {
        Value::Float(x) => x,
        other => panic!("expected a Float, got {other:?}"),
    }
}

fn error_text(result: Result<Outcome>) -> String {
    match result {
        Ok(outcome) => panic!("expected an error, got {outcome:?}"),
        Err(error) => format!("{error:#}"),
    }
}

#[test]
fn many_requests_complete_out_of_order() {
    let program = program();
    for seed in 0..200 {
        let n = 1 + seed % 16;
        let gates = gates(seed);
        let mut instance = gated(&program, &gates);
        let h = float(
            block_on(instance.call_async("app/async.many", &[Value::Float(n as f64)])).unwrap(),
        );
        {
            let state = gates.lock().unwrap();
            assert_eq!(h, fold(&state.completed), "seed {seed}");
            assert_eq!(state.starts, 2 * n as usize, "seed {seed}");
            assert_eq!(state.drops, 2 * n as usize, "seed {seed}");
            assert!(state.unfinished.is_empty(), "seed {seed}");
        }
        // The ids are used again in a second call.
        gates.lock().unwrap().completed.clear();
        let h =
            float(block_on(instance.call_async("app/async.many", &[Value::Float(2.0)])).unwrap());
        assert_eq!(h, fold(&gates.lock().unwrap().completed), "seed {seed}");
        assert!(instance.unusable().is_none());
    }
}

#[test]
fn timers_answer_through_the_reactor() {
    let program = program();
    let n = 8;
    for _ in 0..200 {
        let order = Order::default();
        let mut instance = program
            .instantiate(HostOptions::new(imports_with(
                |_| {},
                |_| Ok(Ok(Value::Nil)),
                timers(n, &order),
            )))
            .unwrap();
        let h = float(
            block_on(async {
                tokio::time::pause();
                instance
                    .call_async("app/async.fan", &[Value::Float(n as f64)])
                    .await
            })
            .unwrap(),
        );
        let expected: Vec<i64> = (1..=n).rev().collect();
        assert_eq!(*order.lock().unwrap(), expected);
        assert_eq!(h, fold(&expected));
    }
}

#[test]
fn a_stop_drops_outstanding_requests() {
    let gates = gates(7);
    let mut instance = gated(&program(), &gates);
    let stop = stop(block_on(instance.call_async("app/async.stop_mid", &[])).unwrap());
    assert_eq!(stop.message.as_deref(), Some("fixture: stop"));
    {
        let state = gates.lock().unwrap();
        assert_eq!(
            (state.starts, state.drops, state.unfinished.len()),
            (3, 3, 2)
        );
    }
    assert!(instance.call("nil_roundtrip", &[]).is_err());
}

#[test]
fn a_host_failure_drops_the_others() {
    let gates = gates(3);
    gates.lock().unwrap().fail = Some(2);
    let mut instance = gated(&program(), &gates);
    let text = error_text(block_on(
        instance.call_async("app/async.fan", &[Value::Float(5.0)]),
    ));
    assert_eq!(text, "the gate failed");
    {
        let state = gates.lock().unwrap();
        assert_eq!(state.drops, 5);
        assert_eq!(state.starts, 5);
        assert_eq!(state.unfinished.len(), 5 - state.completed.len());
    }
    assert!(instance.unusable().is_some());
    assert!(instance.call("nil_roundtrip", &[]).is_err());
}

#[test]
fn status_one_with_nothing_outstanding_is_a_fault() {
    let gates = gates(1);
    let mut instance = gated(&program(), &gates);
    let text = error_text(block_on(instance.call_async("app/async.stuck", &[])));
    assert_eq!(text, "the module waits, but no host function is running");
    assert_eq!(instance.unusable(), Some(text.as_str()));
}

#[test]
fn duplicate_or_negative_id_traps() {
    for (name, text, starts) in [
        ("app/async.dup", "the module started request 7 twice", 1),
        (
            "app/async.negative",
            "the module started request -1, which is below 0",
            0,
        ),
    ] {
        let gates = gates(1);
        let mut instance = gated(&program(), &gates);
        assert_eq!(error_text(block_on(instance.call_async(name, &[]))), text);
        assert!(instance.unusable().is_some());
        let state = gates.lock().unwrap();
        assert_eq!(state.starts, starts, "{name}");
        assert_eq!(state.drops, starts, "{name}");
    }
}

#[test]
fn a_sync_call_may_not_start_requests() {
    let program = program();
    let started = |id: i32| format!("the module started request {id} outside an async call");
    let cancelled = |id: i32| format!("the module cancelled request {id} outside an async call");
    // Sync exports, and imports made from ilang_alloc during an import.
    for (name, text, starts) in [
        ("app/sync.starts", started(3), 0),
        ("app/sync.cancels", cancelled(0), 0),
        ("app/async.nested", started(77), 0),
        ("app/async.nested_cancel", cancelled(5), 1),
        ("app/async.result_starts", started(6), 1),
        ("app/async.result_cancels", cancelled(0), 1),
    ] {
        let gates = gates(1);
        let mut instance = gated(&program, &gates);
        let result = if name.starts_with("app/sync.") {
            instance.call(name, &[])
        } else {
            block_on(instance.call_async(name, &[]))
        };
        assert_eq!(error_text(result), text, "{name}");
        assert_eq!(instance.unusable(), Some(text.as_str()), "{name}");
        assert_eq!(gates.lock().unwrap().starts, starts, "{name}");
    }
    // ilang_start.
    let bytes = wat::parse_str(
        r#"(module
        (import "app:clock/v1" "later" (func $later (param i32 f64)))
        (func (export "ilang_start") (call $later (i32.const 0) (f64.const 1))))"#,
    )
    .unwrap();
    let mut abi = Abi::default();
    abi.services.insert(
        "app:clock/v1".into(),
        BTreeMap::from([(
            "later".to_owned(),
            Signature::new([AbiType::Float], AbiType::Float).asynchronous(),
        )]),
    );
    let gates = gates(1);
    let error = Program::from_bytes(&bytes)
        .unwrap()
        .with_abi(abi)
        .unwrap()
        .instantiate(HostOptions::new(gate_imports(&gates)))
        .err()
        .unwrap();
    assert_eq!(format!("{error:#}"), started(0));
    assert_eq!(gates.lock().unwrap().starts, 0);
}

#[test]
fn leftovers_are_dropped_at_status_zero() {
    let program = program();
    let gates = gates(5);
    let mut instance = gated(&program, &gates);
    let h = float(block_on(instance.call_async("app/async.leftover", &[])).unwrap());
    {
        let state = gates.lock().unwrap();
        assert_eq!(state.completed.len(), 1);
        assert_eq!(h, fold(&state.completed));
        assert_eq!(
            (state.starts, state.drops, state.unfinished.len()),
            (3, 3, 2)
        );
    }
    assert!(instance.unusable().is_none());
    gates.lock().unwrap().completed.clear();
    let h = float(block_on(instance.call_async("app/async.many", &[Value::Float(3.0)])).unwrap());
    assert_eq!(h, fold(&gates.lock().unwrap().completed));
}

/// Calls an async export of an instance directly, as if the host ran it,
/// so that a test can then make calls that a host never makes.
fn raw_async(instance: &mut Instance, name: &str, params: &[Val]) -> i32 {
    instance.store.data_mut().requests.set_accepting(true);
    let function = instance
        .instance
        .get_func(&mut instance.store, name)
        .unwrap();
    let mut status = [Val::I32(0)];
    function
        .call(&mut instance.store, params, &mut status)
        .unwrap();
    instance.store.data_mut().requests.set_accepting(false);
    status[0].unwrap_i32()
}

#[test]
fn a_wrong_resume_traps() {
    let resume = "ilang_resume:app:clock/v1:later";
    let call = |instance: &mut Instance, name: &str, params: &[Val]| {
        let function = instance
            .instance
            .get_func(&mut instance.store, name)
            .unwrap();
        let mut status = [Val::I32(0)];
        let error = function
            .call(&mut instance.store, params, &mut status)
            .unwrap_err();
        assert!(format!("{error:?}").contains("unreachable"), "{error:?}");
        assert_eq!(instance.store.data().stop_message, None);
    };
    let answer = |id: i32| {
        [
            Val::I32(id),
            Val::I32(0),
            Val::F64(2_f64.to_bits()),
            Val::I32(0),
            Val::I32(0),
        ]
    };
    // An id that is not outstanding.
    let gates = gates(1);
    let mut instance = gated(&program(), &gates);
    assert_eq!(
        raw_async(
            &mut instance,
            "app/async.later",
            &[Val::F64(1_f64.to_bits())]
        ),
        1
    );
    call(&mut instance, resume, &answer(9));
    // An id that was answered already.
    let mut instance = gated(&program(), &gates);
    assert_eq!(
        raw_async(
            &mut instance,
            "app/async.later",
            &[Val::F64(1_f64.to_bits())]
        ),
        1
    );
    instance.store.data_mut().requests.set_accepting(true);
    let function = instance
        .instance
        .get_func(&mut instance.store, resume)
        .unwrap();
    let mut status = [Val::I32(0)];
    function
        .call(&mut instance.store, &answer(0), &mut status)
        .unwrap();
    instance.store.data_mut().requests.set_accepting(false);
    assert_eq!(status[0].unwrap_i32(), 1);
    call(&mut instance, resume, &answer(0));
    // A module built by the compiler, answered for another import.
    let mut imports = gate_imports(&gates);
    imports.merge(opaque_imports(Value::NullExternref)).unwrap();
    imports
        .async_service("app:clock/v1", "tick", |_| async {
            Ok(Ok(Value::Int(1.into())))
        })
        .unwrap();
    let mut compiled = Program::from_bytes(COMPILED)
        .unwrap()
        .instantiate(HostOptions::new(imports))
        .unwrap();
    let two = [Val::F64(1_f64.to_bits()), Val::F64(2_f64.to_bits())];
    assert_eq!(raw_async(&mut compiled, "app/app.wait_sum", &two), 1);
    call(
        &mut compiled,
        "ilang_resume:app:clock/v1:tick",
        &[Val::I32(0), Val::I32(1), Val::I32(0), Val::I32(0)],
    );
}

#[test]
fn dropping_a_waiting_call_makes_the_instance_unusable() {
    let gates = gates(1);
    gates.lock().unwrap().hold_all = true;
    let mut instance = gated(&program(), &gates);
    {
        let mut call = Box::pin(instance.call_async("app/async.many", &[Value::Float(3.0)]));
        let polled = call
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()));
        assert!(polled.is_pending());
        assert_eq!(gates.lock().unwrap().drops, 0);
    }
    {
        let state = gates.lock().unwrap();
        assert_eq!(
            (state.starts, state.drops, state.unfinished.len()),
            (3, 3, 3)
        );
    }
    assert_eq!(
        instance.unusable(),
        Some("an async call was dropped while it waited")
    );
    assert!(instance.call("nil_roundtrip", &[]).is_err());
}

#[test]
fn old_and_new_versions_are_refused() {
    let json = |version: u32| {
        format!(
            r#"{{"version":{version},"entry":null,"tests":[],"exports":{{}},"services":{{}},"externals":{{}}}}"#
        )
    };
    for version in [1, ABI_OLDEST - 1] {
        let error = Abi::from_json(json(version).as_bytes()).unwrap_err();
        assert_eq!(
            format!("{error:#}"),
            format!(
                "the module uses i ABI version {version}; build it again with this version of i"
            )
        );
    }
    let error = Abi::from_json(json(ABI_VERSION + 1).as_bytes()).unwrap_err();
    assert_eq!(
        format!("{error:#}"),
        format!(
            "the module uses i ABI version {}, but this host reads version {ABI_VERSION}; update the host",
            ABI_VERSION + 1
        )
    );
    assert!(Abi::from_json(json(ABI_VERSION).as_bytes()).is_ok());
}

#[test]
fn a_sync_service_may_block_on_its_own_thread() {
    let program = Program::from_bytes(ONE_THREAD).unwrap();
    let options = |own_thread: bool| {
        let mut imports = HostImports::default();
        imports
            .service("app:console/v1", "print", move |_| {
                let work = || {
                    tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap()
                        .block_on(async { 1 })
                };
                let value = if own_thread {
                    std::thread::spawn(work).join().unwrap()
                } else {
                    work()
                };
                ensure!(value == 1);
                Ok(Ok(Value::Nil))
            })
            .unwrap();
        imports
            .async_service("app:clock/v1", "later", |arguments| {
                let value = arguments[0].clone();
                async move { Ok(Ok(value)) }
            })
            .unwrap();
        HostOptions::new(imports)
    };
    assert!(matches!(
        program.run_entry(options(true)).unwrap(),
        Outcome::Ok(Value::Nil)
    ));
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        program.run_entry(options(false))
    }))
    .unwrap_err();
    let text = panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
        .unwrap();
    assert!(
        text.contains("Cannot start a runtime from within a runtime"),
        "{text}"
    );
}

#[test]
fn cancel_drops_one_request() {
    let program = program();
    for seed in 0..200 {
        let n = 2 + seed % 15;
        let gates = gates(seed);
        let mut instance = gated(&program, &gates);
        let h = float(
            block_on(instance.call_async("app/async.cancel", &[Value::Float(n as f64)])).unwrap(),
        );
        let state = gates.lock().unwrap();
        assert_eq!(h, fold(&state.completed), "seed {seed}");
        assert_eq!((state.starts, state.drops), (n as usize, n as usize));
        let cancelled = n as usize / 2;
        assert_eq!(state.completed.len(), n as usize - cancelled, "seed {seed}");
        // Each was dropped inside the resume of the answer that cancelled it.
        let answered_then: Vec<usize> = state.unfinished.iter().map(|(_, at)| *at).collect();
        assert_eq!(
            answered_then,
            (1..=cancelled).collect::<Vec<_>>(),
            "seed {seed}"
        );
        assert!(instance.unusable().is_none());
    }
    let gates = gates(1);
    let mut instance = gated(&program, &gates);
    assert_eq!(
        error_text(block_on(instance.call_async("app/async.cancel_bad", &[]))),
        "the module cancelled request 12345, which is not outstanding"
    );
    assert!(instance.unusable().is_some());
}

#[test]
fn status_two_gives_ready_answers() {
    let program = program();
    let mut imports = imports(&Log::default());
    imports = {
        let mut ready = HostImports {
            unrestricted: imports.unrestricted,
            grants: imports.grants,
            ..HostImports::default()
        };
        for ((module, name), function) in imports.functions {
            if name != "later" {
                ready.insert(module, name, function).unwrap();
            }
        }
        ready
            .async_service("app:clock/v1", "later", |arguments| {
                let [Value::Float(x)] = arguments else {
                    panic!("later takes one Float")
                };
                let answer = Value::Float(2.0 * x);
                async move { Ok(Ok(answer)) }
            })
            .unwrap();
        ready
    };
    let mut instance = program.instantiate(HostOptions::new(imports)).unwrap();
    let count = float(
        block_on(instance.call_async("app/async.yielding", &[Value::Float(1000.0)])).unwrap(),
    );
    assert!((1.0..1000.0).contains(&count), "{count}");
    // A timer in real time: the yield lets the timer driver run.
    let order = Order::default();
    let mut instance = program
        .instantiate(HostOptions::new(imports_with(
            |_| {},
            |_| Ok(Ok(Value::Nil)),
            timers(1, &order),
        )))
        .unwrap();
    let count =
        float(block_on(instance.call_async("app/async.yielding", &[Value::Float(1e7)])).unwrap());
    assert!((1.0..1e7).contains(&count), "{count}");
    assert_eq!(*order.lock().unwrap(), [1]);
}

#[test]
fn every_return_from_wait_drops_the_requests() {
    let gates = gates(2);
    gates.lock().unwrap().wrong_type_first = true;
    let mut instance = gated(&program(), &gates);
    let stop = stop(block_on(instance.call_async("app/async.fan", &[Value::Float(3.0)])).unwrap());
    assert_eq!(stop.message.as_deref(), Some(HOST_VALUE_REASON));
    {
        let state = gates.lock().unwrap();
        assert_eq!(
            (state.starts, state.drops, state.unfinished.len()),
            (3, 3, 2)
        );
    }
    assert_eq!(instance.unusable(), Some(HOST_VALUE_REASON));
}

#[test]
fn cancel_and_reuse_with_a_finished_answer() {
    let program = program();
    let mut dropped_released = 0;
    for seed in 0..200 {
        let n = 2 + seed % 15;
        let gates = gates(seed);
        gates.lock().unwrap().burst = true;
        let mut instance = gated(&program, &gates);
        let outcome =
            block_on(instance.call_async("app/async.cancel_reuse", &[Value::Float(n as f64)]))
                .unwrap();
        let h = float(outcome);
        let state = gates.lock().unwrap();
        assert_eq!(h, fold(&state.completed), "seed {seed}");
        assert_eq!(state.drops, state.starts, "seed {seed}");
        assert_eq!(state.completed.len() + state.unfinished.len(), state.starts);
        dropped_released += state.dropped_released;
    }
    assert!(dropped_released > 0);
}

#[test]
fn requests_from_ilang_continue() {
    let program = program();
    for seed in 0..200 {
        let n = 2 + seed % 8;
        let gates = gates(seed);
        let mut instance = gated(&program, &gates);
        let h = float(
            block_on(instance.call_async("app/async.continue_io", &[Value::Float(n as f64)]))
                .unwrap(),
        );
        let state = gates.lock().unwrap();
        assert_eq!(h, fold(&state.completed), "seed {seed}");
        assert_eq!(state.starts, n as usize);
        assert_eq!(state.completed.len(), n as usize - 1);
        // The held request was cancelled inside the second ilang_continue.
        assert_eq!(state.unfinished.len(), 1);
        assert_eq!(state.unfinished[0].0, 0);
        assert!(!state.completed.contains(&0));
    }
}

fn fault_imports(gates: &Gates) -> HostImports {
    let mut imports = gate_imports(gates);
    imports
        .async_service("app:clock/v1", "ready", |_| async { Ok(Ok(Value::Nil)) })
        .unwrap();
    imports
}

/// A module must have the export that answers a request when a request starts,
/// or the export `ilang_continue` when status 2 is returned,
/// even when an answer is ready, the entry ends, or the request is cancelled.
#[test]
fn required_exports_are_checked_before_delivery_or_drop() {
    for (bytes, bad_resume) in [(NO_CONTINUE, "no_resume"), (WRONG_TYPES, "bad_resume")] {
        let program = Program::from_bytes(bytes).unwrap();
        for suffix in [
            "runnable_ready".to_owned(),
            format!("{bad_resume}_done"),
            format!("{bad_resume}_cancel"),
        ] {
            let gates = gates(1);
            let mut instance = program
                .instantiate(HostOptions::new(fault_imports(&gates)))
                .unwrap();
            let text = error_text(block_on(
                instance.call_async(&format!("app/async.{suffix}"), &[]),
            ));
            let expected = if suffix == "runnable_ready" {
                "the module is runnable, but has no ilang_continue"
            } else {
                "the module does not export ilang_resume:app:clock/v1:later"
            };
            assert_eq!(text, expected, "{suffix}");
            assert_eq!(instance.unusable(), Some(expected), "{suffix}");
            assert!(instance.call("app/sync.nil", &[]).is_err(), "{suffix}");
            // Neither an answer, a result export, nor the refused sync call ran.
            assert_eq!(raw(&mut instance, "calls"), 0, "{suffix}");
            assert!(instance.store.data().requests.is_empty());
            if suffix != "runnable_ready" {
                let state = gates.lock().unwrap();
                assert_eq!(state.starts, 1);
                assert_eq!(state.drops, 1);
                assert!(state.completed.is_empty());
            }
        }
    }
}

#[test]
fn bad_statuses_are_faults() {
    let gates = gates(1);
    let mut instance = gated(&program(), &gates);
    let text = error_text(block_on(instance.call_async("app/async.bad_status", &[])));
    assert_eq!(
        text,
        "export app/async.bad_status gave the status Some(7), which is not 0, 1 or 2"
    );
    assert!(instance.unusable().is_some());

    let no_continue = Program::from_bytes(NO_CONTINUE).unwrap();
    let mut instance = no_continue
        .instantiate(HostOptions::new(fault_imports(&gates)))
        .unwrap();
    let text = error_text(block_on(instance.call_async("app/async.runnable", &[])));
    assert_eq!(text, "the module is runnable, but has no ilang_continue");
    assert_eq!(instance.unusable(), Some(text.as_str()));

    let gates = self::gates(1);
    let mut instance = no_continue
        .instantiate(HostOptions::new(fault_imports(&gates)))
        .unwrap();
    let text = error_text(block_on(instance.call_async("app/async.no_resume", &[])));
    assert_eq!(
        text,
        "the module does not export ilang_resume:app:clock/v1:later"
    );
    assert_eq!(instance.unusable(), Some(text.as_str()));
    assert!(instance.call("app/sync.nil", &[]).is_err());
    assert_eq!(raw(&mut instance, "calls"), 0);
    assert_eq!(gates.lock().unwrap().drops, 1);

    // ilang_continue right after status 1 traps.
    let mut instance = gated(&program(), &gates);
    assert_eq!(
        raw_async(
            &mut instance,
            "app/async.later",
            &[Val::F64(1_f64.to_bits())]
        ),
        1
    );
    assert!(
        instance
            .instance
            .get_typed_func::<(), i32>(&mut instance.store, "ilang_continue")
            .unwrap()
            .call(&mut instance.store, ())
            .is_err()
    );
}

/// `ilang_continue` and `ilang_resume:…` exports of the wrong type are
/// faults, like missing ones, and the host never calls them.
#[test]
fn exports_of_the_wrong_type_are_faults() {
    let wrong = Program::from_bytes(WRONG_TYPES).unwrap();
    for (name, text, drops) in [
        (
            "app/async.runnable",
            "the module is runnable, but has no ilang_continue",
            0,
        ),
        (
            "app/async.bad_resume",
            "the module does not export ilang_resume:app:clock/v1:later",
            1,
        ),
    ] {
        let gates = gates(1);
        let mut instance = wrong
            .instantiate(HostOptions::new(fault_imports(&gates)))
            .unwrap();
        let error = error_text(block_on(instance.call_async(name, &[])));
        assert_eq!(error, text, "{name}");
        assert_eq!(instance.unusable(), Some(text), "{name}");
        assert!(instance.call("app/sync.nil", &[]).is_err(), "{name}");
        assert_eq!(raw(&mut instance, "calls"), 0, "{name}");
        assert_eq!(gates.lock().unwrap().drops, drops, "{name}");
    }
}

#[test]
fn a_finished_program_does_not_wait_for_blocking_work() {
    for name in ["app/async.leftover", "app/async.stop_mid"] {
        let running = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let starts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (seen, counted) = (Arc::clone(&running), Arc::clone(&starts));
        let options = HostOptions::new(imports_with(
            |_| {},
            |_| Ok(Ok(Value::Nil)),
            move |arguments| {
                let answer = match arguments {
                    [Value::Float(x)] => Value::Float(2.0 * x),
                    _ => Value::Nil,
                };
                let running = Arc::clone(&seen);
                if counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    // The first request answers once both blocking closures run.
                    return Box::pin(async move {
                        while running.load(std::sync::atomic::Ordering::SeqCst) < 2 {
                            tokio::task::yield_now().await;
                        }
                        Ok(Ok(answer))
                    });
                }
                let blocking = tokio::task::spawn_blocking(move || {
                    running.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_secs(30));
                });
                Box::pin(async move {
                    blocking.await?;
                    Ok(Ok(answer))
                })
            },
        ));
        let mut instance = program().instantiate(options).unwrap();
        let begin = std::time::Instant::now();
        let outcome = block_on(instance.call_async(name, &[])).unwrap();
        assert!(
            begin.elapsed() < std::time::Duration::from_secs(5),
            "{name}"
        );
        assert_eq!(
            running.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "{name}"
        );
        match (name, outcome) {
            ("app/async.leftover", Outcome::Ok(Value::Float(_))) => {}
            ("app/async.stop_mid", Outcome::Stop(stop)) => {
                assert_eq!(stop.message.as_deref(), Some("fixture: stop"));
            }
            (_, other) => panic!("{name}: {other:?}"),
        }
    }
}

/// A request whose work wakes itself during its own poll must not be
/// polled again in the same call; the wait must give the
/// executor a turn first.
mod self_waking_work {
    use crate::requests::{Request, Requests};
    use crate::{AbiType, ServiceFuture, Signature, Value};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    use std::task::Poll;

    /// How many times the busy work may be polled before it gives up.
    const LIMIT: usize = 1_000_000;

    fn request(label: &str) -> Request {
        Request {
            namespace: "test".into(),
            label: label.into(),
            signature: Signature::new([], AbiType::Nil),
        }
    }

    /// Work that wakes itself and stays pending until `flag` is set. It
    /// gives up after LIMIT polls and records that in `gave_up`.
    fn busy(
        flag: Arc<AtomicBool>,
        polls: Arc<AtomicUsize>,
        gave_up: Arc<AtomicBool>,
    ) -> ServiceFuture {
        Box::pin(std::future::poll_fn(move |context| {
            let n = polls.fetch_add(1, Ordering::SeqCst) + 1;
            if flag.load(Ordering::SeqCst) {
                return Poll::Ready(Ok(Ok(Value::Nil)));
            }
            if n >= LIMIT {
                gave_up.store(true, Ordering::SeqCst);
                return Poll::Ready(Ok(Ok(Value::Nil)));
            }
            context.waker().wake_by_ref();
            Poll::Pending
        }))
    }

    #[test]
    fn ready_polls_self_waking_work_once_per_call() {
        let flag = Arc::new(AtomicBool::new(false));
        let polls = Arc::new(AtomicUsize::new(0));
        let gave_up = Arc::new(AtomicBool::new(false));
        let mut requests = Requests::default();
        requests.insert(
            0,
            request("busy"),
            busy(flag, Arc::clone(&polls), Arc::clone(&gave_up)),
        );
        let done = requests.ready();
        assert!(
            done.is_none(),
            "ready() should give nothing for work that is still pending, \
             but the work was polled {} times until it gave up",
            polls.load(Ordering::SeqCst)
        );
        assert_eq!(
            polls.load(Ordering::SeqCst),
            1,
            "one call of ready() polls the work once"
        );
        assert!(!gave_up.load(Ordering::SeqCst));
    }

    #[test]
    fn next_lets_a_timer_fire_while_other_work_spins() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        let polls = Arc::new(AtomicUsize::new(0));
        let gave_up = Arc::new(AtomicBool::new(false));
        let mut requests = Requests::default();
        requests.insert(
            0,
            request("busy"),
            busy(Arc::clone(&flag), Arc::clone(&polls), Arc::clone(&gave_up)),
        );
        let setter = Arc::clone(&flag);
        requests.insert(
            1,
            request("timer"),
            Box::pin(async move {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                setter.store(true, Ordering::SeqCst);
                Ok(Ok(Value::Nil))
            }),
        );
        let mut order = Vec::new();
        runtime.block_on(async {
            while let Some((id, _, reply)) = requests.next().await {
                reply.unwrap().unwrap();
                order.push(id);
            }
        });
        assert!(
            !gave_up.load(Ordering::SeqCst),
            "the busy work spun {} polls inside one wait and the timer never fired \
             (answers in order {order:?})",
            polls.load(Ordering::SeqCst)
        );
        assert_eq!(order, [1, 0]);
    }
}

/// Every guest trap has the same outcome, also one during buffer allocation. The first
/// request is ready; the other two stay pending until the host drops them.
fn allocation_fault_program(allocation: &str) -> Program {
    let bytes = wat::parse_str(format!(
        r#"(module
        (import "ilang:host/v1" "panic" (func $panic (param i32 i32)))
        (import "test:work/v1" "work" (func $work (param i32)))
        (memory (export "memory") 1)
        (data (i32.const 0) "allocator stop")
        (func (export "ilang_start"))
        (func (export "ilang_alloc") (param i32) (result i32)
          {allocation} (i32.const 32))
        (func (export "ilang_free") (param i32 i32))
        (func (export "app/reply") (result i32)
          (call $work (i32.const 0))
          (call $work (i32.const 1))
          (call $work (i32.const 2)) (i32.const 1))
        (func (export "ilang_resume:test:work/v1:work") (param i32 i32 i32) (result i32)
          (i32.const 0))
        (func (export "ilang_result:app/reply"))
        (func (export "app/params") (param i32 i32) (result i32) (i32.const 0))
        (func (export "ilang_result:app/params")))"#
    ))
    .unwrap();
    let mut abi = Abi::default();
    abi.exports.insert(
        "app/reply".into(),
        Signature::new([], AbiType::Nil).asynchronous(),
    );
    abi.exports.insert(
        "app/params".into(),
        Signature::new([AbiType::String], AbiType::Nil).asynchronous(),
    );
    abi.services.insert(
        "test:work/v1".into(),
        BTreeMap::from([(
            "work".into(),
            Signature::new([], AbiType::String).asynchronous(),
        )]),
    );
    Program::from_bytes(&bytes).unwrap().with_abi(abi).unwrap()
}

#[test]
fn allocation_traps_preserve_stops_and_the_first_failure() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Dropped(Arc<AtomicUsize>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    for allocation in [
        "unreachable",
        "(call $panic (i32.const 0) (i32.const 14)) unreachable",
        "(call $work (i32.const 99))",
    ] {
        let program = allocation_fault_program(allocation);
        for name in ["app/reply", "app/params"] {
            let starts = Arc::new(AtomicUsize::new(0));
            let drops = Arc::new(AtomicUsize::new(0));
            let (started, dropped) = (Arc::clone(&starts), Arc::clone(&drops));
            let mut imports = HostImports::default();
            imports
                .async_service("test:work/v1", "work", move |_| {
                    let first = started.fetch_add(1, Ordering::SeqCst) == 0;
                    let guard = Dropped(Arc::clone(&dropped));
                    async move {
                        let _guard = guard;
                        if !first {
                            std::future::pending::<()>().await;
                        }
                        Ok(Ok(Value::String("answer".into())))
                    }
                })
                .unwrap();
            let mut instance = program.instantiate(HostOptions::new(imports)).unwrap();
            let args = if name == "app/params" {
                vec![Value::String("input".into())]
            } else {
                vec![]
            };
            let outcome = block_on(instance.call_async(name, &args));
            if allocation.contains("$work") {
                assert_eq!(
                    error_text(outcome),
                    "the module started request 99 outside an async call",
                    "{name}"
                );
            } else {
                let stopped = stop(outcome.unwrap());
                assert_eq!(
                    stopped.message.as_deref(),
                    allocation.contains("$panic").then_some("allocator stop"),
                    "{name}"
                );
            }
            let expected_starts = if name == "app/reply" { 3 } else { 0 };
            assert_eq!(starts.load(Ordering::SeqCst), expected_starts, "{name}");
            assert_eq!(drops.load(Ordering::SeqCst), expected_starts, "{name}");
            assert!(instance.unusable().is_some());
            assert!(instance.store.data().requests.is_empty());
        }
    }
}

/// The countdown work wakes itself for a seeded number of
/// polls. Unlike a gate, it makes no promise about completion order.
#[test]
fn countdown_requests_match_the_observed_answer_order() {
    let program = program();
    for seed in 0..200_u64 {
        let order = Order::default();
        let recorded = Arc::clone(&order);
        let countdown = move |arguments: &[Value]| -> ServiceFuture {
            let [Value::Float(x)] = arguments else {
                panic!("later takes one Float")
            };
            let x = *x as i64;
            let mut polls = (seed ^ (x as u64).wrapping_mul(37)) % 11;
            let recorded = Arc::clone(&recorded);
            Box::pin(std::future::poll_fn(move |context| {
                if polls > 0 {
                    polls -= 1;
                    context.waker().wake_by_ref();
                    return std::task::Poll::Pending;
                }
                recorded.lock().unwrap().push(x);
                std::task::Poll::Ready(Ok(Ok(Value::Float(2.0 * x as f64))))
            }))
        };
        let mut instance = program
            .instantiate(HostOptions::new(imports_with(
                |_| {},
                |_| Ok(Ok(Value::Nil)),
                countdown,
            )))
            .unwrap();
        let n = 1 + seed % 16;
        let h = float(
            block_on(instance.call_async("app/async.many", &[Value::Float(n as f64)])).unwrap(),
        );
        let order = order.lock().unwrap();
        assert_eq!(order.len(), 2 * n as usize, "seed {seed}");
        assert_eq!(h, fold(&order), "seed {seed}");
        assert!(instance.unusable().is_none());
    }
}

/// The rule for a nested store also applies while the host calls ilang_alloc or
/// ilang_free outside the entry export itself (parameter/result buffers).
#[test]
fn a_nested_store_in_the_allocator_gets_its_own_thread() {
    let threads = Threads::default();
    let recorded = Arc::clone(&threads);
    let inner = program()
        .instantiate(HostOptions::new(imports_with(
            move |_| {
                recorded
                    .lock()
                    .unwrap()
                    .push(("print", std::thread::current().id()));
            },
            |_| Ok(Ok(Value::Nil)),
            |_| Box::pin(async { Ok(Ok(Value::Nil)) }),
        )))
        .unwrap();
    threads.lock().unwrap().clear();
    let inner = Mutex::new(inner);
    let bytes = wat::parse_str(
        r#"(module
        (import "test:allocation/v1" "touch" (func $touch))
        (memory (export "memory") 1)
        (func (export "ilang_start"))
        (func (export "ilang_alloc") (param i32) (result i32)
          (call $touch) (i32.const 32))
        (func (export "ilang_free") (param i32 i32) (call $touch))
        (func (export "app/buffer") (param i32 i32) (result i32) (i32.const 0))
        (func (export "ilang_result:app/buffer")))"#,
    )
    .unwrap();
    let mut abi = Abi::default();
    abi.exports.insert(
        "app/buffer".into(),
        Signature::new([AbiType::String], AbiType::Nil).asynchronous(),
    );
    abi.services.insert(
        "test:allocation/v1".into(),
        BTreeMap::from([("touch".into(), Signature::new([], AbiType::Nil))]),
    );
    let mut imports = HostImports::default();
    imports
        .service("test:allocation/v1", "touch", move |_| {
            ensure!(matches!(
                inner.lock().unwrap().run("app/main.main")?,
                Outcome::Ok(_)
            ));
            Ok(Ok(Value::Nil))
        })
        .unwrap();
    let mut instance = Program::from_bytes(&bytes)
        .unwrap()
        .with_abi(abi)
        .unwrap()
        .instantiate(HostOptions::new(imports))
        .unwrap();
    let outer = block_on(async {
        assert!(matches!(
            instance
                .call_async("app/buffer", &[Value::String("input".into())])
                .await
                .unwrap(),
            Outcome::Ok(Value::Nil)
        ));
        std::thread::current().id()
    });
    let recorded = threads.lock().unwrap();
    assert_eq!(
        recorded.len(),
        2,
        "both allocation and free enter the inner store"
    );
    assert!(recorded.iter().all(|(_, id)| *id != outer), "{recorded:?}");
}

// A resolved declaration is authorized before coalescing, including external
// aliases, and each invocation owns its resources.
fn bg_plan(program: &Program) -> bindings::BindingPlan {
    let mut declarations = Vec::new();
    for (kind, table) in [
        ("service", &program.abi.services),
        ("external", &program.abi.externals),
    ] {
        for (namespace, fields) in table {
            for (name, signature) in fields {
                declarations.push(bindings::BindingDeclaration {
                    package: "fixture-content".into(),
                    module: "fixture/host".into(),
                    module_digest: bindings::artifact_digest(FIXTURE),
                    declaration: format!("{kind}.{name}"),
                    namespace: namespace.clone(),
                    name: name.clone(),
                    kind: kind.into(),
                    signature: signature.clone(),
                });
            }
        }
    }
    bindings::BindingPlan {
        artifact_digest: program.artifact_digest.clone(),
        declarations,
    }
}

#[test]
fn bg_grants_check_every_declaration_before_coalescing() {
    let program = program();
    let mut plan = bg_plan(&program);
    let original = plan
        .declarations
        .iter()
        .find(|d| d.kind == "external")
        .unwrap()
        .clone();
    let mut alias = original.clone();
    alias.declaration = "another_external_alias".into();
    plan.declarations.push(alias.clone());
    let program = program.with_binding_plan(plan.clone()).unwrap();
    let mut imports = imports(&Log::default());
    // Replace the deliberate legacy authority with a restricted registration.
    let key = (original.namespace.clone(), original.name.clone());
    imports.functions.remove(&key);
    imports.unrestricted.remove(&key);
    imports
        .context_external(&key.0, &key.1, |_, _| Ok(Value::Float(5.0)))
        .unwrap();
    imports.grant(&plan, &original);
    let expected = format!(
        "ungranted host import {} {} declared by {} {}",
        alias.namespace, alias.name, alias.module, alias.declaration
    );
    assert_eq!(
        program
            .instantiate(HostOptions::new(imports.clone()))
            .err()
            .unwrap()
            .to_string(),
        expected
    );
    imports.grant(&plan, &alias);
    program
        .instantiate(HostOptions::new(imports.clone()))
        .unwrap();
    // Even a duplicate changes the complete authenticated plan. Reapproval
    // preserves the obligations of every original declaration.
    let mut twice = plan.clone();
    twice.declarations.push(original.clone());
    assert!(
        self::program()
            .with_binding_plan(twice.clone())
            .unwrap()
            .instantiate(HostOptions::new(imports.clone()))
            .is_err()
    );
    imports.grant(&twice, &original);
    imports.grant(&twice, &alias);
    self::program()
        .with_binding_plan(twice)
        .unwrap()
        .instantiate(HostOptions::new(imports))
        .unwrap();
}

#[test]
fn bg_grants_are_bound_to_artifact_package_module_declaration_and_signature() {
    let p = program();
    let plan = bg_plan(&p);
    let mut changed = plan.clone();
    changed.artifact_digest.push('0');
    assert!(p.with_binding_plan(changed).is_err());
    for field in 0..4 {
        let p = program();
        let mut changed = plan.clone();
        let d = &mut changed.declarations[0];
        match field {
            0 => d.package.push('x'),
            1 => d.module.push('x'),
            2 => d.module_digest.push('x'),
            _ => d.declaration.push('x'),
        }
        let p = p.with_binding_plan(changed).unwrap();
        let mut imports = imports(&Log::default());
        imports.unrestricted.clear();
        imports.grant_package(&plan, "fixture-content");
        assert!(
            p.instantiate(HostOptions::new(imports))
                .err()
                .unwrap()
                .to_string()
                .starts_with("ungranted host import")
        );
    }
    let mut missing = plan.clone();
    missing.declarations.pop();
    assert!(program().with_binding_plan(missing).is_err());
    let mut wrong = plan;
    wrong.declarations[0].signature.params.push(AbiType::Int);
    assert!(program().with_binding_plan(wrong).is_err());
}

#[test]
fn bg_failed_duplicate_registration_cannot_broaden_a_grant() {
    let mut imports = HostImports::default();
    imports
        .context_external("app:math/v1", "hypot", |_, _| Ok(Value::Float(0.0)))
        .unwrap();
    assert!(
        imports
            .external("app:math/v1", "hypot", |_| Ok(Value::Float(0.0)))
            .is_err()
    );
    assert!(
        !imports
            .unrestricted
            .contains(&("app:math/v1".into(), "hypot".into()))
    );
}

fn bg_context_imports(
    contexts: Arc<Mutex<Vec<(ResourceContext, BigInt)>>>,
    disposed: Arc<std::sync::atomic::AtomicUsize>,
) -> HostImports {
    let mut imports = imports(&Log::default());
    let key = ("app:console/v1".into(), "print".into());
    imports.functions.remove(&key);
    imports.unrestricted.remove(&key);
    imports
        .context_service("app:console/v1", "print", move |context, _| {
            let disposed = disposed.clone();
            let id = context.insert("fixture", (), move |_, _| {
                disposed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            })?;
            contexts.lock().unwrap().push((context, id));
            Ok(Ok(Value::Nil))
        })
        .unwrap();
    imports
}

#[test]
fn bg_sequential_invocations_and_instances_have_fresh_tables() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let p = program();
    let plan = bg_plan(&p);
    let p = p.with_binding_plan(plan.clone()).unwrap();
    let contexts = Arc::new(Mutex::new(Vec::new()));
    let disposed = Arc::new(AtomicUsize::new(0));
    let mut imports = bg_context_imports(contexts.clone(), disposed.clone());
    imports.grant_package(&plan, "fixture-content");
    let options = HostOptions::new(imports);
    for _ in 0..2 {
        let mut instance = p.instantiate(options.clone()).unwrap();
        for _ in 0..2 {
            instance
                .call("print", &[Value::String("test".into())])
                .unwrap();
        }
        instance.dispose();
    }
    let contexts = contexts.lock().unwrap();
    assert_eq!(contexts.len(), 6); // Two initializations and four invocations.
    assert_eq!(disposed.load(Ordering::SeqCst), 6);
    for pair in contexts.windows(2) {
        assert!(pair[0].1 < pair[1].1);
    }
    for (context, id) in contexts.iter() {
        assert!(!context.is_live());
        assert!(matches!(
            context.acquire::<()>(id, "fixture", "read"),
            Err(resources::ResourceError::Closed)
        ));
    }
}

#[test]
fn bg_stop_failure_and_abandoned_future_teardown_captured_contexts() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    for mode in 0..3 {
        let p = program();
        let plan = bg_plan(&p);
        let p = p.with_binding_plan(plan.clone()).unwrap();
        let contexts = Arc::new(Mutex::new(Vec::new()));
        let disposed = Arc::new(AtomicUsize::new(0));
        let mut imports = imports(&Log::default());
        let key = ("app:clock/v1".into(), "later".into());
        imports.functions.remove(&key);
        imports.unrestricted.remove(&key);
        let saved = contexts.clone();
        let count = disposed.clone();
        imports
            .context_async_service("app:clock/v1", "later", move |context, _| {
                let count = count.clone();
                let id = context
                    .insert("fixture", (), move |_, _| {
                        count.fetch_add(1, Ordering::SeqCst);
                    })
                    .unwrap();
                saved.lock().unwrap().push((context, id));
                async move {
                    if mode == 0 {
                        return Ok(Ok(Value::Float(1.0)));
                    }
                    if mode == 1 {
                        bail!("fixture host failure");
                    }
                    std::future::pending::<()>().await;
                    Ok(Ok(Value::Float(1.0)))
                }
            })
            .unwrap();
        imports.grant_package(&plan, "fixture-content");
        let mut instance = p.instantiate(HostOptions::new(imports)).unwrap();
        if mode == 0 {
            assert!(matches!(
                block_on(instance.call_async("app/async.stop_mid", &[])).unwrap(),
                Outcome::Stop(_)
            ));
        } else if mode == 1 {
            assert!(
                block_on(instance.call_async("app/async.later", &[Value::Float(1.0)])).is_err()
            );
        } else {
            let mut future = Box::pin(instance.call_async("app/async.later", &[Value::Float(1.0)]));
            let waker = std::task::Waker::noop();
            assert!(
                future
                    .as_mut()
                    .poll(&mut std::task::Context::from_waker(waker))
                    .is_pending()
            );
            drop(future);
        }
        assert_eq!(
            disposed.load(Ordering::SeqCst),
            contexts.lock().unwrap().len()
        );
        assert!(!contexts.lock().unwrap().is_empty());
        for (context, _) in contexts.lock().unwrap().iter() {
            assert!(!context.is_live());
        }
    }
}

#[test]
fn bg_two_concurrent_runs_in_one_host_are_independent() {
    let p = program();
    let plan = bg_plan(&p);
    let p = p.with_binding_plan(plan.clone()).unwrap();
    let ids = Arc::new(Mutex::new(Vec::new()));
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let mut imports = imports(&Log::default());
    let key = ("app:console/v1".into(), "print".into());
    imports.functions.remove(&key);
    imports.unrestricted.remove(&key);
    imports
        .context_service("app:console/v1", "print", move |context, args| {
            if !matches!(args, [Value::String(s)] if s == "run") {
                return Ok(Ok(Value::Nil));
            }
            let id = context.insert("fixture", (), |_, _| {})?;
            ids.lock().unwrap().push(id.clone());
            barrier.wait();
            for number in ids.lock().unwrap().iter() {
                let result = context.acquire::<()>(number, "fixture", "read");
                if number == &id {
                    result.unwrap().commit(()).unwrap();
                } else {
                    assert!(matches!(result, Err(resources::ResourceError::Closed)));
                }
            }
            barrier.wait();
            Ok(Ok(Value::Nil))
        })
        .unwrap();
    imports.grant_package(&plan, "fixture-content");
    let options = HostOptions::new(imports);
    std::thread::scope(|scope| {
        for _ in 0..2 {
            let options = options.clone();
            let p = &p;
            scope.spawn(move || {
                let mut instance = p.instantiate(options).unwrap();
                instance
                    .call("print", &[Value::String("run".into())])
                    .unwrap();
            });
        }
    });
}

#[test]
fn bg_close_while_a_request_is_outstanding_rejects_its_late_success() {
    let p = program();
    let plan = bg_plan(&p);
    let p = p.with_binding_plan(plan.clone()).unwrap();
    let captured = Arc::new(Mutex::new(None));
    let save = captured.clone();
    let mut imports = imports(&Log::default());
    let key = ("app:clock/v1".into(), "later".into());
    imports.functions.remove(&key);
    imports.unrestricted.remove(&key);
    imports
        .context_async_service("app:clock/v1", "later", move |context, _| {
            let id = context.insert("test", (), |_, _| {}).unwrap();
            let lease = context.acquire::<()>(&id, "test", "read").unwrap();
            *save.lock().unwrap() = Some((context, id));
            async move {
                lease.closed().await;
                assert!(matches!(
                    lease.commit(Value::Float(1.0)),
                    Err(resources::ResourceError::Closed)
                ));
                Ok(Err(Value::String("closed".into())))
            }
        })
        .unwrap();
    imports.grant_package(&plan, "fixture-content");
    let mut instance = p.instantiate(HostOptions::new(imports)).unwrap();
    let args = [Value::Float(1.0)];
    let mut call = Box::pin(instance.call_async("app/async.later", &args));
    assert!(
        call.as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            .is_pending()
    );
    let (context, id) = captured.lock().unwrap().take().unwrap();
    block_on(context.close(&id, "test"));
    assert!(
        matches!(block_on(call).unwrap(), Outcome::Err(GuestError(Value::String(s))) if s == "closed")
    );
}

// Distinct nominal constructors are different values, even
// when the host passes one object (or null) through both declarations.
#[test]
fn compiled_opaque_boundaries_preserve_nominal_identity() {
    let reference = HostReference::new(42);
    let raw = Value::Externref(reference.clone());
    let other = Value::Externref(HostReference::new(42));
    for seed in [raw.clone(), Value::NullExternref] {
        let mut imports = imports(&Arc::default());
        imports
            .async_service("app:clock/v1", "tick", |_| async {
                Ok(Ok(Value::Int(42.into())))
            })
            .unwrap();
        imports.merge(opaque_imports(seed)).unwrap();
        let mut instance = Program::from_bytes(COMPILED)
            .unwrap()
            .instantiate(HostOptions::new(imports))
            .unwrap();
        let boolean = |instance: &mut Instance, name: &str, args: &[Value]| {
            let result = ok(instance.call(&format!("app/opaque.{name}"), args).unwrap());
            let Value::Bool(value) = result else {
                panic!("expected Bool: {result:?}")
            };
            value
        };
        for (a, b, same) in [
            (raw.clone(), raw.clone(), true),
            (raw.clone(), other.clone(), false),
            (Value::NullExternref, Value::NullExternref, true),
            (Value::NullExternref, raw.clone(), false),
        ] {
            let args = [a, b];
            for name in [
                "direct_aa",
                "generic_aa",
                "alias_aa",
                "collection_aa",
                "roundtrip_aa",
            ] {
                assert_eq!(
                    boolean(&mut instance, name, &args),
                    same,
                    "{name}: {args:?}"
                );
            }
            for name in ["direct_ab", "generic_ab", "module_ab", "collection_ab"] {
                assert!(!boolean(&mut instance, name, &args), "{name}: {args:?}");
            }
        }
        for a in [raw.clone(), Value::NullExternref] {
            let returned = ok(instance
                .call("app/opaque.echo", std::slice::from_ref(&a))
                .unwrap());
            match &returned {
                Value::Externref(value) => assert!(reference.ptr_eq(value)),
                Value::NullExternref => assert!(matches!(a, Value::NullExternref)),
                _ => panic!("expected opaque reference"),
            }
            assert!(boolean(
                &mut instance,
                "generic_aa",
                &[a.clone(), returned.clone()]
            ));
            assert!(!boolean(
                &mut instance,
                "generic_ab",
                &[a.clone(), returned]
            ));
            assert!(!boolean(
                &mut instance,
                "roundtrip_ab",
                std::slice::from_ref(&a)
            ));
            assert!(boolean(&mut instance, "hash_aa", &[a.clone(), a.clone()]));
            assert!(
                matches!(ok(instance.call("app/opaque.inspect_a", &[a]).unwrap()),
                Value::String(text) if text == "//wasm(opaque)")
            );
            instance.collect_garbage().unwrap();
        }
        for (name, expected) in [
            ("import_aa", true),
            ("import_ab", false),
            ("sync_aa", true),
            ("sync_ab", false),
        ] {
            assert_eq!(boolean(&mut instance, name, &[]), expected, "{name}");
        }
        for (name, expected) in [("async_aa", true), ("async_ab", false)] {
            let result =
                ok(block_on(instance.call_async(&format!("app/opaque.{name}"), &[])).unwrap());
            assert!(
                matches!(result, Value::Bool(value) if value == expected),
                "{name}: {result:?}"
            );
        }
    }
}

/// The additional host functions of the compiled opaque boundary fixture.
fn opaque_imports(seed: Value) -> HostImports {
    let mut imports = HostImports::default();
    let get = seed.clone();
    imports
        .external("app:opaque/v1", "get", move |_| Ok(get.clone()))
        .unwrap();
    imports
        .external("app:opaque/v1", "echo", |args| Ok(args[0].clone()))
        .unwrap();
    for label in ["get_a", "get_b"] {
        let get = seed.clone();
        imports
            .service("app:opaque-sync/v1", label, move |_| Ok(Ok(get.clone())))
            .unwrap();
        let get = seed.clone();
        imports
            .async_service("app:opaque-async/v1", label, move |_| {
                let get = get.clone();
                async move { Ok(Ok(get)) }
            })
            .unwrap();
    }
    imports
}

#[test]
fn bg_review_complete_plan_cannot_drop_a_denied_coalesced_declaration() {
    let plan: bindings::BindingPlan = serde_json::from_str(include_str!(
        "../../../tests/browser/fixtures/compiled.bindings.json"
    ))
    .unwrap();
    let mut imports = imports(&Log::default());
    imports
        .async_service("app:clock/v1", "tick", |_| async {
            Ok(Ok(Value::Int(1.into())))
        })
        .unwrap();
    imports.merge(opaque_imports(Value::NullExternref)).unwrap();
    let key = ("app:opaque/v1".into(), "get".into());
    imports.functions.remove(&key);
    imports.unrestricted.remove(&key);
    imports
        .context_external(&key.0, &key.1, |_, _| Ok(Value::NullExternref))
        .unwrap();
    assert_eq!(
        plan.declarations
            .iter()
            .filter(|d| d.namespace == key.0 && d.name == key.1)
            .count(),
        2
    );
    let allowed = plan
        .declarations
        .iter()
        .find(|d| d.declaration == "get_a")
        .unwrap();
    imports.grant(&plan, allowed);
    let options = HostOptions::new(imports);
    assert!(
        Program::from_bytes(COMPILED)
            .unwrap()
            .with_binding_plan(plan.clone())
            .unwrap()
            .instantiate(options.clone())
            .err()
            .unwrap()
            .to_string()
            .contains("get_b")
    );
    let mut omitted = plan.clone();
    omitted.declarations.retain(|d| d.declaration != "get_b");
    assert!(
        Program::from_bytes(COMPILED)
            .unwrap()
            .with_binding_plan(omitted)
            .unwrap()
            .instantiate(options.clone())
            .is_err(),
        "a replacement plan must not reuse approval of the complete original plan"
    );
    let mut reordered = plan;
    reordered.declarations.reverse();
    assert!(
        Program::from_bytes(COMPILED)
            .unwrap()
            .with_binding_plan(reordered)
            .unwrap()
            .instantiate(options)
            .is_err()
    );
}

#[test]
fn bg_review_ended_context_blocks_runnable_continuations() {
    use std::task::{Context, Waker};
    let p = program();
    let plan = bg_plan(&p);
    let mut imports = imports(&Log::default());
    let key = ("app:clock/v1".into(), "later".into());
    imports.functions.remove(&key);
    imports.unrestricted.remove(&key);
    let saved = Arc::new(Mutex::new(None));
    let capture = saved.clone();
    imports
        .context_async_service(&key.0, &key.1, move |context, _| {
            *capture.lock().unwrap() = Some(context);
            async { Ok(Ok(Value::Float(2.0))) }
        })
        .unwrap();
    imports.grant_package(&plan, "fixture-content");
    let mut guest = p
        .with_binding_plan(plan)
        .unwrap()
        .instantiate(HostOptions::new(imports))
        .unwrap();
    let args = [Value::Float(100.0)];
    let mut call = Box::pin(guest.call_async("app/async.yielding", &args));
    let mut cx = Context::from_waker(Waker::noop());
    assert!(call.as_mut().poll(&mut cx).is_pending());
    assert!(call.as_mut().poll(&mut cx).is_pending());
    saved.lock().unwrap().as_ref().unwrap().teardown();
    assert!(
        matches!(call.as_mut().poll(&mut cx), std::task::Poll::Ready(Err(error)) if error.to_string().contains("ended"))
    );
}

#[test]
fn bg_review_opaque_shape_is_not_an_async_protocol_on_rust() {
    struct Opaque {
        _then: fn(),
        _finally: fn(),
    }
    let value = HostReference::new(Opaque {
        _then: || panic!("opaque then must not run"),
        _finally: || panic!("opaque finally must not run"),
    });
    let (mut guest, _) = fixture();
    let Value::Externref(returned) = ok(guest
        .call("opaque_roundtrip", &[Value::Externref(value.clone())])
        .unwrap())
    else {
        panic!("expected opaque reference")
    };
    assert!(value.ptr_eq(&returned));
}

fn bg_lifecycle_program() -> Program {
    let bytes = include_bytes!("../../../tests/browser/fixtures/bg-lifecycle.wasm");
    let point = Signature::new([], AbiType::Nil).asynchronous();
    Program::from_bytes(bytes)
        .unwrap()
        .with_abi(Abi {
            entry: Some(EntryPoint {
                name: "entry".into(),
                is_async: true,
            }),
            exports: [
                ("runnable".into(), point.clone()),
                ("immediate".into(), point.clone()),
                ("leftover".into(), point),
                ("sync".into(), Signature::new([], AbiType::Nil)),
                (
                    "buffers".into(),
                    Signature::new([AbiType::String, AbiType::String], AbiType::Nil),
                ),
            ]
            .into(),
            services: [
                (
                    "bg:control/v1".into(),
                    [
                        ("capture".into(), Signature::new([], AbiType::Nil)),
                        ("end".into(), Signature::new([], AbiType::Nil)),
                    ]
                    .into(),
                ),
                (
                    "bg:work/v1".into(),
                    [(
                        "wait".into(),
                        Signature::new([], AbiType::Nil).asynchronous(),
                    )]
                    .into(),
                ),
            ]
            .into(),
            ..Abi::default()
        })
        .unwrap()
}
fn bg_lifecycle_counter(guest: &mut Instance, offset: usize) -> u32 {
    let memory = guest
        .instance
        .get_memory(&mut guest.store, "memory")
        .unwrap();
    u32::from_le_bytes(
        memory.data(&guest.store)[offset..offset + 4]
            .try_into()
            .unwrap(),
    )
}
fn bg_lifecycle_imports(
    p: &Program,
    capture: impl Fn(ResourceContext) + Send + Sync + 'static,
) -> HostImports {
    let plan = bg_plan(p);
    let mut imports = HostImports::default();
    imports
        .context_service("bg:control/v1", "capture", move |context, _| {
            capture(context);
            Ok(Ok(Value::Nil))
        })
        .unwrap();
    imports
        .context_service("bg:control/v1", "end", |context, _| {
            context.teardown();
            Ok(Ok(Value::Nil))
        })
        .unwrap();
    imports
        .context_async_service("bg:work/v1", "wait", |_, _| std::future::pending())
        .unwrap();
    imports.grant_package(&plan, "fixture-content");
    imports
}
#[test]
fn bg_review_no_request_runnable_and_result_paths_do_not_enter_ended_wasm() {
    for entry in [false, true] {
        let p = bg_lifecycle_program();
        let plan = bg_plan(&p);
        let saved = Arc::new(Mutex::new(None));
        let capture = saved.clone();
        let imports =
            bg_lifecycle_imports(&p, move |context| *capture.lock().unwrap() = Some(context));
        let mut guest = p
            .with_binding_plan(plan)
            .unwrap()
            .instantiate(HostOptions::new(imports))
            .unwrap();
        let mut call: Pin<Box<dyn Future<Output = Result<Outcome>> + Send + '_>> = if entry {
            Box::pin(guest.run_async("entry"))
        } else {
            Box::pin(guest.call_async("runnable", &[]))
        };
        assert!(
            call.as_mut()
                .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
                .is_pending()
        );
        saved.lock().unwrap().as_ref().unwrap().teardown();
        assert!(matches!(block_on(call), Err(error) if error.to_string().contains("ended")));
        assert_eq!(bg_lifecycle_counter(&mut guest, 0), 0);
        assert_eq!(bg_lifecycle_counter(&mut guest, 4), 0);
    }
}
#[test]
fn bg_review_drop_at_status_zero_blocks_later_result_export() {
    struct EndOnDrop(ResourceContext);
    impl Drop for EndOnDrop {
        fn drop(&mut self) {
            self.0.teardown();
        }
    }
    let p = bg_lifecycle_program();
    let plan = bg_plan(&p);
    let mut imports = bg_lifecycle_imports(&p, |_| {});
    imports
        .functions
        .remove(&("bg:work/v1".into(), "wait".into()));
    imports
        .context_async_service("bg:work/v1", "wait", |context, _| {
            let guard = EndOnDrop(context);
            async move {
                let _guard = guard;
                std::future::pending().await
            }
        })
        .unwrap();
    let mut guest = p
        .with_binding_plan(plan)
        .unwrap()
        .instantiate(HostOptions::new(imports))
        .unwrap();
    assert!(
        matches!(block_on(guest.call_async("leftover", &[])), Err(error) if error.to_string().contains("ended"))
    );
    assert_eq!(bg_lifecycle_counter(&mut guest, 4), 0);
}
#[test]
fn bg_review_gate_covers_sync_callbacks_allocations_and_each_free() {
    for mode in [0, 16, 20] {
        let p = bg_lifecycle_program();
        let plan = bg_plan(&p);
        let imports = bg_lifecycle_imports(&p, move |context| {
            if mode == 0 {
                context.teardown();
            }
        });
        let mut guest = p
            .with_binding_plan(plan)
            .unwrap()
            .instantiate(HostOptions::new(imports))
            .unwrap();
        let memory = guest
            .instance
            .get_memory(&mut guest.store, "memory")
            .unwrap();
        if mode != 0 {
            memory.data_mut(&mut guest.store)[mode] = 1;
        }
        let result = if mode == 0 {
            guest.call("sync", &[])
        } else {
            guest.call(
                "buffers",
                &[
                    Value::String("first".into()),
                    Value::String("second".into()),
                ],
            )
        };
        assert!(!matches!(result, Ok(Outcome::Ok(_))));
        if mode == 20 {
            assert_eq!(bg_lifecycle_counter(&mut guest, 24), 1);
        } else {
            assert_eq!(bg_lifecycle_counter(&mut guest, 12), 0);
        }
    }
}

#[test]
fn bg_review_cleanup_failure_is_reported_without_replacing_a_success() {
    struct PanicOnDrop;
    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("request destructor failed");
        }
    }
    let p = bg_lifecycle_program();
    let plan = bg_plan(&p);
    let mut imports = bg_lifecycle_imports(&p, |_| {});
    imports
        .functions
        .remove(&("bg:work/v1".into(), "wait".into()));
    imports
        .context_async_service("bg:work/v1", "wait", |_, _| {
            let guard = PanicOnDrop;
            async move {
                let _guard = guard;
                std::future::pending().await
            }
        })
        .unwrap();
    let mut guest = p
        .with_binding_plan(plan)
        .unwrap()
        .instantiate(HostOptions::new(imports))
        .unwrap();
    assert!(matches!(
        block_on(guest.call_async("leftover", &[])).unwrap(),
        Outcome::Ok(Value::Nil)
    ));
    assert_eq!(bg_lifecycle_counter(&mut guest, 4), 1);
    let errors = guest.take_cleanup_errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].phase, "cancel");
    assert_eq!(errors[0].message, "request destructor failed");
    assert!(guest.unusable().is_none());
}
