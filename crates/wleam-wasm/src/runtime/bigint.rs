//! Arbitrary precision integer primitives emitted into the WasmGC runtime.
//!
//! Small values are signed i31 references. Larger values have a sign and a
//! normalized, little-endian array of unsigned 32-bit limbs. Every arithmetic
//! operation returns the canonical representation and never mutates an input.

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

(func $mag_compare (param $a (ref $limbs)) (param $b (ref $limbs)) (result i32)
  (local $an i32) (local $bn i32) (local $x i32) (local $y i32)
  (local.set $an (call $mag_len (local.get $a)))
  (local.set $bn (call $mag_len (local.get $b)))
  (if (i32.lt_u (local.get $an) (local.get $bn)) (then (return (i32.const -1))))
  (if (i32.gt_u (local.get $an) (local.get $bn)) (then (return (i32.const 1))))
  (block $done
    (loop $scan
      (br_if $done (i32.eqz (local.get $an)))
      (local.set $an (i32.sub (local.get $an) (i32.const 1)))
      (local.set $x (array.get $limbs (local.get $a) (local.get $an)))
      (local.set $y (array.get $limbs (local.get $b) (local.get $an)))
      (if (i32.lt_u (local.get $x) (local.get $y)) (then (return (i32.const -1))))
      (if (i32.gt_u (local.get $x) (local.get $y)) (then (return (i32.const 1))))
      (br $scan)))
  (i32.const 0))

(func $int_compare (param $a eqref) (param $b eqref) (result i32)
  (local $as i32) (local $bs i32)
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (local.set $as (i31.get_s (ref.cast (ref i31) (local.get $a))))
      (local.set $bs (i31.get_s (ref.cast (ref i31) (local.get $b))))
      (return
        (i32.sub (i32.gt_s (local.get $as) (local.get $bs))
                 (i32.lt_s (local.get $as) (local.get $bs))))))
  (local.set $as (call $int_sign (local.get $a)))
  (local.set $bs (call $int_sign (local.get $b)))
  (if (i32.lt_s (local.get $as) (local.get $bs)) (then (return (i32.const -1))))
  (if (i32.gt_s (local.get $as) (local.get $bs)) (then (return (i32.const 1))))
  (if (i32.eqz (local.get $as)) (then (return (i32.const 0))))
  (i32.mul (local.get $as)
    (call $mag_compare (call $int_limbs (local.get $a)) (call $int_limbs (local.get $b)))))

