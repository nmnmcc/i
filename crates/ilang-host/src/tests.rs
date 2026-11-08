use super::*;

const FIXTURE: &[u8] = include_bytes!("../../../tests/browser/fixtures/host-abi.wasm");
const CUSTOM: &[u8] = include_bytes!("../../../tests/browser/fixtures/custom-imports.wasm");

fn signature(kind: AbiType) -> Signature {
    Signature {
        params: if kind == AbiType::Nil {
            vec![]
        } else {
            vec![kind]
        },
        result: kind,
    }
}

fn fixture() -> Instance {
    let mut abi = Abi::default();
    for (name, kind) in [
        ("string_roundtrip", AbiType::String),
        ("int_roundtrip", AbiType::Int),
        ("bit_array_roundtrip", AbiType::BitArray),
        ("float_roundtrip", AbiType::Float),
        ("bool_roundtrip", AbiType::Bool),
        ("nil_roundtrip", AbiType::Nil),
        ("opaque_roundtrip", AbiType::Externref),
    ] {
        abi.exports.insert(name.into(), signature(kind));
    }
    for name in ["trap_with_string", "fail"] {
        abi.exports.insert(
            name.into(),
            Signature {
                params: vec![AbiType::String],
                result: AbiType::Nil,
            },
        );
    }
    Program::from_bytes(FIXTURE)
        .unwrap()
        .with_abi(abi)
        .unwrap()
        .instantiate(HostOptions::default())
        .unwrap()
}

fn allocations(instance: &mut Instance) -> i32 {
    instance
        .instance
        .get_typed_func::<(), i32>(&mut instance.store, "live_allocations")
        .unwrap()
        .call(&mut instance.store, ())
        .unwrap()
}

#[test]
fn strings_including_empty_unicode_and_embedded_nul_are_copied_and_freed() {
    let mut instance = fixture();
    for expected in ["", "Gleam 🦀 e\u{301}", "one\0two"] {
        let Value::String(actual) = instance
            .call("string_roundtrip", &[Value::String(expected.into())])
            .unwrap()
        else {
            panic!("expected string")
        };
        assert_eq!(actual, expected);
        assert_eq!(allocations(&mut instance), 0);
    }
}

#[test]
fn arbitrary_precision_integers_roundtrip_without_host_arithmetic() {
    let mut instance = fixture();
    let huge = BigInt::from(1_u32) << 4096_u32;
    for expected in [BigInt::from(0), BigInt::from(-1), huge.clone(), -huge] {
        let Value::Int(actual) = instance
            .call("int_roundtrip", &[Value::Int(expected.clone())])
            .unwrap()
        else {
            panic!("expected int")
        };
        assert_eq!(actual, expected);
        assert_eq!(allocations(&mut instance), 0);
    }
}

#[test]
fn bit_array_preserves_non_byte_aligned_length_and_rejects_invalid_padding() {
    let mut instance = fixture();
    for expected in [
        BitArray::new(vec![], 0).unwrap(),
        BitArray::new(vec![0b1010_0000], 3).unwrap(),
        BitArray::new(vec![0xff, 0x80], 9).unwrap(),
    ] {
        let Value::BitArray(actual) = instance
            .call("bit_array_roundtrip", &[Value::BitArray(expected.clone())])
            .unwrap()
        else {
            panic!("expected bits")
        };
        assert_eq!(actual, expected);
        assert_eq!(allocations(&mut instance), 0);
    }
    assert!(BitArray::new(vec![0xff], 3).is_err());
    assert!(BitArray::new(vec![0], 9).is_err());
}

#[test]
fn scalars_validate_language_semantics() {
    let mut instance = fixture();
    assert!(matches!(
        instance
            .call("bool_roundtrip", &[Value::Bool(true)])
            .unwrap(),
        Value::Bool(true)
    ));
    assert!(matches!(
        instance.call("nil_roundtrip", &[]).unwrap(),
        Value::Nil
    ));
    let Value::Float(value) = instance
        .call("float_roundtrip", &[Value::Float(-0.0)])
        .unwrap()
    else {
        panic!("expected float")
    };
    assert_eq!(value.to_bits(), (-0.0_f64).to_bits());
    assert!(
        instance
            .call("float_roundtrip", &[Value::Float(f64::INFINITY)])
            .is_err()
    );
    assert!(
        instance
            .call("float_roundtrip", &[Value::Float(f64::NAN)])
            .is_err()
    );
    assert!(instance.call("bool_roundtrip", &[]).is_err());
    assert!(
        instance
            .call("bool_roundtrip", &[Value::String("bad".into())])
            .is_err()
    );
}

