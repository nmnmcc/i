//! Arbitrary precision integer primitives emitted into the WasmGC runtime.
//!
//! Small values are signed i31 references. Larger values have a sign and a
//! normalized, little-endian array of unsigned 32-bit limbs. Every arithmetic
//! operation returns the canonical representation and never mutates an input.
//!
//! The operators work out small operands here. When an operand is large they
//! call the i code of `lib/i/big.i` (`$big_add`, … of `ir::RUNTIME_CALLS`),
//! which reads magnitudes through `$big_size` and `$big_limb` and writes
//! private construction storage through `$big_builder`, `$big_put` and
//! `$big_finish`. Only single-digit multiply-add and division live here;
//! their surrounding algorithms remain in i.

#[path = "bigint_bits.rs"]
mod bits;
#[path = "bigint_conversions.rs"]
mod conversions;

pub fn wat() -> String {
    let mut source = BASE_WAT.to_owned();
    source.push_str(&bits::wat());
    source.push_str(&conversions::wat());
    source
}

const BASE_WAT: &str = r#"
(func $int_sign (param $v eqref) (result i32)
  (local $x i32)
  (if (result i32) (ref.test (ref i31) (local.get $v))
    (then
      (local.set $x (i31.get_s (ref.cast (ref i31) (local.get $v))))
      (i32.sub (i32.gt_s (local.get $x) (i32.const 0))
               (i32.lt_s (local.get $x) (i32.const 0))))
    (else (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))))))

(func $int_limbs (param $v eqref) (result (ref $limbs))
  (local $x i32)
  (if (result (ref $limbs)) (ref.test (ref i31) (local.get $v))
    (then
      (local.set $x (i31.get_s (ref.cast (ref i31) (local.get $v))))
      (if (result (ref $limbs)) (i32.eqz (local.get $x))
        (then (array.new_default $limbs (i32.const 0)))
        (else
          (array.new_fixed $limbs 1
            (if (result i32) (i32.lt_s (local.get $x) (i32.const 0))
              (then (i32.sub (i32.const 0) (local.get $x)))
              (else (local.get $x)))))))
    (else
      (ref.as_non_null
        (struct.get $obj 6 (ref.cast (ref $obj) (local.get $v)))))))

(func $mag_len (param $a (ref $limbs)) (result i32)
  (local $n i32)
  (local.set $n (array.len (local.get $a)))
  (block $done
    (loop $trim
      (br_if $done (i32.eqz (local.get $n)))
      (br_if $done
        (array.get $limbs (local.get $a) (i32.sub (local.get $n) (i32.const 1))))
      (local.set $n (i32.sub (local.get $n) (i32.const 1)))
      (br $trim)))
  (local.get $n))

(func $int_pack (param $sign i32) (param $a (ref $limbs)) (result eqref)
  (local $n i32) (local $x i32) (local $trimmed (ref null $limbs))
  (local.set $n (call $mag_len (local.get $a)))
  (if (i32.eqz (local.get $n))
    (then (return (ref.i31 (i32.const 0)))))
  (local.set $sign
    (if (result i32) (i32.lt_s (local.get $sign) (i32.const 0))
      (then (i32.const -1)) (else (i32.const 1))))
  (if (i32.eq (local.get $n) (i32.const 1))
    (then
      (local.set $x (array.get $limbs (local.get $a) (i32.const 0)))
      (if
        (i32.or
          (i32.and (i32.gt_s (local.get $sign) (i32.const 0))
                   (i32.le_u (local.get $x) (i32.const 1073741823)))
          (i32.and (i32.lt_s (local.get $sign) (i32.const 0))
                   (i32.le_u (local.get $x) (i32.const 1073741824))))
        (then
          (return (ref.i31 (i32.mul (local.get $sign) (local.get $x))))))))
  (local.set $trimmed (local.get $a))
  (if (i32.ne (local.get $n) (array.len (local.get $a)))
    (then
      (local.set $trimmed (array.new_default $limbs (local.get $n)))
      (array.copy $limbs $limbs
        (local.get $trimmed) (i32.const 0)
        (local.get $a) (i32.const 0) (local.get $n))))
  (struct.new $obj
    (i32.const 8) (local.get $sign) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (local.get $trimmed) (ref.null $call)))

