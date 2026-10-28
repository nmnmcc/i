//! Shortest, correctly rounded binary64 decimal rendering.
//!
//! The digit loop uses the Dragon interval construction: the exact IEEE value
//! and the two rounding midpoints are represented with the runtime's arbitrary
//! precision integers. All decisions therefore use integer comparisons, even
//! for subnormals and the largest finite value. This implementation is separate
//! from the standard library formatter so `echo` also works without imports.
//!
//! Algorithm background: Steele & White, "How to Print Floating-Point Numbers
//! Accurately" (1990), and Burger & Dybvig, "Printing Floating-Point Numbers
//! Quickly and Accurately" (1996). No floating-point logarithm is used.

pub fn wat() -> String {
    r#"
(func $float_to_string (param $value eqref) (result eqref)
  (call $f64_to_string (call $float_get (local.get $value))))

(func $f64_to_string (param $value f64) (result eqref)
  (local $bits i64) (local $fraction i64) (local $mantissa i64)
  (local $encoded_exp i32) (local $exp i32) (local $negative i32)
  (local $inclusive i32) (local $k i32) (local $n i32) (local $i i32)
  (local $digit i32) (local $comparison i32) (local $down i32) (local $up i32)
  (local $round_up i32) (local $decimal_exp i32) (local $lower i32) (local $upper i32)
  (local $scientific i32) (local $exponent i32) (local $power i32)
  (local $r eqref) (local $s eqref) (local $minus eqref) (local $plus eqref)
  (local $ten eqref) (local $tmp eqref) (local $shift eqref)
  (local $digits (ref null $bytes)) (local $out (ref null $bytes))
  (local $result (ref null $bytes)) (local $at i32)
  (local.set $bits (i64.reinterpret_f64 (local.get $value)))
  (local.set $negative (i32.wrap_i64 (i64.shr_u (local.get $bits) (i64.const 63))))
  (local.set $encoded_exp
    (i32.and (i32.wrap_i64 (i64.shr_u (local.get $bits) (i64.const 52))) (i32.const 2047)))
  (if (i32.eq (local.get $encoded_exp) (i32.const 2047)) (then unreachable))
  (local.set $fraction (i64.and (local.get $bits) (i64.const 4503599627370495)))
  (local.set $out (array.new_default $bytes (i32.const 64)))
  (if (local.get $negative)
    (then
      (array.set $bytes (local.get $out) (i32.const 0) (i32.const 45))
      (local.set $at (i32.const 1))))
  (if (f64.eq (local.get $value) (f64.const 0))
    (then
      (array.set $bytes (local.get $out) (local.get $at) (i32.const 48))
      (array.set $bytes (local.get $out) (i32.add (local.get $at) (i32.const 1)) (i32.const 46))
      (array.set $bytes (local.get $out) (i32.add (local.get $at) (i32.const 2)) (i32.const 48))
      (local.set $result (array.new_default $bytes (i32.add (local.get $at) (i32.const 3))))
      (array.copy $bytes $bytes (local.get $result) (i32.const 0)
        (local.get $out) (i32.const 0) (i32.add (local.get $at) (i32.const 3)))
      (return (call $string_from_bytes (ref.as_non_null (local.get $result))))))

  ;; Decode v = r * 2^exp, with midpoint distances minus/plus. At an
  ;; exact normal power of two, the preceding binade has half the spacing.
  (if (i32.eqz (local.get $encoded_exp))
    (then
      (local.set $mantissa (local.get $fraction))
      (local.set $r (call $int_from_i64 (i64.shl (local.get $mantissa) (i64.const 1))))
      (local.set $minus (ref.i31 (i32.const 1)))
      (local.set $plus (ref.i31 (i32.const 1)))
      (local.set $exp (i32.const -1075)))
    (else
      (local.set $mantissa (i64.or (local.get $fraction) (i64.const 4503599627370496)))
      (local.set $r (call $int_from_i64 (i64.shl (local.get $mantissa) (i64.const 2))))
      (local.set $plus (ref.i31 (i32.const 2)))
      (local.set $minus
        (ref.i31
          (if (result i32)
            (i32.and (i64.eqz (local.get $fraction))
                     (i32.gt_u (local.get $encoded_exp) (i32.const 1)))
            (then (i32.const 1)) (else (i32.const 2)))))
      (local.set $exp (i32.sub (local.get $encoded_exp) (i32.const 1077)))))
  (local.set $inclusive (i64.eqz (i64.and (local.get $mantissa) (i64.const 1))))
  (local.set $s (ref.i31 (i32.const 1)))
  (if (i32.lt_s (local.get $exp) (i32.const 0))
    (then
      (local.set $s
        (call $int_shl (local.get $s)
          (ref.i31 (i32.sub (i32.const 0) (local.get $exp))))))
    (else
      (local.set $shift (ref.i31 (local.get $exp)))
      (local.set $r (call $int_shl (local.get $r) (local.get $shift)))
      (local.set $minus (call $int_shl (local.get $minus) (local.get $shift)))
      (local.set $plus (call $int_shl (local.get $plus) (local.get $shift)))))
  (local.set $ten (ref.i31 (i32.const 10)))
  (local.set $k (i32.const 1))

  ;; Exact normalization: 1 <= r/s < 10. Multiplying numerator and both
  ;; margins together preserves the round-to-nearest acceptance interval.
  (block $large_done
    (loop $large
      (local.set $tmp (call $int_mul (local.get $s) (local.get $ten)))
      (br_if $large_done (i32.lt_s (call $int_compare (local.get $r) (local.get $tmp)) (i32.const 0)))
      (local.set $s (local.get $tmp))
      (local.set $k (i32.add (local.get $k) (i32.const 1)))
      (br $large)))
  (block $small_done
    (loop $small
      (br_if $small_done (i32.ge_s (call $int_compare (local.get $r) (local.get $s)) (i32.const 0)))
      (local.set $r (call $int_mul (local.get $r) (local.get $ten)))
      (local.set $minus (call $int_mul (local.get $minus) (local.get $ten)))
      (local.set $plus (call $int_mul (local.get $plus) (local.get $ten)))
      (local.set $k (i32.sub (local.get $k) (i32.const 1)))
      (br $small)))

  (local.set $digits (array.new_default $bytes (i32.const 32)))
  (block $digits_done
    (loop $generate
      (if (i32.ge_u (local.get $n) (i32.const 32)) (then unreachable))
      (local.set $digit
        (i32.wrap_i64 (call $int_to_i64 (call $int_div (local.get $r) (local.get $s)))))
      (local.set $r (call $int_rem (local.get $r) (local.get $s)))
      (array.set $bytes (local.get $digits) (local.get $n) (local.get $digit))
      (local.set $n (i32.add (local.get $n) (i32.const 1)))
      (local.set $comparison (call $int_compare (local.get $r) (local.get $minus)))
      (local.set $down
        (i32.or (i32.lt_s (local.get $comparison) (i32.const 0))
                (i32.and (local.get $inclusive) (i32.eqz (local.get $comparison)))))
      (local.set $comparison
        (call $int_compare (call $int_add (local.get $r) (local.get $plus)) (local.get $s)))
      (local.set $up
        (i32.or (i32.gt_s (local.get $comparison) (i32.const 0))
                (i32.and (local.get $inclusive) (i32.eqz (local.get $comparison)))))
      (br_if $digits_done (i32.or (local.get $down) (local.get $up)))
      (local.set $r (call $int_mul (local.get $r) (local.get $ten)))
      (local.set $minus (call $int_mul (local.get $minus) (local.get $ten)))
      (local.set $plus (call $int_mul (local.get $plus) (local.get $ten)))
      (br $generate)))

  ;; Where both shortest candidates are valid, choose the closer one. An
  ;; exact halfway decimal tie is resolved towards an even final digit.
  (local.set $comparison
    (call $int_compare (call $int_add (local.get $r) (local.get $r)) (local.get $s)))
  (local.set $round_up
    (i32.and (local.get $up)
      (i32.or (i32.eqz (local.get $down))
        (i32.or (i32.gt_s (local.get $comparison) (i32.const 0))
          (i32.and (i32.eqz (local.get $comparison))
                   (i32.and (local.get $digit) (i32.const 1)))))))
  (if (local.get $round_up)
    (then
      (local.set $i (local.get $n))
      (block $rounded
        (loop $carry
          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
          (local.set $digit (array.get_u $bytes (local.get $digits) (local.get $i)))
          (if (i32.lt_u (local.get $digit) (i32.const 9))
            (then
              (array.set $bytes (local.get $digits) (local.get $i) (i32.add (local.get $digit) (i32.const 1)))
              (br $rounded)))
          (array.set $bytes (local.get $digits) (local.get $i) (i32.const 0))
          (if (i32.eqz (local.get $i))
            (then
              (array.set $bytes (local.get $digits) (i32.const 0) (i32.const 1))
              (local.set $k (i32.add (local.get $k) (i32.const 1)))
              (br $rounded)))
          (br $carry)))))
  (block $trim_done
    (loop $trim
      (br_if $trim_done (i32.le_u (local.get $n) (i32.const 1)))
      (br_if $trim_done
        (array.get_u $bytes (local.get $digits) (i32.sub (local.get $n) (i32.const 1))))
      (local.set $n (i32.sub (local.get $n) (i32.const 1)))
      (br $trim)))

  ;; The presentation follows Erlang's shortest format, including scientific
  ;; notation once integral decimal output would imply precision above 2^53.
  (local.set $decimal_exp (i32.sub (local.get $k) (local.get $n)))
  (local.set $lower
    (if (result i32) (i32.eq (local.get $n) (i32.const 1))
      (then (i32.const -4))
      (else (i32.sub (i32.const -2) (local.get $n)))))
  (local.set $upper
    (if (result i32)
      (i32.or (i32.eq (local.get $n) (i32.const 1))
              (i32.ge_s (local.get $k) (i32.const 11)))
      (then (i32.const 2)) (else (i32.const 1))))
  (local.set $scientific
    (i32.or
      (i32.or (i32.lt_s (local.get $decimal_exp) (local.get $lower))
              (i32.gt_s (local.get $decimal_exp) (local.get $upper)))
      (i32.and (i32.ge_s (local.get $decimal_exp) (i32.const 0))
               (f64.ge (f64.abs (local.get $value)) (f64.const 9007199254740992)))))
  (if (local.get $scientific)
    (then
      (array.set $bytes (local.get $out) (local.get $at)
        (i32.add (array.get_u $bytes (local.get $digits) (i32.const 0)) (i32.const 48)))
      (local.set $at (i32.add (local.get $at) (i32.const 1)))
      (array.set $bytes (local.get $out) (local.get $at) (i32.const 46))
      (local.set $at (i32.add (local.get $at) (i32.const 1)))
      (local.set $i (i32.const 1))
      (block $fraction_done
        (loop $fraction_digits
          (br_if $fraction_done (i32.ge_u (local.get $i) (local.get $n)))
          (array.set $bytes (local.get $out) (local.get $at)
            (i32.add (array.get_u $bytes (local.get $digits) (local.get $i)) (i32.const 48)))
          (local.set $at (i32.add (local.get $at) (i32.const 1)))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $fraction_digits)))
      (if (i32.eq (local.get $n) (i32.const 1))
        (then
          (array.set $bytes (local.get $out) (local.get $at) (i32.const 48))
          (local.set $at (i32.add (local.get $at) (i32.const 1)))))
      (array.set $bytes (local.get $out) (local.get $at) (i32.const 101))
      (local.set $at (i32.add (local.get $at) (i32.const 1)))
      (local.set $exponent (i32.sub (local.get $k) (i32.const 1)))
      (if (i32.lt_s (local.get $exponent) (i32.const 0))
        (then
          (array.set $bytes (local.get $out) (local.get $at) (i32.const 45))
          (local.set $at (i32.add (local.get $at) (i32.const 1)))
          (local.set $exponent (i32.sub (i32.const 0) (local.get $exponent)))))
      (local.set $power
        (if (result i32) (i32.ge_u (local.get $exponent) (i32.const 100))
          (then (i32.const 100))
          (else (if (result i32) (i32.ge_u (local.get $exponent) (i32.const 10))
            (then (i32.const 10)) (else (i32.const 1))))))
      (loop $exponent_digits
        (array.set $bytes (local.get $out) (local.get $at)
          (i32.add (i32.div_u (local.get $exponent) (local.get $power)) (i32.const 48)))
        (local.set $at (i32.add (local.get $at) (i32.const 1)))
        (local.set $exponent (i32.rem_u (local.get $exponent) (local.get $power)))
        (local.set $power (i32.div_u (local.get $power) (i32.const 10)))
        (br_if $exponent_digits (local.get $power))))
    (else
      (if (i32.le_s (local.get $k) (i32.const 0))
        (then
          (array.set $bytes (local.get $out) (local.get $at) (i32.const 48))
          (array.set $bytes (local.get $out) (i32.add (local.get $at) (i32.const 1)) (i32.const 46))
          (local.set $at (i32.add (local.get $at) (i32.const 2)))
          (local.set $i (local.get $k))
          (block $zeroes_done
            (loop $zeroes
              (br_if $zeroes_done (i32.ge_s (local.get $i) (i32.const 0)))
              (array.set $bytes (local.get $out) (local.get $at) (i32.const 48))
              (local.set $at (i32.add (local.get $at) (i32.const 1)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $zeroes)))))
      (local.set $i (i32.const 0))
      (block $fixed_done
        (loop $fixed_digits
          (br_if $fixed_done (i32.ge_u (local.get $i) (local.get $n)))
          (if (i32.and (i32.gt_s (local.get $k) (i32.const 0))
                       (i32.eq (local.get $i) (local.get $k)))
            (then
              (array.set $bytes (local.get $out) (local.get $at) (i32.const 46))
              (local.set $at (i32.add (local.get $at) (i32.const 1)))))
          (array.set $bytes (local.get $out) (local.get $at)
            (i32.add (array.get_u $bytes (local.get $digits) (local.get $i)) (i32.const 48)))
          (local.set $at (i32.add (local.get $at) (i32.const 1)))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $fixed_digits)))
      (if (i32.ge_s (local.get $k) (local.get $n))
        (then
          (block $integer_zeroes_done
            (loop $integer_zeroes
              (br_if $integer_zeroes_done (i32.ge_s (local.get $i) (local.get $k)))
              (array.set $bytes (local.get $out) (local.get $at) (i32.const 48))
              (local.set $at (i32.add (local.get $at) (i32.const 1)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $integer_zeroes)))
          (array.set $bytes (local.get $out) (local.get $at) (i32.const 46))
          (array.set $bytes (local.get $out) (i32.add (local.get $at) (i32.const 1)) (i32.const 48))
          (local.set $at (i32.add (local.get $at) (i32.const 2)))))))
  (local.set $result (array.new_default $bytes (local.get $at)))
  (array.copy $bytes $bytes (local.get $result) (i32.const 0)
    (local.get $out) (i32.const 0) (local.get $at))
  (call $string_from_bytes (ref.as_non_null (local.get $result))))
"#.to_owned()
}

