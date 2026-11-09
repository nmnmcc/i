//! Execute the emitted integer conversion routines in a real WasmGC engine.

#[path = "../src/runtime/bigint.rs"]
mod bigint;

use num_bigint::BigInt;
use num_traits::{FromPrimitive, One, ToPrimitive};
use std::fmt::Write;
use std::process::Command;

const PREFIX: &str = r#"(module
(type $bytes (array (mut i8)))
(type $values (array (mut eqref)))
(type $limbs (array (mut i32)))
(type $call (func (param eqref eqref) (result eqref)))
(type $obj (struct
  (field i32) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values))
  (field (ref null $limbs)) (field (ref null $call))))
(func $stop_float unreachable)
(func $float_get (param $x eqref) (result f64)
  (struct.get $obj 3 (ref.cast (ref $obj) (local.get $x))))
(func $float_new (param $x f64) (result eqref)
  (if (i32.or (f64.ne (local.get $x) (local.get $x))
    (f64.gt (f64.abs (local.get $x)) (f64.const 0x1.fffffffffffffp+1023)))
    (then unreachable))
  (struct.new $obj (i32.const 2) (i32.const 0) (i64.const 0) (local.get $x)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
"#;

const HELPERS: &str = r#"
(func $same_string (param $a eqref) (param $b eqref) (result i32)
  (local $aa (ref $bytes)) (local $bb (ref $bytes)) (local $n i32) (local $i i32)
  (local.set $aa (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $a)))))
  (local.set $bb (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $b)))))
  (local.set $n (array.len (local.get $aa)))
  (if (i32.ne (local.get $n) (array.len (local.get $bb))) (then (return (i32.const 0))))
  (block $done (loop $scan
    (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
    (if (i32.ne (array.get_u $bytes (local.get $aa) (local.get $i))
      (array.get_u $bytes (local.get $bb) (local.get $i)))
      (then (return (i32.const 0))))
    (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $scan)))
  (i32.const 1))
(func $parsed (param $string eqref) (param $base eqref) (result eqref)
  (local $value eqref) (local $ok i32)
  (call $int_parse (local.get $string) (local.get $base))
  (local.set $ok) (local.set $value)
  (if (i32.eqz (local.get $ok)) (then unreachable))
  (local.get $value))
(func $parse_fails (param $string eqref) (param $base eqref) (result i32)
  (local $value eqref) (local $ok i32)
  (call $int_parse (local.get $string) (local.get $base))
  (local.set $ok) (local.set $value)
  (i32.and (i32.eqz (local.get $ok)) (i32.eqz (call $int_sign (local.get $value)))))
"#;

fn string(text: &str) -> String {
    let mut result = format!("(call $conv_string (array.new_fixed $bytes {}", text.len());
    for byte in text.bytes() {
        write!(result, " (i32.const {byte})").expect("write to String");
    }
    result.push_str("))");
    result
}

fn integer(value: impl std::fmt::Display) -> String {
    format!(
        "(call $parsed {} (ref.i31 (i32.const 10)))",
        string(&value.to_string())
    )
}

fn float(value: f64) -> String {
    format!("(f64.reinterpret_i64 (i64.const {}))", value.to_bits())
}

fn assert_wasm(checks: &mut String, expression: &str) {
    writeln!(checks, "(if (i32.eqz {expression}) (then unreachable))").expect("write to String");
}

fn module(body: &str) -> String {
    format!("{PREFIX}\n{}\n{HELPERS}\n{body}\n)", bigint::wat())
}