(func $int_from_i64 (param $v i64) (result eqref)
  (local $sign i32) (local $mag i64)
  (if
    (i32.and (i64.ge_s (local.get $v) (i64.const -1073741824))
             (i64.le_s (local.get $v) (i64.const 1073741823)))
    (then (return (ref.i31 (i32.wrap_i64 (local.get $v))))))
  (local.set $sign
    (if (result i32) (i64.lt_s (local.get $v) (i64.const 0))
      (then (i32.const -1)) (else (i32.const 1))))
  (local.set $mag
    (if (result i64) (i32.lt_s (local.get $sign) (i32.const 0))
      (then (i64.sub (i64.const 0) (local.get $v)))
      (else (local.get $v))))
  (if (i64.le_u (local.get $mag) (i64.const 4294967295))
    (then
      (return
        (call $int_pack (local.get $sign)
          (array.new_fixed $limbs 1 (i32.wrap_i64 (local.get $mag)))))))
  (call $int_pack (local.get $sign)
    (array.new_fixed $limbs 2
      (i32.wrap_i64 (local.get $mag))
      (i32.wrap_i64 (i64.shr_u (local.get $mag) (i64.const 32))))))

(func $int_to_i64 (param $v eqref) (result i64)
  (local $a (ref null $limbs)) (local $n i32) (local $mag i64)
  (if (ref.test (ref i31) (local.get $v))
    (then
      (return (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $v)))))))
  (local.set $a (call $int_limbs (local.get $v)))
  (local.set $n (array.len (local.get $a)))
  (if (i32.gt_u (local.get $n) (i32.const 2)) (then unreachable))
  (local.set $mag (i64.extend_i32_u (array.get $limbs (local.get $a) (i32.const 0))))
  (if (i32.eq (local.get $n) (i32.const 2))
    (then
      (local.set $mag
        (i64.or (local.get $mag)
          (i64.shl
            (i64.extend_i32_u (array.get $limbs (local.get $a) (i32.const 1)))
            (i64.const 32))))))
  (if (result i64) (i32.lt_s (call $int_sign (local.get $v)) (i32.const 0))
    (then
      (if (i64.gt_u (local.get $mag) (i64.const -9223372036854775808)) (then unreachable))
      (i64.sub (i64.const 0) (local.get $mag)))
    (else
      (if (i64.gt_u (local.get $mag) (i64.const 9223372036854775807)) (then unreachable))
      (local.get $mag))))

;; Whether an Int's magnitude is below 2^62: then the operators below work
;; in i64 without overflow (a product is checked), and only larger operands
;; call the i code of `lib/i/big.i`.
(func $int_fits62 (param $v eqref) (result i32)
  (local $a (ref $limbs))
  (if (ref.test (ref i31) (local.get $v)) (then (return (i32.const 1))))
  (local.set $a (ref.as_non_null (struct.get $obj 6 (ref.cast (ref $obj) (local.get $v)))))
  (if (i32.eq (array.len (local.get $a)) (i32.const 1)) (then (return (i32.const 1))))
  (i32.and (i32.eq (array.len (local.get $a)) (i32.const 2))
    (i32.lt_u (array.get $limbs (local.get $a) (i32.const 1)) (i32.const 1073741824))))

(func $int_both62 (param $a eqref) (param $b eqref) (result i32)
  (if (result i32) (call $int_fits62 (local.get $a))
    (then (call $int_fits62 (local.get $b)))
    (else (i32.const 0))))

