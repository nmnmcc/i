//! Integer conversions implemented in the generated Wasm runtime.
//!
//! `$int_parse` returns `(integer, success)` and consumes the entire UTF-8 string.
//! Rendering uses uppercase ASCII digits, matching `gleam/int`.

pub fn wat() -> String {
    r#"
;; Read a magnitude bit. Callers only use valid, nonnegative bit indices.
(func $conv_mag_bit (param $a (ref $limbs)) (param $bit i32) (result i32)
  (i32.and
    (i32.shr_u
      (array.get $limbs (local.get $a) (i32.shr_u (local.get $bit) (i32.const 5)))
      (i32.and (local.get $bit) (i32.const 31)))
    (i32.const 1)))

(func $conv_string (param $bytes (ref $bytes)) (result eqref)
  (struct.new $obj
    (i32.const 3) (i32.const 0) (i64.const 0) (f64.const 0)
    (local.get $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))

;; Round the integer once, directly to the nearest binary64 significand. Building
;; a float by repeatedly multiplying rounded limb prefixes can double-round.
(func $int_to_f64 (param $value eqref) (result f64)
  (local $sign i32) (local $a (ref $limbs)) (local $n i32)
  (local $bits i32) (local $shift i32) (local $i i32)
  (local $q i64) (local $sticky i32) (local $number f64)
  (if (ref.test (ref i31) (local.get $value))
    (then
      (return
        (f64.convert_i32_s (i31.get_s (ref.cast (ref i31) (local.get $value)))))))
  (local.set $sign (call $int_sign (local.get $value)))
  (if (i32.eqz (local.get $sign)) (then (return (f64.const 0))))
  (local.set $a (call $int_limbs (local.get $value)))
  (local.set $n (array.len (local.get $a)))
  (if (i32.gt_u (local.get $n) (i32.const 32)) (then unreachable))
  (local.set $bits
    (i32.sub
      (i32.mul (local.get $n) (i32.const 32))
      (i32.clz
        (array.get $limbs (local.get $a) (i32.sub (local.get $n) (i32.const 1))))))
  (local.set $shift
    (if (result i32) (i32.gt_u (local.get $bits) (i32.const 53))
      (then (i32.sub (local.get $bits) (i32.const 53)))
      (else (i32.const 0))))
  (local.set $i (local.get $bits))
  (block $significand_done
    (loop $significand
      (br_if $significand_done (i32.le_u (local.get $i) (local.get $shift)))
      (local.set $i (i32.sub (local.get $i) (i32.const 1)))
      (local.set $q
        (i64.or
          (i64.shl (local.get $q) (i64.const 1))
          (i64.extend_i32_u (call $conv_mag_bit (local.get $a) (local.get $i)))))
      (br $significand)))
  (if (i32.gt_u (local.get $shift) (i32.const 0))
    (then
      (if (call $conv_mag_bit (local.get $a) (i32.sub (local.get $shift) (i32.const 1)))
        (then
          (local.set $i (i32.const 0))
          (block $sticky_done
            (loop $scan_sticky
              (br_if $sticky_done
                (i32.ge_u (local.get $i) (i32.sub (local.get $shift) (i32.const 1))))
              (if (call $conv_mag_bit (local.get $a) (local.get $i))
                (then (local.set $sticky (i32.const 1)) (br $sticky_done)))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $scan_sticky)))
          (if (i32.or (local.get $sticky) (i32.wrap_i64 (i64.and (local.get $q) (i64.const 1))))
            (then (local.set $q (i64.add (local.get $q) (i64.const 1)))))))))
  (local.set $number
    (f64.mul
      (f64.convert_i64_u (local.get $q))
      (f64.reinterpret_i64
        (i64.shl
          (i64.extend_i32_u (i32.add (local.get $shift) (i32.const 1023)))
          (i64.const 52)))))
  (if (f64.gt (local.get $number) (f64.const 0x1.fffffffffffffp+1023))
    (then unreachable))
  (if (result f64) (i32.lt_s (local.get $sign) (i32.const 0))
    (then (f64.neg (local.get $number)))
    (else (local.get $number))))

;; Decode binary64's exact integer significand and shift it. This avoids any
;; fixed-width intermediate restriction when truncating a finite large float.
(func $int_from_f64 (param $number f64) (result eqref)
  (local $absolute f64) (local $bits i64) (local $mantissa i64)
  (local $exponent i32) (local $shift i32) (local $sign i32)
  (local $a (ref $limbs)) (local $i i32) (local $position i32)
  (local $word i32) (local $small i64)
  (local.set $absolute (f64.abs (local.get $number)))
  (if
    (i32.or
      (f64.ne (local.get $number) (local.get $number))
      (f64.gt (local.get $absolute) (f64.const 0x1.fffffffffffffp+1023)))
    (then unreachable))
  (if (f64.lt (local.get $absolute) (f64.const 1))
    (then (return (ref.i31 (i32.const 0)))))
  (local.set $sign
    (if (result i32) (f64.lt (local.get $number) (f64.const 0))
      (then (i32.const -1)) (else (i32.const 1))))
  (local.set $bits (i64.reinterpret_f64 (local.get $absolute)))
  (local.set $exponent
    (i32.sub
      (i32.wrap_i64 (i64.and (i64.shr_u (local.get $bits) (i64.const 52)) (i64.const 2047)))
      (i32.const 1023)))
  (local.set $mantissa
    (i64.or (i64.and (local.get $bits) (i64.const 0x000fffffffffffff))
      (i64.const 0x0010000000000000)))
  (if (i32.le_s (local.get $exponent) (i32.const 52))
    (then
      (local.set $small
        (i64.shr_u (local.get $mantissa)
          (i64.extend_i32_u (i32.sub (i32.const 52) (local.get $exponent)))))
      (if (i32.lt_s (local.get $sign) (i32.const 0))
        (then (local.set $small (i64.sub (i64.const 0) (local.get $small)))))
      (return (call $int_from_i64 (local.get $small)))))
  (local.set $shift (i32.sub (local.get $exponent) (i32.const 52)))
  (local.set $a
    (array.new_default $limbs
      (i32.shr_u (i32.add (local.get $exponent) (i32.const 32)) (i32.const 5))))
  (block $decoded
    (loop $decode
      (br_if $decoded (i32.ge_u (local.get $i) (i32.const 53)))
      (if (i64.ne
        (i64.and (i64.shr_u (local.get $mantissa) (i64.extend_i32_u (local.get $i))) (i64.const 1))
        (i64.const 0))
        (then
          (local.set $position (i32.add (local.get $i) (local.get $shift)))
          (local.set $word (i32.shr_u (local.get $position) (i32.const 5)))
          (array.set $limbs (local.get $a) (local.get $word)
            (i32.or (array.get $limbs (local.get $a) (local.get $word))
              (i32.shl (i32.const 1) (i32.and (local.get $position) (i32.const 31)))))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $decode)))
  (call $int_pack (local.get $sign) (local.get $a)))

(func $float_truncate (param $value eqref) (result eqref)
  (call $int_from_f64 (call $float_get (local.get $value))))

(func $int_to_float (param $value eqref) (result eqref)
  (call $float_new (call $int_to_f64 (local.get $value))))

(func $float_to_bits (param $value eqref) (result eqref)
  (local $bits i64)
  (local.set $bits (i64.reinterpret_f64 (call $float_get (local.get $value))))
  (call $int_pack (i32.const 1)
    (array.new_fixed $limbs 2
      (i32.wrap_i64 (local.get $bits))
      (i32.wrap_i64 (i64.shr_u (local.get $bits) (i64.const 32))))))

(func $float_from_bits (param $value eqref) (result eqref)
  (local $a (ref $limbs)) (local $n i32) (local $bits i64)
  (if (i32.lt_s (call $int_sign (local.get $value)) (i32.const 0))
    (then unreachable))
  (local.set $a (call $int_limbs (local.get $value)))
  (local.set $n (array.len (local.get $a)))
  (if (i32.gt_u (local.get $n) (i32.const 2)) (then unreachable))
  (if (i32.gt_u (local.get $n) (i32.const 0))
    (then
      (local.set $bits (i64.extend_i32_u (array.get $limbs (local.get $a) (i32.const 0))))))
  (if (i32.eq (local.get $n) (i32.const 2))
    (then
      (local.set $bits
        (i64.or (local.get $bits)
          (i64.shl (i64.extend_i32_u (array.get $limbs (local.get $a) (i32.const 1)))
            (i64.const 32))))))
  (call $float_new (f64.reinterpret_i64 (local.get $bits))))

(func $int_to_string (param $value eqref) (result eqref)
  (call $int_to_base_string (local.get $value) (ref.i31 (i32.const 10))))

;; Divide a private working magnitude in place by the small output base. Each
;; quotient step uses an exact unsigned 64-bit dividend (remainder < base <= 36).
(func $int_to_base_string (param $value eqref) (param $base_value eqref) (result eqref)
  (local $sign i32) (local $base i64) (local $a (ref $limbs))
  (local $work (ref $limbs)) (local $n i32) (local $i i32)
  (local $count i32) (local $remainder i64) (local $dividend i64)
  (local $digit i32) (local $reverse (ref $bytes)) (local $bytes (ref $bytes))
  (if
    (i32.or
      (i32.lt_s (call $int_compare (local.get $base_value) (ref.i31 (i32.const 2))) (i32.const 0))
      (i32.gt_s (call $int_compare (local.get $base_value) (ref.i31 (i32.const 36))) (i32.const 0)))
    (then unreachable))
  (local.set $base (call $int_to_i64 (local.get $base_value)))
  (local.set $sign (call $int_sign (local.get $value)))
  (if (i32.eqz (local.get $sign))
    (then (return (call $conv_string (array.new_fixed $bytes 1 (i32.const 48))))))
  (local.set $a (call $int_limbs (local.get $value)))
  (local.set $n (array.len (local.get $a)))
  (if (i32.gt_u (local.get $n) (i32.const 0x07ffffff)) (then unreachable))
  (local.set $work (array.new_default $limbs (local.get $n)))
  (array.copy $limbs $limbs (local.get $work) (i32.const 0)
    (local.get $a) (i32.const 0) (local.get $n))
  (local.set $reverse
    (array.new_default $bytes (i32.add (i32.mul (local.get $n) (i32.const 32)) (i32.const 1))))
  (block $digits_done
    (loop $digits
      (br_if $digits_done (i32.eqz (local.get $n)))
      (local.set $i (local.get $n))
      (local.set $remainder (i64.const 0))
      (block $division_done
        (loop $division
          (br_if $division_done (i32.eqz (local.get $i)))
          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
          (local.set $dividend
            (i64.or (i64.shl (local.get $remainder) (i64.const 32))
              (i64.extend_i32_u (array.get $limbs (local.get $work) (local.get $i)))))
          (array.set $limbs (local.get $work) (local.get $i)
            (i32.wrap_i64 (i64.div_u (local.get $dividend) (local.get $base))))
          (local.set $remainder (i64.rem_u (local.get $dividend) (local.get $base)))
          (br $division)))
      (local.set $digit (i32.wrap_i64 (local.get $remainder)))
      (array.set $bytes (local.get $reverse) (local.get $count)
        (i32.add (local.get $digit)
          (if (result i32) (i32.lt_u (local.get $digit) (i32.const 10))
            (then (i32.const 48)) (else (i32.const 55)))))
      (local.set $count (i32.add (local.get $count) (i32.const 1)))
      (block $trimmed
        (loop $trim
          (br_if $trimmed (i32.eqz (local.get $n)))
          (br_if $trimmed
            (array.get $limbs (local.get $work) (i32.sub (local.get $n) (i32.const 1))))
          (local.set $n (i32.sub (local.get $n) (i32.const 1)))
          (br $trim)))
      (br $digits)))
  (if (i32.lt_s (local.get $sign) (i32.const 0))
    (then
      (array.set $bytes (local.get $reverse) (local.get $count) (i32.const 45))
      (local.set $count (i32.add (local.get $count) (i32.const 1)))))
  (local.set $bytes (array.new_default $bytes (local.get $count)))
  (local.set $i (i32.const 0))
  (block $reversed
    (loop $reverse_digits
      (br_if $reversed (i32.ge_u (local.get $i) (local.get $count)))
      (array.set $bytes (local.get $bytes) (local.get $i)
        (array.get_u $bytes (local.get $reverse)
          (i32.sub (i32.sub (local.get $count) (local.get $i)) (i32.const 1))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $reverse_digits)))
  (call $conv_string (local.get $bytes)))

;; Return the parsed integer plus a success flag. No partial prefix, whitespace,
;; underscore, non-ASCII digit, or sign without digits is accepted.
(func $int_parse (param $string eqref) (param $base eqref) (result eqref i32)
  (local $bytes (ref $bytes)) (local $n i32) (local $i i32)
  (local $negative i32) (local $digit i32) (local $char i32)
  (local $radix i32) (local $value eqref)
  (block $invalid
    (br_if $invalid
      (i32.or
        (i32.lt_s (call $int_compare (local.get $base) (ref.i31 (i32.const 2))) (i32.const 0))
        (i32.gt_s (call $int_compare (local.get $base) (ref.i31 (i32.const 36))) (i32.const 0))))
    (local.set $radix (i32.wrap_i64 (call $int_to_i64 (local.get $base))))
    (local.set $bytes
      (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $string)))))
    (local.set $n (array.len (local.get $bytes)))
    (br_if $invalid (i32.eqz (local.get $n)))
    (local.set $char (array.get_u $bytes (local.get $bytes) (i32.const 0)))
    (if (i32.or (i32.eq (local.get $char) (i32.const 43)) (i32.eq (local.get $char) (i32.const 45)))
      (then
        (local.set $negative (i32.eq (local.get $char) (i32.const 45)))
        (local.set $i (i32.const 1))))
    (br_if $invalid (i32.eq (local.get $i) (local.get $n)))
    (local.set $value (ref.i31 (i32.const 0)))
    (block $parsed
      (loop $parse_digit
        (br_if $parsed (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $char (array.get_u $bytes (local.get $bytes) (local.get $i)))
        (local.set $digit
          (if (result i32)
            (i32.and (i32.ge_u (local.get $char) (i32.const 48)) (i32.le_u (local.get $char) (i32.const 57)))
            (then (i32.sub (local.get $char) (i32.const 48)))
            (else
              (if (result i32)
                (i32.and (i32.ge_u (local.get $char) (i32.const 65)) (i32.le_u (local.get $char) (i32.const 90)))
                (then (i32.sub (local.get $char) (i32.const 55)))
                (else
                  (if (result i32)
                    (i32.and (i32.ge_u (local.get $char) (i32.const 97)) (i32.le_u (local.get $char) (i32.const 122)))
                    (then (i32.sub (local.get $char) (i32.const 87)))
                    (else (i32.const -1))))))))
        (br_if $invalid (i32.ge_u (local.get $digit) (local.get $radix)))
        (local.set $value
          (call $int_add
            (call $int_mul (local.get $value) (local.get $base))
            (ref.i31 (local.get $digit))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $parse_digit)))
    (if (local.get $negative)
      (then (local.set $value (call $int_neg (local.get $value)))))
    (return (local.get $value) (i32.const 1)))
  (ref.i31 (i32.const 0)) (i32.const 0))
"#
    .to_owned()
}