#[test]
fn opaque_references_keep_identity_across_collection_and_support_null() {
    let mut instance = fixture();
    let reference = HostReference::new(String::from("owned by the embedder"));
    for _ in 0..10 {
        let Value::Externref(returned) = instance
            .call("opaque_roundtrip", &[Value::Externref(reference.clone())])
            .unwrap()
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
    assert!(matches!(
        instance
            .call("opaque_roundtrip", &[Value::NullExternref])
            .unwrap(),
        Value::NullExternref
    ));
}

#[test]
fn identical_host_arcs_get_identical_ids_even_with_separate_externrefs() {
    let bytes = wat::parse_str(
        r#"(module
        (import "ilang:host/v1" "reference_id" (func $id (param externref) (result i64)))
        (func (export "same") (param externref externref) (result i32)
            (i64.eq (call $id (local.get 0)) (call $id (local.get 1)))))"#,
    )
    .unwrap();
    let mut abi = Abi::default();
    abi.exports.insert(
        "same".into(),
        Signature {
            params: vec![AbiType::Externref, AbiType::Externref],
            result: AbiType::Bool,
        },
    );
    let mut instance = Program::from_bytes(&bytes)
        .unwrap()
        .with_abi(abi)
        .unwrap()
        .instantiate(HostOptions::default())
        .unwrap();
    let reference = HostReference::new(42);
    assert!(matches!(
        instance
            .call(
                "same",
                &[
                    Value::Externref(reference.clone()),
                    Value::Externref(reference)
                ]
            )
            .unwrap(),
        Value::Bool(true)
    ));
    assert!(matches!(
        instance
            .call(
                "same",
                &[
                    Value::Externref(HostReference::new(42)),
                    Value::Externref(HostReference::new(42))
                ]
            )
            .unwrap(),
        Value::Bool(false)
    ));
    assert!(matches!(
        instance
            .call("same", &[Value::NullExternref, Value::NullExternref])
            .unwrap(),
        Value::Bool(true)
    ));
}

#[test]
fn guest_retained_references_outlive_calls_and_are_released_after_collection() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Resource(Arc<AtomicUsize>);
    impl Drop for Resource {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    let bytes = wat::parse_str(
        r#"(module
        (import "ilang:host/v1" "reference_id" (func $id (param externref) (result i64)))
        (global $retained (mut externref) (ref.null extern))
        (func (export "retain") (param externref)
            (drop (call $id (local.get 0)))
            (global.set $retained (local.get 0)))
        (func (export "release") (global.set $retained (ref.null extern))))"#,
    )
    .unwrap();
    let mut abi = Abi::default();
    abi.exports.insert(
        "retain".into(),
        Signature {
            params: vec![AbiType::Externref],
            result: AbiType::Nil,
        },
    );
    let mut instance = Program::from_bytes(&bytes)
        .unwrap()
        .with_abi(abi)
        .unwrap()
        .instantiate(HostOptions::default())
        .unwrap();
    let drops = Arc::new(AtomicUsize::new(0));
    instance
        .call(
            "retain",
            &[Value::Externref(HostReference::new(Resource(
                drops.clone(),
            )))],
        )
        .unwrap();
    instance.collect_garbage().unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    instance.call_raw_empty("release").unwrap();
    instance.collect_garbage().unwrap();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn trapping_exports_release_owned_argument_buffers() {
    let mut instance = fixture();
    let error = instance
        .call(
            "trap_with_string",
            &[Value::String("error with allocated input".into())],
        )
        .unwrap_err();
    assert!(format!("{error:#}").contains("executing trap_with_string"));
    assert_eq!(allocations(&mut instance), 0);
    let error = instance
        .call(
            "fail",
            &[Value::String("fixture.i:12: assertion failed".into())],
        )
        .unwrap_err();
    assert!(format!("{error:#}").contains("fixture.i:12: assertion failed"));
    assert_eq!(allocations(&mut instance), 0);
    instance.call_raw_empty("passing_test").unwrap();
}