(func $int_compare (param $a eqref) (param $b eqref) (result i32)
  (local $as i32) (local $bs i32) (local $x i64) (local $y i64)
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (local.set $as (i31.get_s (ref.cast (ref i31) (local.get $a))))
      (local.set $bs (i31.get_s (ref.cast (ref i31) (local.get $b))))
      (return
        (i32.sub (i32.gt_s (local.get $as) (local.get $bs))
                 (i32.lt_s (local.get $as) (local.get $bs))))))
  (if (call $int_both62 (local.get $a) (local.get $b))
    (then
      (local.set $x (call $int_to_i64 (local.get $a)))
      (local.set $y (call $int_to_i64 (local.get $b)))
      (return (i32.sub (i64.gt_s (local.get $x) (local.get $y)) (i64.lt_s (local.get $x) (local.get $y))))))
  ;; A large Int is outside the small range, so its sign decides against a
  ;; small one, and different signs decide between two large ones.
  (local.set $as (call $int_sign (local.get $a)))
  (local.set $bs (call $int_sign (local.get $b)))
  (if (ref.test (ref i31) (local.get $a)) (then (return (i32.sub (i32.const 0) (local.get $bs)))))
  (if (ref.test (ref i31) (local.get $b)) (then (return (local.get $as))))
  (if (i32.ne (local.get $as) (local.get $bs))
    (then (return (i32.sub (i32.gt_s (local.get $as) (local.get $bs)) (i32.lt_s (local.get $as) (local.get $bs))))))
  (i31.get_s (ref.cast (ref i31) (call $big_compare (local.get $a) (local.get $b)))))