#[cfg(test)]
use crate::runtime::bigint as test_bigint;

#[cfg(test)]
mod tests {
    use super::{test_bigint, wat};

    // These are the fixed reference presentations from Gleam/Erlang's shortest
    // formatter. Hexadecimal IEEE encodings also cover subnormal endpoints and
    // the two neighbors around 1 without depending on Rust literal rounding.
    const CASES: &[(u64, &str)] = &[
        (0x0000_0000_0000_0000, "0.0"),
        (0x8000_0000_0000_0000, "-0.0"),
        (0x3ff0_0000_0000_0000, "1.0"),
        (0xbff0_0000_0000_0000, "-1.0"),
        (0x3fb9_9999_9999_999a, "0.1"),
        (0x3ff1_9999_9999_999a, "1.1"),
        (0x3fef_ffff_ffff_ffff, "0.9999999999999999"),
        (0x3ff0_0000_0000_0001, "1.0000000000000002"),
        (0x0000_0000_0000_0001, "5.0e-324"),
        (0x8000_0000_0000_0001, "-5.0e-324"),
        (0x000f_ffff_ffff_ffff, "2.225073858507201e-308"),
        (0x0010_0000_0000_0000, "2.2250738585072014e-308"),
        (0x7fef_ffff_ffff_ffff, "1.7976931348623157e308"),
        (0xffef_ffff_ffff_ffff, "-1.7976931348623157e308"),
        (0x4340_0000_0000_0000, "9.007199254740992e15"),
        (0x433f_ffff_ffff_ffff, "9007199254740991.0"),
        (0x4059_0000_0000_0000, "100.0"),
        (0x408f_4000_0000_0000, "1.0e3"),
        (0x4415_af1d_78b5_8c40, "1.0e20"),
    ];

