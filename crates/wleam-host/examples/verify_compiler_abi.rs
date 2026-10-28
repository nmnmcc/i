//! Verify a real compiler artifact against the same host ABI corpus as browsers.
//! Run with the Wasm file produced from tests/conformance/projects/host_abi.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use anyhow::{Context, Result, bail, ensure};
use wleam_host::{
    BigInt, BitArray, HostImports, HostOptions, HostReference, Instance, Program, Value,
};

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments
        .next()
        .context("usage: verify_compiler_abi MODULE.wasm")?;
    ensure!(
        arguments.next().is_none(),
        "expected exactly one Wasm module path"
    );
    let bytes = std::fs::read(&path).context("reading compiler-produced host ABI module")?;
    let program = Program::from_bytes(&bytes)?;
    let fail_strings = Arc::new(AtomicBool::new(false));
    let callbacks = Arc::new(AtomicUsize::new(0));
    let mut imports = HostImports::default();
    for name in [
        "echo_int",
        "echo_string",
        "echo_bits",
        "echo_ref",
        "echo_bool",
        "echo_float",
        "echo_nil",
    ] {
        let fail_strings = Arc::clone(&fail_strings);
        let callbacks = Arc::clone(&callbacks);
        imports.define("custom", name, move |arguments| {
            callbacks.fetch_add(1, Ordering::Relaxed);
            if name == "echo_string" && fail_strings.load(Ordering::Relaxed) {
                bail!("intentional compiler ABI host callback failure");
            }
            Ok(arguments.first().cloned().unwrap_or(Value::Nil))
        })?;
    }
    let mut instance = program.instantiate(HostOptions {
        imports,
        ..HostOptions::default()
    })?;
    let mut passed = 0;

    let huge = (BigInt::from(1_u32) << 4096_u32) + BigInt::from(0x1234_5678_u32);
    for integer in [
        BigInt::from(0),
        BigInt::from(-1),
        BigInt::from(i32::MIN),
        BigInt::from(i32::MAX),
        huge.clone(),
        -huge,
    ] {
        roundtrip(&mut instance, "echo_int", Value::Int(integer))?;
        passed += 1;
    }
    for string in [
        String::new(),
        "e\u{301} 🐉 中文".into(),
        "one\0two".into(),
        "λ".repeat(65_537),
    ] {
        roundtrip(&mut instance, "echo_string", Value::String(string))?;
        passed += 1;
    }
    for bits in [
        BitArray::new(vec![], 0)?,
        BitArray::new(vec![0xff, 0x80], 9)?,
    ] {
        roundtrip(&mut instance, "echo_bits", Value::BitArray(bits))?;
        passed += 1;
    }
    for boolean in [false, true] {
        roundtrip(&mut instance, "echo_bool", Value::Bool(boolean))?;
        passed += 1;
    }
    for float in [-0.0_f64, 1.25, -9876.5] {
        roundtrip(&mut instance, "echo_float", Value::Float(float))?;
        passed += 1;
    }
    ensure!(
        matches!(
            instance.call("host_abi.echo_nil", &[Value::Nil])?,
            Value::Nil
        ),
        "Nil callback changed its result"
    );
    passed += 1;

    let reference = HostReference::new(String::from("compiler ABI object"));
    for _ in 0..10 {
        roundtrip(
            &mut instance,
            "echo_ref",
            Value::Externref(reference.clone()),
        )?;
        instance.collect_garbage()?;
        passed += 1;
    }
    roundtrip(&mut instance, "echo_ref", Value::NullExternref)?;
    passed += 1;
    for (left, right, expected) in [
        (
            Value::Externref(reference.clone()),
            Value::Externref(reference.clone()),
            true,
        ),
        (
            Value::Externref(reference.clone()),
            Value::Externref(HostReference::new(String::from("compiler ABI object"))),
            false,
        ),
        (Value::NullExternref, Value::NullExternref, true),
    ] {
        let result = instance.call("host_abi.reference_same", &[left, right])?;
        ensure!(
            matches!(result, Value::Bool(actual) if actual == expected),
            "opaque reference identity mismatch: {result:?}"
        );
        passed += 1;
    }

    let normalized = instance.call("host_abi.unaligned_bits", &[])?;
    ensure!(
        matches!(normalized, Value::BitArray(ref bits) if bits == &BitArray::new(vec![0xff, 0x80], 9)?),
        "guest bit slice was not normalized: {normalized:?}"
    );
    passed += 1;

    // Exercise guest allocations across the host exception boundary, then reuse
    // the instance. Success after every failure catches stale frame/heap state.
    for _ in 0..10 {
        fail_strings.store(true, Ordering::Relaxed);
        let error = instance
            .call(
                "host_abi.echo_string",
                &[Value::String("input allocated before callback".into())],
            )
            .err()
            .context("host callback failure did not trap")?;
        ensure!(
            format!("{error:#}").contains("intentional compiler ABI host callback failure"),
            "host callback error lost: {error:#}"
        );
        fail_strings.store(false, Ordering::Relaxed);
        roundtrip(
            &mut instance,
            "echo_string",
            Value::String("recovered after host failure".into()),
        )?;
        passed += 1;
    }
    ensure!(
        callbacks.load(Ordering::Relaxed) == 49,
        "expected 49 actual guest-to-host callbacks, got {}",
        callbacks.load(Ordering::Relaxed)
    );
    let panic = instance
        .call("host_abi.panic_now", &[])
        .err()
        .context("guest panic did not trap")?;
    ensure!(
        format!("{panic:#}").contains("host_abi intentional guest panic"),
        "guest panic context was lost: {panic:#}"
    );
    roundtrip(
        &mut instance,
        "echo_string",
        Value::String("recovered after guest panic".into()),
    )?;
    passed += 1;
    println!(
        "{}",
        serde_json::json!({ "host": "wasmtime", "passed": passed, "failed": 0, "module": path.to_string_lossy() })
    );
    Ok(())
}

fn roundtrip(instance: &mut Instance, export: &str, expected: Value) -> Result<()> {
    let actual = instance.call(
        &format!("host_abi.{export}"),
        std::slice::from_ref(&expected),
    )?;
    let equal = match (&actual, &expected) {
        (Value::Nil, Value::Nil) | (Value::NullExternref, Value::NullExternref) => true,
        (Value::Bool(actual), Value::Bool(expected)) => actual == expected,
        (Value::Float(actual), Value::Float(expected)) => actual.to_bits() == expected.to_bits(),
        (Value::Int(actual), Value::Int(expected)) => actual == expected,
        (Value::String(actual), Value::String(expected)) => actual == expected,
        (Value::BitArray(actual), Value::BitArray(expected)) => actual == expected,
        (Value::Externref(actual), Value::Externref(expected)) => actual.ptr_eq(expected),
        _ => false,
    };
    ensure!(equal, "{export} changed {expected:?} to {actual:?}");
    Ok(())
}
