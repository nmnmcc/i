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
mod tests {
    use crate::runtime::tests::Checks;
    use num_bigint::BigInt;

    // Fixed presentations, with hexadecimal IEEE encodings for subnormal
    // endpoints and the neighbours of 1, and each side of every limit of the
    // long form in spec §11.8.
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

    const DECIMAL_CASES: &[(f64, &str)] = &[
        (123.0, "123.0"),
        (12.67, "12.67"),
        (0.0123, "0.0123"),
        (3.0e26, "3.0e26"),
        (1.0e-7, "1.0e-7"),
        (0.0001, "0.0001"),
        (0.00001, "1.0e-5"),
        (0.0015, "0.0015"),
        (0.00015, "1.5e-4"),
        (-0.00123, "-0.00123"),
        (0.000123, "1.23e-4"),
        (120.0, "120.0"),
        (1200.0, "1.2e3"),
        (1230.0, "1230.0"),
        (12300.0, "1.23e4"),
        (12345.0, "12345.0"),
        (123456.0, "123456.0"),
        (1.0e10, "1.0e10"),
        (12345678901.0, "12345678901.0"),
        (123456789010.0, "123456789010.0"),
        (1234567890100.0, "1234567890100.0"),
        (12345678901000.0, "1.2345678901e13"),
        (123400000000.0, "1.234e11"),
        (10000000000.5, "10000000000.5"),
        (2251799813685248.5, "2251799813685248.5"),
        (9007199254740994.0, "9.007199254740994e15"),
        (1.5e300, "1.5e300"),
        (5.0, "5.0"),
        (-2.5e-3, "-0.0025"),
        (0.3, "0.3"),
        (2.0 / 3.0, "0.6666666666666666"),
        (100.5, "100.5"),
    ];

    /// |digits × 10^t − value|, scaled by a positive factor that is the same
    /// for every candidate of one value.
    fn distance(digits: &BigInt, t: i32, value: f64) -> BigInt {
        let bits = value.abs().to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as i32;
        let fraction = bits & ((1 << 52) - 1);
        let (mantissa, q) = if exponent == 0 {
            (fraction, -1074)
        } else {
            (fraction | (1 << 52), exponent - 1075)
        };
        // Multiply both by 2^-q (when q < 0) and 10^-t (when t < 0).
        let mut candidate = digits.clone();
        let mut exact = BigInt::from(mantissa);
        if t >= 0 {
            candidate *= BigInt::from(10).pow(t as u32);
        } else {
            exact *= BigInt::from(10).pow((-t) as u32);
        }
        if q >= 0 {
            exact <<= q as usize;
        } else {
            candidate <<= (-q) as usize;
        }
        let difference = candidate - exact;
        if difference.sign() == num_bigint::Sign::Minus {
            -difference
        } else {
            difference
        }
    }

    /// The shortest digits that read back as `value`; of those, the nearest,
    /// and of two equally near, the one with an even last digit.
    fn shortest(value: f64) -> (String, i32) {
        let scientific = format!("{:e}", value.abs());
        let (mantissa, exponent) = scientific.split_once('e').unwrap();
        let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
        let digits = digits.trim_end_matches('0').to_owned();
        let n = digits.len() as i32;
        let k = exponent.parse::<i32>().unwrap() + 1;
        let t = k - n;
        let found: BigInt = digits.parse().unwrap();
        let mut best: Option<(BigInt, BigInt)> = None;
        for candidate in [&found - 1, found.clone(), &found + 1] {
            let text = candidate.to_string();
            if text.len() != digits.len() || text.ends_with('0') {
                continue;
            }
            if format!("{text}e{t}").parse::<f64>().unwrap() != value.abs() {
                continue;
            }
            let far = distance(&candidate, t, value);
            let better = match &best {
                None => true,
                Some((_, best_far)) => {
                    far < *best_far
                        || (far == *best_far && text.ends_with(['0', '2', '4', '6', '8']))
                }
            };
            if better {
                best = Some((candidate, far));
            }
        }
        (best.unwrap().0.to_string(), k)
    }