#[test]
fn shared_fixture_executes_gc_typed_function_references_and_deep_tail_calls() {
    let mut instance = fixture();
    let probe = instance
        .instance
        .get_typed_func::<(), i32>(&mut instance.store, "feature_probe")
        .unwrap();
    assert_eq!(probe.call(&mut instance.store, ()).unwrap(), 42);
    let tail = instance
        .instance
        .get_typed_func::<(i32, i32), i32>(&mut instance.store, "tail_count")
        .unwrap();
    assert_eq!(
        tail.call(&mut instance.store, (1_000_000, 7)).unwrap(),
        1_000_007
    );
    let gc_sum = instance
        .instance
        .get_typed_func::<i32, i32>(&mut instance.store, "gc_sum")
        .unwrap();
    assert_eq!(
        gc_sum.call(&mut instance.store, 10_000).unwrap(),
        50_005_000
    );
    instance.collect_garbage().unwrap();
}

#[test]
fn standard_output_is_exact_and_initialization_runs() {
    let mut instance = fixture();
    instance.call_raw_empty("passing_test").unwrap();
    instance.call_raw_empty("main").unwrap();
    assert_eq!(instance.output().stdout, "fixture main\n");
    assert_eq!(instance.take_output().stdout, "fixture main\n");
    assert!(instance.output().stdout.is_empty());
}

#[test]
fn custom_imports_roundtrip_all_supported_types_and_free_buffers() {
    let mut abi = Abi::default();
    let mut imports = HostImports::default();
    let types = [
        ("echo_string", AbiType::String),
        ("echo_int", AbiType::Int),
        ("echo_bits", AbiType::BitArray),
        ("echo_ref", AbiType::Externref),
        ("echo_bool", AbiType::Bool),
        ("echo_float", AbiType::Float),
        ("echo_nil", AbiType::Nil),
    ];
    let mut signatures = BTreeMap::new();
    for (name, kind) in types {
        signatures.insert(name.into(), signature(kind));
        abi.exports.insert(name.into(), signature(kind));
        imports
            .define("custom", name, |arguments| {
                Ok(arguments.first().cloned().unwrap_or(Value::Nil))
            })
            .unwrap();
    }
    abi.imports.insert("custom".into(), signatures);
    let program = Program::from_bytes(CUSTOM).unwrap().with_abi(abi).unwrap();
    assert!(program.instantiate(HostOptions::default()).is_err());
    let mut instance = program
        .instantiate(HostOptions {
            imports,
            ..HostOptions::default()
        })
        .unwrap();
    for value in ["", "host callback 🐉"] {
        assert!(
            matches!(instance.call("echo_string", &[Value::String(value.into())]).unwrap(), Value::String(actual) if actual == value)
        );
        assert_eq!(allocations(&mut instance), 0);
    }
    for value in [BigInt::from(0), -(BigInt::from(1) << 500_u32)] {
        assert!(
            matches!(instance.call("echo_int", &[Value::Int(value.clone())]).unwrap(), Value::Int(actual) if actual == value)
        );
        assert_eq!(allocations(&mut instance), 0);
    }
    let bits = BitArray::new(vec![0xa0], 3).unwrap();
    assert!(
        matches!(instance.call("echo_bits", &[Value::BitArray(bits.clone())]).unwrap(), Value::BitArray(actual) if actual == bits)
    );
    assert!(matches!(
        instance.call("echo_nil", &[]).unwrap(),
        Value::Nil
    ));
    let reference = HostReference::new("callback object");
    assert!(
        matches!(instance.call("echo_ref", &[Value::Externref(reference.clone())]).unwrap(), Value::Externref(actual) if actual.ptr_eq(&reference))
    );
    assert_eq!(allocations(&mut instance), 0);
}