(func $int_add (param $a eqref) (param $b eqref) (result eqref)
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then (return (call $int_from_i64
      (i64.add
        (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
        (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (if (result eqref) (call $int_both62 (local.get $a) (local.get $b))
    (then
      (call $int_from_i64
        (i64.add (call $int_to_i64 (local.get $a)) (call $int_to_i64 (local.get $b)))))
    (else (call $big_add (local.get $a) (local.get $b)))))

(func $int_neg (param $v eqref) (result eqref)
  (if (result eqref) (ref.test (ref i31) (local.get $v))
    (then
      (call $int_from_i64
        (i64.sub (i64.const 0)
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $v)))))))
    (else
      (call $int_pack (i32.sub (i32.const 0) (call $int_sign (local.get $v)))
        (call $int_limbs (local.get $v))))))

(func $int_sub (param $a eqref) (param $b eqref) (result eqref)
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then (return (call $int_from_i64
      (i64.sub
        (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
        (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (if (result eqref) (call $int_both62 (local.get $a) (local.get $b))
    (then
      (call $int_from_i64
        (i64.sub (call $int_to_i64 (local.get $a)) (call $int_to_i64 (local.get $b)))))
    (else (call $big_subtract (local.get $a) (local.get $b)))))

(func $int_mul (param $a eqref) (param $b eqref) (result eqref)
  (local $x i64) (local $y i64) (local $p i64)
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then (return (call $int_from_i64
      (i64.mul
        (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
        (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (if (call $int_both62 (local.get $a) (local.get $b))
    (then
      (local.set $x (call $int_to_i64 (local.get $a)))
      (local.set $y (call $int_to_i64 (local.get $b)))
      (local.set $p (i64.mul (local.get $x) (local.get $y)))
      ;; |x|, |y| < 2^62, so x = -1 never meets the most negative product.
      (if (if (result i32) (i64.eqz (local.get $x))
            (then (i32.const 1))
            (else (i64.eq (i64.div_s (local.get $p) (local.get $x)) (local.get $y))))
        (then (return (call $int_from_i64 (local.get $p)))))))
  (call $big_multiply (local.get $a) (local.get $b)))

(func $int_div (param $a eqref) (param $b eqref) (result eqref)
  (local $y i64)
  (if (result eqref) (call $int_both62 (local.get $a) (local.get $b))
    (then
      (local.set $y (call $int_to_i64 (local.get $b)))
      (if (result eqref) (i64.eqz (local.get $y))
        (then (ref.i31 (i32.const 0)))
        (else (call $int_from_i64 (i64.div_s (call $int_to_i64 (local.get $a)) (local.get $y))))))
    (else (call $big_divide (local.get $a) (local.get $b)))))

(func $int_rem (param $a eqref) (param $b eqref) (result eqref)
  (local $y i64)
  (if (result eqref) (call $int_both62 (local.get $a) (local.get $b))
    (then
      (local.set $y (call $int_to_i64 (local.get $b)))
      (if (result eqref) (i64.eqz (local.get $y))
        (then (ref.i31 (i32.const 0)))
        (else (call $int_from_i64 (i64.rem_s (call $int_to_i64 (local.get $a)) (local.get $y))))))
    (else (call $big_remainder (local.get $a) (local.get $b)))))

;; The private primitives of `lib/i/big.i` ("ilang:big"): the sign, and the
;; magnitude as base 2^30 digits, least significant first, as a List(Int).
(func $big_sign (param $v eqref) (result eqref)
  (ref.i31 (call $int_sign (local.get $v))))

(func $big_digits (param $v eqref) (result eqref)
  (local $a (ref $limbs)) (local $n i32) (local $bits i64) (local $k i64)
  (local $position i64) (local $word i32) (local $window i64) (local $out eqref)
  (local.set $out (call $list_nil))
  (local.set $a (call $int_limbs (local.get $v)))
  (local.set $n (call $mag_len (local.get $a)))
  (if (i32.eqz (local.get $n)) (then (return (local.get $out))))
  (local.set $bits
    (i64.sub (i64.mul (i64.extend_i32_u (local.get $n)) (i64.const 32))
      (i64.extend_i32_u
        (i32.clz (array.get $limbs (local.get $a) (i32.sub (local.get $n) (i32.const 1)))))))
  (local.set $k (i64.div_u (i64.add (local.get $bits) (i64.const 29)) (i64.const 30)))
  (block $done
    (loop $digit
      (br_if $done (i64.eqz (local.get $k)))
      (local.set $k (i64.sub (local.get $k) (i64.const 1)))
      (local.set $position (i64.mul (local.get $k) (i64.const 30)))
      (local.set $word (i32.wrap_i64 (i64.shr_u (local.get $position) (i64.const 5))))
      (local.set $window
        (i64.extend_i32_u (array.get $limbs (local.get $a) (local.get $word))))
      (if (i32.lt_u (i32.add (local.get $word) (i32.const 1)) (local.get $n))
        (then
          (local.set $window
            (i64.or (local.get $window)
              (i64.shl
                (i64.extend_i32_u
                  (array.get $limbs (local.get $a) (i32.add (local.get $word) (i32.const 1))))
                (i64.const 32))))))
      (local.set $out
        (call $list_cons
          (ref.i31 (i32.wrap_i64
            (i64.and
              (i64.shr_u (local.get $window) (i64.and (local.get $position) (i64.const 31)))
              (i64.const 1073741823))))
          (local.get $out)))
      (br $digit)))
  (local.get $out))

(func $big_from_digits (param $sign eqref) (param $digits eqref) (result eqref)
  (local $count i32) (local $cursor eqref) (local $a (ref $limbs))
  (local $position i32) (local $word i32) (local $value i64)
  (local.set $cursor (local.get $digits))
  (block $counted
    (loop $count_digits
      (br_if $counted (call $list_is_empty (local.get $cursor)))
      (local.set $count (i32.add (local.get $count) (i32.const 1)))
      (local.set $cursor (call $list_tail (local.get $cursor)))
      (br $count_digits)))
  (local.set $a
    (array.new_default $limbs
      (i32.add (i32.div_u (i32.mul (local.get $count) (i32.const 30)) (i32.const 32)) (i32.const 2))))
  (local.set $cursor (local.get $digits))
  (block $done
    (loop $digit
      (br_if $done (call $list_is_empty (local.get $cursor)))
      (local.set $word (i32.shr_u (local.get $position) (i32.const 5)))
      (local.set $value
        (i64.shl
          (i64.extend_i32_u (i31.get_s (ref.cast (ref i31) (call $list_head (local.get $cursor)))))
          (i64.extend_i32_u (i32.and (local.get $position) (i32.const 31)))))
      (array.set $limbs (local.get $a) (local.get $word)
        (i32.or (array.get $limbs (local.get $a) (local.get $word)) (i32.wrap_i64 (local.get $value))))
      (array.set $limbs (local.get $a) (i32.add (local.get $word) (i32.const 1))
        (i32.or (array.get $limbs (local.get $a) (i32.add (local.get $word) (i32.const 1)))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 32)))))
      (local.set $position (i32.add (local.get $position) (i32.const 30)))
      (local.set $cursor (call $list_tail (local.get $cursor)))
      (br $digit)))
  (call $int_pack (i31.get_s (ref.cast (ref i31) (local.get $sign))) (local.get $a)))
;; The number of base 2^30 digits of the magnitude.
(func $big_size (param $v eqref) (result eqref)
  (local $a (ref $limbs)) (local $n i32) (local $small i32)
  (if (ref.test (ref i31) (local.get $v))
    (then
      (local.set $small (i31.get_s (ref.cast (ref i31) (local.get $v))))
      (if (i32.eqz (local.get $small)) (then (return (ref.i31 (i32.const 0)))))
      (return (ref.i31
        (if (result i32) (i32.eq (local.get $small) (i32.const -1073741824))
          (then (i32.const 2)) (else (i32.const 1)))))))
  (local.set $a (call $int_limbs (local.get $v)))
  (local.set $n (call $mag_len (local.get $a)))
  (if (i32.eqz (local.get $n)) (then (return (ref.i31 (i32.const 0)))))
  (ref.i31 (i32.div_u
    (i32.add
      (i32.sub (i32.mul (local.get $n) (i32.const 32))
        (i32.clz (array.get $limbs (local.get $a) (i32.sub (local.get $n) (i32.const 1)))))
      (i32.const 29))
    (i32.const 30))))

;; The base 2^30 digit of the magnitude at this index, 0 past the end. A
;; small Int is read without making its limbs.
(func $big_limb (param $v eqref) (param $index eqref) (result eqref)
  (local $a (ref null $limbs)) (local $n i32) (local $position i64) (local $word i32)
  (local $window i64) (local $x i64)
  (local.set $position
    (i64.mul (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $index)))) (i64.const 30)))
  (if (ref.test (ref i31) (local.get $v))
    (then
      (local.set $x (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $v)))))
      (if (i64.lt_s (local.get $x) (i64.const 0)) (then (local.set $x (i64.sub (i64.const 0) (local.get $x)))))
      (if (i64.ge_u (local.get $position) (i64.const 60)) (then (return (ref.i31 (i32.const 0)))))
      (return (ref.i31 (i32.wrap_i64 (i64.and (i64.shr_u (local.get $x) (local.get $position))
        (i64.const 1073741823)))))))
  (local.set $a (struct.get $obj 6 (ref.cast (ref $obj) (local.get $v))))
  (local.set $n (array.len (local.get $a)))
  (local.set $word (i32.wrap_i64 (i64.shr_u (local.get $position) (i64.const 5))))
  (if (i32.ge_u (local.get $word) (local.get $n)) (then (return (ref.i31 (i32.const 0)))))
  (local.set $window (i64.extend_i32_u (array.get $limbs (local.get $a) (local.get $word))))
  (if (i32.lt_u (i32.add (local.get $word) (i32.const 1)) (local.get $n))
    (then
      (local.set $window
        (i64.or (local.get $window)
          (i64.shl
            (i64.extend_i32_u (array.get $limbs (local.get $a) (i32.add (local.get $word) (i32.const 1))))
            (i64.const 32))))))
  (ref.i31 (i32.wrap_i64
    (i64.and (i64.shr_u (local.get $window) (i64.and (local.get $position) (i64.const 31)))
      (i64.const 1073741823)))))

;; The Int with this sign and these base 2^30 digits, most significant first.
(func $big_from_top (param $sign eqref) (param $digits eqref) (result eqref)
  (local $count i32) (local $cursor eqref) (local $a (ref $limbs))
  (local $position i32) (local $word i32) (local $value i64)
  (local.set $cursor (local.get $digits))
  (block $counted
    (loop $count_digits
      (br_if $counted (call $list_is_empty (local.get $cursor)))
      (local.set $count (i32.add (local.get $count) (i32.const 1)))
      (local.set $cursor (call $list_tail (local.get $cursor)))
      (br $count_digits)))
  (local.set $a
    (array.new_default $limbs
      (i32.add (i32.div_u (i32.mul (local.get $count) (i32.const 30)) (i32.const 32)) (i32.const 2))))
  (local.set $cursor (local.get $digits))
  (local.set $position (i32.mul (local.get $count) (i32.const 30)))
  (block $done
    (loop $digit
      (br_if $done (call $list_is_empty (local.get $cursor)))
      (local.set $position (i32.sub (local.get $position) (i32.const 30)))
      (local.set $word (i32.shr_u (local.get $position) (i32.const 5)))
      (local.set $value
        (i64.shl
          (i64.extend_i32_u (i31.get_s (ref.cast (ref i31) (call $list_head (local.get $cursor)))))
          (i64.extend_i32_u (i32.and (local.get $position) (i32.const 31)))))
      (array.set $limbs (local.get $a) (local.get $word)
        (i32.or (array.get $limbs (local.get $a) (local.get $word)) (i32.wrap_i64 (local.get $value))))
      (array.set $limbs (local.get $a) (i32.add (local.get $word) (i32.const 1))
        (i32.or (array.get $limbs (local.get $a) (i32.add (local.get $word) (i32.const 1)))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 32)))))
      (local.set $cursor (call $list_tail (local.get $cursor)))
      (br $digit)))
  (call $int_pack (i31.get_s (ref.cast (ref i31) (local.get $sign))) (local.get $a)))