    fn fixture() -> Vec<u8> {
        let mut source = String::from(
            r#"(module
(type $bytes (array (mut i8)))
(type $values (array (mut eqref)))
(type $limbs (array (mut i32)))
(type $call (func (param eqref eqref) (result eqref)))
(type $obj (struct (field i32) (field i32) (field i64) (field f64)
  (field (ref null $bytes)) (field (ref null $values))
  (field (ref null $limbs)) (field (ref null $call))))
(func $string_from_bytes (param $bytes (ref $bytes)) (result eqref)
  (struct.new $obj (i32.const 3) (i32.const 0) (i64.const 0) (f64.const 0)
    (local.get $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $float_new (param $value f64) (result eqref)
  (struct.new $obj (i32.const 2) (i32.const 0) (i64.const 0) (local.get $value)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $float_get (param $value eqref) (result f64)
  (struct.get $obj 3 (ref.cast (ref $obj) (local.get $value))))
"#,
        );
        source.push_str(&test_bigint::wat());
        source.push_str(&wat());
        source.push_str(
            r#"
(func $matches (param $bits i64) (param $expected (ref $bytes)) (result i32)
  (local $actual (ref $bytes)) (local $i i32)
  (local.set $actual
    (ref.as_non_null (struct.get $obj 4
      (ref.cast (ref $obj) (call $f64_to_string (f64.reinterpret_i64 (local.get $bits)))))))
  (if (i32.ne (array.len (local.get $actual)) (array.len (local.get $expected)))
    (then (return (i32.const 0))))
  (block $done (loop $bytes
    (br_if $done (i32.ge_u (local.get $i) (array.len (local.get $actual))))
    (if (i32.ne (array.get_u $bytes (local.get $actual) (local.get $i))
                (array.get_u $bytes (local.get $expected) (local.get $i)))
      (then (return (i32.const 0))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $bytes)))
  (i32.const 1))
(func (export "verify") (result i32)
"#,
        );
        for (index, (bits, expected)) in CASES.iter().enumerate() {
            source.push_str(&format!(
                "(if (i32.eqz (call $matches (i64.const {bits}) (array.new_fixed $bytes {}",
                expected.len()
            ));
            for byte in expected.bytes() {
                source.push_str(&format!(" (i32.const {byte})"));
            }
            source.push_str(&format!("))) (then (return (i32.const {}))))\n", index + 1));
        }
        source.push_str(
            r#"(i32.const 0))
(func (export "positive_infinity")
  (drop (call $f64_to_string (f64.const inf))))
(func (export "negative_infinity")
  (drop (call $f64_to_string (f64.const -inf))))
(func (export "nan")
  (drop (call $f64_to_string (f64.const nan))))
)"#,
        );
        wat::parse_str(&source).expect("float formatting fixture must assemble")
    }

    #[test]
    fn shortest_formatter_is_valid_wasm_gc() {
        wasmparser::Validator::new()
            .validate_all(&fixture())
            .expect("float formatting fixture must validate");
    }

    #[test]
    fn formatter_rejects_non_finite_values() {
        let file = tempfile::NamedTempFile::new().expect("create wasm fixture");
        std::fs::write(file.path(), fixture()).expect("write wasm fixture");
        let wasmtime = std::env::var_os("WLEAM_WASMTIME").unwrap_or_else(|| "wasmtime".into());
        for export in ["positive_infinity", "negative_infinity", "nan"] {
            let output = std::process::Command::new(&wasmtime)
                .args(["run", "--invoke", export])
                .arg(file.path())
                .output()
                .expect("Wasmtime is required; run this test in the Devenv shell");
            assert!(!output.status.success(), "{export} must trap");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("unreachable"),
                "{export} failed for a reason other than the non-finite guard: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn shortest_formatter_matches_reference_values() {
        let file = tempfile::NamedTempFile::new().expect("create wasm fixture");
        std::fs::write(file.path(), fixture()).expect("write wasm fixture");
        let wasmtime = std::env::var_os("WLEAM_WASMTIME").unwrap_or_else(|| "wasmtime".into());
        let output = std::process::Command::new(wasmtime)
            .args(["run", "--invoke", "verify"])
            .arg(file.path())
            .output()
            .expect("Wasmtime is required; run this test in the Devenv shell");
        assert!(
            output.status.success(),
            "Wasmtime failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "0",
            "formatter returned the one-based index of a mismatching reference case"
        );
    }
}