(func $mag_add (param $a (ref $limbs)) (param $b (ref $limbs)) (result (ref $limbs))
  (local $out (ref null $limbs))
  (local $an i32) (local $bn i32) (local $n i32) (local $i i32)
  (local $carry i64) (local $sum i64)
  (local.set $an (array.len (local.get $a)))
  (local.set $bn (array.len (local.get $b)))
  (local.set $n
    (if (result i32) (i32.gt_u (local.get $an) (local.get $bn))
      (then (local.get $an)) (else (local.get $bn))))
  (local.set $out (array.new_default $limbs (i32.add (local.get $n) (i32.const 1))))
  (block $done
    (loop $add
      (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
      (local.set $sum (local.get $carry))
      (if (i32.lt_u (local.get $i) (local.get $an))
        (then
          (local.set $sum (i64.add (local.get $sum)
            (i64.extend_i32_u (array.get $limbs (local.get $a) (local.get $i)))))))
      (if (i32.lt_u (local.get $i) (local.get $bn))
        (then
          (local.set $sum (i64.add (local.get $sum)
            (i64.extend_i32_u (array.get $limbs (local.get $b) (local.get $i)))))))
      (array.set $limbs (local.get $out) (local.get $i) (i32.wrap_i64 (local.get $sum)))
      (local.set $carry (i64.shr_u (local.get $sum) (i64.const 32)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $add)))
  (array.set $limbs (local.get $out) (local.get $n) (i32.wrap_i64 (local.get $carry)))
  (ref.as_non_null (local.get $out)))

;; Magnitude subtraction; the caller proves a >= b.
(func $mag_sub (param $a (ref $limbs)) (param $b (ref $limbs)) (result (ref $limbs))
  (local $out (ref null $limbs)) (local $an i32) (local $bn i32) (local $i i32)
  (local $borrow i64) (local $x i64) (local $y i64)
  (local.set $an (array.len (local.get $a)))
  (local.set $bn (array.len (local.get $b)))
  (local.set $out (array.new_default $limbs (local.get $an)))
  (block $done
    (loop $sub
      (br_if $done (i32.ge_u (local.get $i) (local.get $an)))
      (local.set $x (i64.extend_i32_u (array.get $limbs (local.get $a) (local.get $i))))
      (local.set $y (local.get $borrow))
      (if (i32.lt_u (local.get $i) (local.get $bn))
        (then
          (local.set $y (i64.add (local.get $y)
            (i64.extend_i32_u (array.get $limbs (local.get $b) (local.get $i)))))))
      (array.set $limbs (local.get $out) (local.get $i)
        (i32.wrap_i64 (i64.sub (local.get $x) (local.get $y))))
      (local.set $borrow (i64.extend_i32_u (i64.lt_u (local.get $x) (local.get $y))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $sub)))
  (ref.as_non_null (local.get $out)))

(func $int_add (param $a eqref) (param $b eqref) (result eqref)
  (local $as i32) (local $bs i32) (local $cmp i32)
  (local $al (ref null $limbs)) (local $bl (ref null $limbs))
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (return (call $int_from_i64
        (i64.add
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (local.set $as (call $int_sign (local.get $a)))
  (local.set $bs (call $int_sign (local.get $b)))
  (if (i32.eqz (local.get $as)) (then (return (local.get $b))))
  (if (i32.eqz (local.get $bs)) (then (return (local.get $a))))
  (local.set $al (call $int_limbs (local.get $a)))
  (local.set $bl (call $int_limbs (local.get $b)))
  (if (i32.eq (local.get $as) (local.get $bs))
    (then
      (return (call $int_pack (local.get $as)
        (call $mag_add (ref.as_non_null (local.get $al)) (ref.as_non_null (local.get $bl)))))))
  (local.set $cmp
    (call $mag_compare (ref.as_non_null (local.get $al)) (ref.as_non_null (local.get $bl))))
  (if (i32.eqz (local.get $cmp)) (then (return (ref.i31 (i32.const 0)))))
  (if (result eqref) (i32.gt_s (local.get $cmp) (i32.const 0))
    (then
      (call $int_pack (local.get $as)
        (call $mag_sub (ref.as_non_null (local.get $al)) (ref.as_non_null (local.get $bl)))))
    (else
      (call $int_pack (local.get $bs)
        (call $mag_sub (ref.as_non_null (local.get $bl)) (ref.as_non_null (local.get $al)))))))

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
  (if (result eqref)
    (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (call $int_from_i64
        (i64.sub
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))
    (else (call $int_add (local.get $a) (call $int_neg (local.get $b))))))

(func $mag_mul (param $a (ref $limbs)) (param $b (ref $limbs)) (result (ref $limbs))
  (local $out (ref null $limbs)) (local $an i32) (local $bn i32)
  (local $i i32) (local $j i32) (local $k i32)
  (local $carry i64) (local $product i64) (local $x i64)
  (local.set $an (array.len (local.get $a)))
  (local.set $bn (array.len (local.get $b)))
  (local.set $out (array.new_default $limbs (i32.add (local.get $an) (local.get $bn))))
  (block $done
    (loop $rows
      (br_if $done (i32.ge_u (local.get $i) (local.get $an)))
      (local.set $x (i64.extend_i32_u (array.get $limbs (local.get $a) (local.get $i))))
      (local.set $carry (i64.const 0))
      (local.set $j (i32.const 0))
      (block $row_done
        (loop $columns
          (br_if $row_done (i32.ge_u (local.get $j) (local.get $bn)))
          (local.set $k (i32.add (local.get $i) (local.get $j)))
          (local.set $product
            (i64.add
              (i64.add
                (i64.mul (local.get $x)
                  (i64.extend_i32_u (array.get $limbs (local.get $b) (local.get $j))))
                (i64.extend_i32_u (array.get $limbs (local.get $out) (local.get $k))))
              (local.get $carry)))
          (array.set $limbs (local.get $out) (local.get $k) (i32.wrap_i64 (local.get $product)))
          (local.set $carry (i64.shr_u (local.get $product) (i64.const 32)))
          (local.set $j (i32.add (local.get $j) (i32.const 1)))
          (br $columns)))
      (array.set $limbs (local.get $out) (i32.add (local.get $i) (local.get $bn))
        (i32.wrap_i64 (local.get $carry)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $rows)))
  (ref.as_non_null (local.get $out)))

(func $int_mul (param $a eqref) (param $b eqref) (result eqref)
  (local $sign i32)
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (return (call $int_from_i64
        (i64.mul
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (local.set $sign (i32.mul (call $int_sign (local.get $a)) (call $int_sign (local.get $b))))
  (if (i32.eqz (local.get $sign)) (then (return (ref.i31 (i32.const 0)))))
  (call $int_pack (local.get $sign)
    (call $mag_mul (call $int_limbs (local.get $a)) (call $int_limbs (local.get $b)))))

;; Binary long division. All writes target newly allocated q and r arrays.
;; The remainder carries one extra limb while it is shifted, so no leading
;; carry is lost even when the divisor's highest bit is set.
(func $mag_divrem (param $a (ref $limbs)) (param $b (ref $limbs))
  (result (ref $limbs) (ref $limbs))
  (local $q (ref null $limbs)) (local $r (ref null $limbs))
  (local $an i32) (local $bn i32) (local $rn i32)
  (local $bit i64) (local $i i32) (local $word i32) (local $shift i32)
  (local $carry i64) (local $sum i64) (local $x i64) (local $y i64) (local $borrow i64)
  (local.set $an (call $mag_len (local.get $a)))
  (local.set $bn (call $mag_len (local.get $b)))
  (if (i32.eqz (local.get $bn)) (then unreachable))
  (if (i32.lt_s (call $mag_compare (local.get $a) (local.get $b)) (i32.const 0))
    (then (return (array.new_default $limbs (i32.const 0)) (local.get $a))))
  ;; Common divisors such as 10 and 36 need a single pass over the limbs.
  ;; carry < divisor <= u32::MAX keeps the combined dividend within u64.
  (if (i32.eq (local.get $bn) (i32.const 1))
    (then
      (local.set $q (array.new_default $limbs (local.get $an)))
      (local.set $x (i64.extend_i32_u (array.get $limbs (local.get $b) (i32.const 0))))
      (local.set $i (local.get $an))
      (local.set $carry (i64.const 0))
      (block $small_done
        (loop $small_divide
          (br_if $small_done (i32.eqz (local.get $i)))
          (local.set $i (i32.sub (local.get $i) (i32.const 1)))
          (local.set $sum
            (i64.or (i64.shl (local.get $carry) (i64.const 32))
              (i64.extend_i32_u (array.get $limbs (local.get $a) (local.get $i)))))
          (array.set $limbs (local.get $q) (local.get $i)
            (i32.wrap_i64 (i64.div_u (local.get $sum) (local.get $x))))
          (local.set $carry (i64.rem_u (local.get $sum) (local.get $x)))
          (br $small_divide)))
      (return (ref.as_non_null (local.get $q))
        (array.new_fixed $limbs 1 (i32.wrap_i64 (local.get $carry))))))
  (local.set $rn (i32.add (local.get $bn) (i32.const 1)))
  (local.set $q (array.new_default $limbs (local.get $an)))
  (local.set $r (array.new_default $limbs (local.get $rn)))
  (local.set $bit (i64.mul (i64.extend_i32_u (local.get $an)) (i64.const 32)))
  (block $done
    (loop $bits
      (br_if $done (i64.eqz (local.get $bit)))
      (local.set $bit (i64.sub (local.get $bit) (i64.const 1)))
      (local.set $word (i32.wrap_i64 (i64.shr_u (local.get $bit) (i64.const 5))))
      (local.set $shift (i32.and (i32.wrap_i64 (local.get $bit)) (i32.const 31)))
      (local.set $carry (i64.extend_i32_u
        (i32.and
          (i32.shr_u (array.get $limbs (local.get $a) (local.get $word)) (local.get $shift))
          (i32.const 1))))
      (local.set $i (i32.const 0))
      (block $shift_done
        (loop $shift_r
          (br_if $shift_done (i32.ge_u (local.get $i) (local.get $rn)))
          (local.set $sum
            (i64.or
              (i64.shl (i64.extend_i32_u (array.get $limbs (local.get $r) (local.get $i)))
                (i64.const 1))
              (local.get $carry)))
          (array.set $limbs (local.get $r) (local.get $i) (i32.wrap_i64 (local.get $sum)))
          (local.set $carry (i64.shr_u (local.get $sum) (i64.const 32)))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $shift_r)))
      (if
        (i32.ge_s
          (call $mag_compare (ref.as_non_null (local.get $r)) (local.get $b))
          (i32.const 0))
        (then
          (local.set $i (i32.const 0))
          (local.set $borrow (i64.const 0))
          (block $sub_done
            (loop $sub_r
              (br_if $sub_done (i32.ge_u (local.get $i) (local.get $rn)))
              (local.set $x (i64.extend_i32_u (array.get $limbs (local.get $r) (local.get $i))))
              (local.set $y (local.get $borrow))
              (if (i32.lt_u (local.get $i) (local.get $bn))
                (then
                  (local.set $y
                    (i64.add (local.get $y)
                      (i64.extend_i32_u (array.get $limbs (local.get $b) (local.get $i)))))))
              (array.set $limbs (local.get $r) (local.get $i)
                (i32.wrap_i64 (i64.sub (local.get $x) (local.get $y))))
              (local.set $borrow (i64.extend_i32_u (i64.lt_u (local.get $x) (local.get $y))))
              (local.set $i (i32.add (local.get $i) (i32.const 1)))
              (br $sub_r)))
          (array.set $limbs (local.get $q) (local.get $word)
            (i32.or (array.get $limbs (local.get $q) (local.get $word))
              (i32.shl (i32.const 1) (local.get $shift))))))
      (br $bits)))
  (ref.as_non_null (local.get $q))
  (ref.as_non_null (local.get $r)))

(func $int_div (param $a eqref) (param $b eqref) (result eqref)
  (local $q (ref null $limbs)) (local $r (ref null $limbs))
  (if (i32.eqz (call $int_sign (local.get $b)))
    (then (return (ref.i31 (i32.const 0)))))
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (return (call $int_from_i64
        (i64.div_s
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (call $mag_divrem (call $int_limbs (local.get $a)) (call $int_limbs (local.get $b)))
  (local.set $r) (local.set $q)
  (call $int_pack (i32.mul (call $int_sign (local.get $a)) (call $int_sign (local.get $b)))
    (ref.as_non_null (local.get $q))))

(func $int_rem (param $a eqref) (param $b eqref) (result eqref)
  (local $q (ref null $limbs)) (local $r (ref null $limbs))
  (if (i32.eqz (call $int_sign (local.get $b)))
    (then (return (ref.i31 (i32.const 0)))))
  (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
    (then
      (return (call $int_from_i64
        (i64.rem_s
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $a))))
          (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $b)))))))))
  (call $mag_divrem (call $int_limbs (local.get $a)) (call $int_limbs (local.get $b)))
  (local.set $r) (local.set $q)
  (call $int_pack (call $int_sign (local.get $a)) (ref.as_non_null (local.get $r))))
"#;