fn execute(wat: &str, export: &str, succeeds: bool) {
    let binary = wat::parse_str(wat).expect("conversion fixture must parse");
    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
        .validate_all(&binary)
        .expect("conversion fixture must validate");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "ilang-bigint-conversion-{}-{stamp}.wasm",
        std::process::id()
    ));
    std::fs::write(&path, binary).expect("write generated module");
    let command = std::env::var_os("WASMTIME").unwrap_or_else(|| "wasmtime".into());
    let output = Command::new(command)
        .args(["run", "--invoke", export])
        .arg(&path)
        .output()
        .expect("Wasmtime is required: use devenv shell or set WASMTIME to its executable");
    let _ = std::fs::remove_file(path);
    assert_eq!(
        output.status.success(),
        succeeds,
        "Wasmtime conversion fixture {export}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !succeeds {
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unreachable"),
            "expected an explicit conversion trap, got {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn integer_float_rounding_truncation_and_base_round_trips() {
    let mut checks = String::new();
    let mut values = vec![
        BigInt::from(0),
        BigInt::from(1),
        BigInt::from(-1),
        (BigInt::one() << 30) - 1,
        BigInt::one() << 30,
        -(BigInt::one() << 30usize) - 1,
        (BigInt::one() << 53) + 1,
        (BigInt::one() << 53) + 3,
        (BigInt::one() << 64) - 1,
        -(BigInt::one() << 128usize) - 1,
        BigInt::from_f64(f64::MAX).expect("finite float is an integer"),
    ];
    // Cover exact ties and values immediately around them, including an odd
    // significand and a carry into the next exponent.
    for significand in [1_u64 << 52, (1_u64 << 52) + 1, (1_u64 << 53) - 1] {
        for shift in [1_usize, 2, 17, 32, 53, 200, 970] {
            for delta in [-1, 0, 1] {
                let value: BigInt =
                    (BigInt::from(significand) << shift) + (BigInt::one() << (shift - 1)) + delta;
                values.push(value.clone());
                values.push(-value);
            }
        }
    }
    for value in &values {
        let expected = value.to_f64().expect("within binary64 exponent range");
        assert!(expected.is_finite());
        assert_wasm(
            &mut checks,
            &format!(
                "(i64.eq (i64.reinterpret_f64 (call $int_to_f64 {})) (i64.const {}))",
                integer(value),
                expected.to_bits()
            ),
        );
    }
    for value in [
        0.0,
        -0.0,
        0.5,
        -0.5,
        1.9,
        -1.9,
        2_f64.powi(53),
        -2_f64.powi(100),
        f64::MAX,
    ] {
        let expected = BigInt::from_f64(value).expect("finite float");
        assert_wasm(
            &mut checks,
            &format!(
                "(call $same_string (call $int_to_string (call $int_from_f64 {})) {})",
                float(value),
                string(&expected.to_string())
            ),
        );
        // Float bit conversions preserve all finite patterns, including -0.
        assert_wasm(
            &mut checks,
            &format!(
                "(i64.eq (i64.reinterpret_f64 (call $float_get (call $float_from_bits (call $float_to_bits (call $float_new {}))))) (i64.const {}))",
                float(value),
                value.to_bits()
            ),
        );
    }
    for base in [2_u32, 3, 8, 10, 16, 36] {
        for value in &values[..10] {
            assert_wasm(
                &mut checks,
                &format!(
                    "(call $same_string (call $int_to_base_string {} (ref.i31 (i32.const {base}))) {})",
                    integer(value),
                    string(&value.to_str_radix(base).to_uppercase())
                ),
            );
        }
    }
    execute(
        &module(&format!("(func (export \"test\")\n{checks}\n)")),
        "test",
        true,
    );
}

#[test]
fn integer_parsing_rejects_invalid_input_without_truncating() {
    let mut checks = String::new();
    for text in [
        "", "+", "-", " 1", "1 ", "1_000", "1.0", "12x", "١٢", "0x12", "++1", "--1",
    ] {
        assert_wasm(
            &mut checks,
            &format!(
                "(call $parse_fails {} (ref.i31 (i32.const 10)))",
                string(text)
            ),
        );
    }
    for base in [
        BigInt::from(-1),
        BigInt::from(0),
        BigInt::from(1),
        BigInt::from(37),
        BigInt::one() << 100,
    ] {
        assert_wasm(
            &mut checks,
            &format!("(call $parse_fails {} {})", string("1"), integer(base)),
        );
    }
    for (text, base, expected) in [
        ("+000", 10, "0"),
        ("-000", 10, "0"),
        ("-fF", 16, "-255"),
        ("zZ", 36, "1295"),
        ("+101", 2, "5"),
    ] {
        assert_wasm(
            &mut checks,
            &format!(
                "(call $same_string (call $int_to_string (call $parsed {} (ref.i31 (i32.const {base})))) {})",
                string(text),
                string(expected)
            ),
        );
    }
    execute(
        &module(&format!("(func (export \"test\")\n{checks}\n)")),
        "test",
        true,
    );
}

#[test]
fn conversions_trap_for_nonfinite_float_and_out_of_range_bit_patterns() {
    for (name, expression) in [
        (
            "nan",
            "(drop (call $int_from_f64 (f64.const nan)))".to_owned(),
        ),
        (
            "infinity",
            "(drop (call $int_from_f64 (f64.const inf)))".to_owned(),
        ),
        (
            "negative_infinity",
            "(drop (call $int_from_f64 (f64.const -inf)))".to_owned(),
        ),
        (
            "integer_overflow",
            format!(
                "(drop (call $int_to_f64 {}))",
                integer(BigInt::one() << 1024)
            ),
        ),
        (
            "negative_bits",
            format!("(drop (call $float_from_bits {}))", integer(-1)),
        ),
        (
            "infinite_bits",
            format!(
                "(drop (call $float_from_bits {}))",
                integer((BigInt::one() << 64) + 0x7ff0_0000_0000_0000_u64)
            ),
        ),
    ] {
        execute(
            &module(&format!("(func (export \"{name}\") {expression})")),
            name,
            false,
        );
    }
}