    /// The text form of spec §11.8, written independently of the runtime.
    fn reference(value: f64) -> String {
        if value == 0.0 {
            return if value.is_sign_negative() {
                "-0.0"
            } else {
                "0.0"
            }
            .into();
        }
        let (digits, k) = shortest(value);
        let digits = digits.as_str();
        let n = digits.len() as i32;
        let p = k - n;
        let lower = if n == 1 { -4 } else { -2 - n };
        let upper = if n == 1 || k >= 11 { 2 } else { 1 };
        let long = p >= lower && p <= upper && !(p >= 0 && value.abs() >= 2_f64.powi(53));
        let mut out = String::from(if value < 0.0 { "-" } else { "" });
        if long {
            if k <= 0 {
                out.push_str("0.");
                out.push_str(&"0".repeat((-k) as usize));
                out.push_str(digits);
            } else if k >= n {
                out.push_str(digits);
                out.push_str(&"0".repeat((k - n) as usize));
                out.push_str(".0");
            } else {
                out.push_str(&digits[..k as usize]);
                out.push('.');
                out.push_str(&digits[k as usize..]);
            }
        } else {
            out.push_str(&digits[..1]);
            out.push('.');
            out.push_str(if n == 1 { "0" } else { &digits[1..] });
            out.push('e');
            out.push_str(&(k - 1).to_string());
        }
        out
    }

    fn check(checks: &mut Checks, value: f64, expected: &str) {
        checks.string(
            format!("{value:e} ({:#x}) is {expected}", value.to_bits()),
            &format!(
                "(call $f64_to_string (f64.reinterpret_i64 (i64.const {})))",
                value.to_bits() as i64
            ),
            expected,
        );
    }

    #[test]
    fn reference_follows_the_examples_of_the_specification() {
        for (bits, expected) in CASES {
            assert_eq!(reference(f64::from_bits(*bits)), *expected);
        }
        for (value, expected) in DECIMAL_CASES {
            assert_eq!(reference(*value), *expected);
        }
        // 2^-25 is exactly halfway between two 17-digit candidates.
        assert_eq!(reference(2_f64.powi(-25)), "2.9802322387695312e-8");
    }

    #[test]
    fn shortest_formatter_matches_fixed_values() {
        let mut checks = Checks::default();
        for (bits, expected) in CASES {
            check(&mut checks, f64::from_bits(*bits), expected);
        }
        for (value, expected) in DECIMAL_CASES {
            check(&mut checks, *value, expected);
            match expected.strip_prefix('-') {
                Some(positive) => check(&mut checks, -*value, positive),
                None => check(&mut checks, -*value, &format!("-{expected}")),
            }
        }
        checks.run();
    }

    #[test]
    fn shortest_formatter_matches_the_reference_on_many_floats() {
        let mut checks = Checks::default();
        let mut values = Vec::new();
        for exponent in -325..=308 {
            values.push(format!("1e{exponent}").parse::<f64>().unwrap());
            values.push(format!("1.5e{exponent}").parse::<f64>().unwrap());
            values.push(
                format!("9.999999999999999e{exponent}")
                    .parse::<f64>()
                    .unwrap(),
            );
        }
        for exponent in -1074..=1023 {
            let power = 2_f64.powi(exponent);
            values.extend([power, power.next_up(), power.next_down()]);
        }
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        for _ in 0..3000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            values.push(f64::from_bits(state));
        }
        for value in values {
            if value.is_finite() {
                check(&mut checks, value, &reference(value));
            }
        }
        checks.run();
    }

    #[test]
    fn formatter_rejects_non_finite_values() {
        for value in ["inf", "-inf", "nan"] {
            let source = format!(
                "(module {} {} {} (func (export \"format\") (result i32) \
                 (drop (call $f64_to_string (f64.const {value}))) (i32.const 0)))",
                crate::runtime::TYPES,
                crate::runtime::tests::STUB_HOST,
                crate::runtime::wat()
            );
            let result =
                std::panic::catch_unwind(|| crate::runtime::tests::run_export(&source, "format"));
            assert!(result.is_err(), "{value} must trap");
        }
    }
}