;; Private construction storage, packed directly in the native limb layout.
;; Each base-2^30 digit is written once. No writes follow big_finish, so an
;; existing Int can never be mutated through these private primitives.
(func $big_builder (param $n eqref) (result eqref)
  (array.new_default $limbs
    (i32.div_u
      (i32.add (i32.mul (i31.get_s (ref.cast (ref i31) (local.get $n))) (i32.const 30))
        (i32.const 31))
      (i32.const 32))))

(func $big_put (param $out eqref) (param $index eqref) (param $digit eqref) (result eqref)
  (local $a (ref $limbs)) (local $position i32) (local $word i32) (local $value i64)
  (local.set $a (ref.cast (ref $limbs) (local.get $out)))
  (local.set $position (i32.mul (i31.get_s (ref.cast (ref i31) (local.get $index))) (i32.const 30)))
  (local.set $word (i32.shr_u (local.get $position) (i32.const 5)))
  (local.set $value
    (i64.shl (i64.extend_i32_u (i31.get_s (ref.cast (ref i31) (local.get $digit))))
      (i64.extend_i32_u (i32.and (local.get $position) (i32.const 31)))))
  (array.set $limbs (local.get $a) (local.get $word)
    (i32.or (array.get $limbs (local.get $a) (local.get $word)) (i32.wrap_i64 (local.get $value))))
  (if (i32.lt_u (i32.add (local.get $word) (i32.const 1)) (array.len (local.get $a)))
    (then
      (array.set $limbs (local.get $a) (i32.add (local.get $word) (i32.const 1))
        (i32.or (array.get $limbs (local.get $a) (i32.add (local.get $word) (i32.const 1)))
          (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 32)))))))
  (local.get $out))