#[test]
fn shared_fixture_restores_allocation_frames_after_host_callback_errors() {
    let mut abi = Abi::default();
    let mut imports = HostImports::default();
    let mut signatures = BTreeMap::new();
    for (name, kind) in [
        ("echo_string", AbiType::String),
        ("echo_int", AbiType::Int),
        ("echo_bits", AbiType::BitArray),
        ("echo_ref", AbiType::Externref),
        ("echo_bool", AbiType::Bool),
        ("echo_float", AbiType::Float),
        ("echo_nil", AbiType::Nil),
    ] {
        signatures.insert(name.into(), signature(kind));
        imports
            .define("custom", name, |_| bail!("custom host failure"))
            .unwrap();
    }
    abi.imports.insert("custom".into(), signatures);
    abi.exports
        .insert("leaking_throw".into(), signature(AbiType::String));
    let mut instance = Program::from_bytes(CUSTOM)
        .unwrap()
        .with_abi(abi)
        .unwrap()
        .instantiate(HostOptions {
            imports,
            ..HostOptions::default()
        })
        .unwrap();
    let heap_end = instance
        .instance
        .get_typed_func::<(), i32>(&mut instance.store, "heap_end")
        .unwrap();
    let frames = instance
        .instance
        .get_typed_func::<(), i32>(&mut instance.store, "active_frames")
        .unwrap();
    let initial = heap_end.call(&mut instance.store, ()).unwrap();
    for _ in 0..10 {
        let error = instance
            .call("leaking_throw", &[Value::String("owned input".into())])
            .unwrap_err();
        assert!(format!("{error:#}").contains("custom host failure"));
        assert_eq!(allocations(&mut instance), 0);
        assert_eq!(heap_end.call(&mut instance.store, ()).unwrap(), initial);
        assert_eq!(frames.call(&mut instance.store, ()).unwrap(), 0);
    }
}

#[test]
fn malformed_metadata_reserved_modules_and_out_of_bounds_results_fail() {
    let abi = Abi {
        version: 2,
        ..Abi::default()
    };
    assert!(Program::from_bytes(FIXTURE).unwrap().with_abi(abi).is_err());
    let mut abi = Abi::default();
    abi.imports.insert(HOST_MODULE.into(), BTreeMap::new());
    assert!(Program::from_bytes(FIXTURE).unwrap().with_abi(abi).is_err());
    let bytes = wat::parse_str(
        r#"(module
        (memory (export "memory") 1)
        (func (export "bad") (result i32 i32) i32.const 65536 i32.const 10))"#,
    )
    .unwrap();
    let mut abi = Abi::default();
    abi.exports.insert(
        "bad".into(),
        Signature {
            params: vec![],
            result: AbiType::String,
        },
    );
    let mut instance = Program::from_bytes(&bytes)
        .unwrap()
        .with_abi(abi)
        .unwrap()
        .instantiate(HostOptions::default())
        .unwrap();
    assert!(
        instance
            .call("bad", &[])
            .unwrap_err()
            .to_string()
            .contains("outside guest memory")
    );
}