(func $big_finish (param $sign eqref) (param $out eqref) (result eqref)
  (call $int_pack (i31.get_s (ref.cast (ref i31) (local.get $sign)))
    (ref.cast (ref $limbs) (local.get $out))))

;; Only single-digit arithmetic lives here; the algorithms and loops are i.
(func $big_multiply_add (param $a eqref) (param $b eqref) (param $carry eqref) (result eqref)
  (local $p i64)
  (local.set $p
    (i64.add
      (i64.mul (i64.extend_i32_u (i31.get_s (ref.cast (ref i31) (local.get $a))))
        (i64.extend_i32_u (i31.get_s (ref.cast (ref i31) (local.get $b)))))
      (i64.extend_i32_u (i31.get_s (ref.cast (ref i31) (local.get $carry))))))
  (call $tuple (array.new_fixed $values 2
    (ref.i31 (i32.wrap_i64 (i64.and (local.get $p) (i64.const 1073741823))))
    (ref.i31 (i32.wrap_i64 (i64.shr_u (local.get $p) (i64.const 30)))))))

(func $big_divide_digit (param $high eqref) (param $low eqref) (param $d eqref) (result eqref)
  (local $v i64) (local $divisor i64) (local $q i64)
  (local.set $v
    (i64.or
      (i64.shl (i64.extend_i32_u (i31.get_s (ref.cast (ref i31) (local.get $high)))) (i64.const 30))
      (i64.extend_i32_u (i31.get_s (ref.cast (ref i31) (local.get $low))))))
  (local.set $divisor (call $int_to_i64 (local.get $d)))
  (local.set $q (i64.div_u (local.get $v) (local.get $divisor)))
  (call $tuple (array.new_fixed $values 2
    (ref.i31 (i32.wrap_i64 (local.get $q)))
    (ref.i31 (i32.wrap_i64 (i64.sub (local.get $v) (i64.mul (local.get $q) (local.get $divisor))))))))
"#;