#[test]
fn embedded_abi_is_validated_and_cannot_be_overridden() {
    fn append_abi(bytes: &mut Vec<u8>, json: &[u8]) {
        fn leb(mut value: usize, out: &mut Vec<u8>) {
            loop {
                let byte = (value & 0x7f) as u8;
                value >>= 7;
                out.push(byte | if value != 0 { 0x80 } else { 0 });
                if value == 0 {
                    break;
                }
            }
        }
        let mut data = Vec::new();
        leb("ilang.abi".len(), &mut data);
        data.extend_from_slice(b"ilang.abi");
        data.extend_from_slice(json);
        bytes.push(0);
        leb(data.len(), bytes);
        bytes.extend(data);
    }
    let mut bytes = wat::parse_str("(module)").unwrap();
    append_abi(&mut bytes, br#"{"version":1,"entry":null}"#);
    let program = Program::from_bytes(&bytes).unwrap();
    assert!(program.abi().entry.is_none());
    assert!(program.with_abi(Abi::default()).is_err());
    append_abi(&mut bytes, br#"{"version":1}"#);
    assert!(
        Abi::from_wasm(&bytes)
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
    let mut bytes = wat::parse_str("(module)").unwrap();
    append_abi(&mut bytes, br#"{"version":999}"#);
    assert!(
        Abi::from_wasm(&bytes)
            .unwrap_err()
            .to_string()
            .contains("unsupported")
    );
}

#[test]
fn bridge_frames_reclaim_guest_internal_allocations_when_host_imports_fail() {
    let bytes = wat::parse_str(
        r#"(module
        (import "custom" "fail" (func $fail))
        (memory (export "memory") 1)
        (global $live (mut i32) (i32.const 0))
        (func (export "__ilang_abi_mark") (result i32) (global.get $live))
        (func (export "__ilang_abi_reset") (param i32) (global.set $live (local.get 0)))
        (func $alloc (export "__ilang_alloc") (param i32) (result i32)
            (global.set $live (i32.add (global.get $live) (i32.const 1)))
            i32.const 8)
        (func (export "__ilang_free") (param i32 i32)
            (global.set $live (i32.sub (global.get $live) (i32.const 1))))
        (func (export "bad") (drop (call $alloc (i32.const 12))) (call $fail))
        (func (export "clean") (if (global.get $live) (then unreachable))))"#,
    )
    .unwrap();
    let mut abi = Abi::default();
    abi.imports.insert(
        "custom".into(),
        BTreeMap::from([("fail".into(), signature(AbiType::Nil))]),
    );
    abi.exports.insert("bad".into(), signature(AbiType::Nil));
    let mut imports = HostImports::default();
    imports
        .define("custom", "fail", |_| bail!("intentional host failure"))
        .unwrap();
    let mut instance = Program::from_bytes(&bytes)
        .unwrap()
        .with_abi(abi)
        .unwrap()
        .instantiate(HostOptions {
            imports,
            ..HostOptions::default()
        })
        .unwrap();
    for _ in 0..10 {
        let error = instance.call("bad", &[]).unwrap_err();
        assert!(format!("{error:#}").contains("intentional host failure"));
        instance.call_raw_empty("clean").unwrap();
        assert!(instance.call_raw_empty("bad").is_err());
        instance.call_raw_empty("clean").unwrap();
    }
}

#[test]
fn incomplete_frame_exports_fail_and_reset_does_not_hide_primary_error() {
    let bytes = wat::parse_str(
        r#"(module
        (func (export "__ilang_abi_mark") (result i32) i32.const 1)
        (func (export "main")))"#,
    )
    .unwrap();
    let mut instance = Program::from_bytes(&bytes)
        .unwrap()
        .instantiate(HostOptions::default())
        .unwrap();
    assert!(
        instance
            .call_raw_empty("main")
            .unwrap_err()
            .to_string()
            .contains("both __ilang_abi_mark")
    );
    let bytes = wat::parse_str(
        r#"(module
        (func (export "__ilang_abi_mark") (result i32) i32.const 1)
        (func (export "__ilang_abi_reset") (param i32) unreachable)
        (func (export "bad") unreachable))"#,
    )
    .unwrap();
    let mut instance = Program::from_bytes(&bytes)
        .unwrap()
        .instantiate(HostOptions::default())
        .unwrap();
    assert!(
        instance
            .call_raw_empty("bad")
            .unwrap_err()
            .to_string()
            .contains("executing bad")
    );
}

#[test]
fn randomness_is_bounded_and_never_overwrites_adjacent_memory() {
    let bytes = wat::parse_str(
        r#"(module
        (import "ilang:host/v1" "random" (func $random (param i32 i32) (result i32)))
        (memory (export "memory") 1)
        (func (export "randomness")
            (i32.store (i32.const 4) (i32.const 1234))
            (i32.store (i32.const 24) (i32.const 5678))
            (if (call $random (i32.const 8) (i32.const 16)) (then unreachable))
            (if (i32.ne (i32.load (i32.const 4)) (i32.const 1234)) (then unreachable))
            (if (i32.ne (i32.load (i32.const 24)) (i32.const 5678)) (then unreachable)))
        (func (export "bad_randomness")
            (drop (call $random (i32.const 65535) (i32.const 2)))))"#,
    )
    .unwrap();
    let mut instance = Program::from_bytes(&bytes)
        .unwrap()
        .instantiate(HostOptions::default())
        .unwrap();
    instance.call_raw_empty("randomness").unwrap();
    let error = instance.call_raw_empty("bad_randomness").unwrap_err();
    assert!(format!("{error:#}").contains("outside guest memory"));
}
