//! Infinite two's-complement operations over the GC-backed integer representation.
//!
//! The finite limb buffers used by bitwise operators contain one extra sign word.
//! They are converted back to a normalized signed magnitude before returning.

pub fn wat() -> String {
    r#"
  (func $int_twos (param $v eqref) (param $n i32) (result (ref $limbs))
    (local $m (ref $limbs)) (local $out (ref $limbs))
    (local $i i32) (local $size i32) (local $negative i32)
    (local $carry i64) (local $word i64)
    (local.set $m (call $int_limbs (local.get $v)))
    (local.set $size (array.len (local.get $m)))
    (local.set $out (array.new_default $limbs (local.get $n)))
    (local.set $negative (i32.lt_s (call $int_sign (local.get $v)) (i32.const 0)))
    (local.set $carry (i64.extend_i32_u (local.get $negative)))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $word
          (if (result i64) (i32.lt_u (local.get $i) (local.get $size))
            (then (i64.extend_i32_u (array.get $limbs (local.get $m) (local.get $i))))
            (else (i64.const 0))))
        (if (local.get $negative)
          (then
            (local.set $word
              (i64.add (i64.xor (local.get $word) (i64.const 4294967295)) (local.get $carry)))
            (local.set $carry (i64.shr_u (local.get $word) (i64.const 32)))))
        (array.set $limbs (local.get $out) (local.get $i) (i32.wrap_i64 (local.get $word)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (local.get $out))

  (func $int_from_twos (param $words (ref $limbs)) (result eqref)
    (local $n i32) (local $i i32) (local $carry i64) (local $word i64)
    (local.set $n (array.len (local.get $words)))
    (if (i32.eqz (local.get $n))
      (then (return (ref.i31 (i32.const 0)))))
    (if (i32.ge_s
          (array.get $limbs (local.get $words) (i32.sub (local.get $n) (i32.const 1)))
          (i32.const 0))
      (then (return (call $int_pack (i32.const 1) (local.get $words)))))
    (local.set $carry (i64.const 1))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $word
          (i64.add
            (i64.extend_i32_u (i32.xor (array.get $limbs (local.get $words) (local.get $i)) (i32.const -1)))
            (local.get $carry)))
        (array.set $limbs (local.get $words) (local.get $i) (i32.wrap_i64 (local.get $word)))
        (local.set $carry (i64.shr_u (local.get $word) (i64.const 32)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (call $int_pack (i32.const -1) (local.get $words)))

  (func $int_bitop (param $a eqref) (param $b eqref) (param $op i32) (result eqref)
    (local $an i32) (local $bn i32) (local $n i32) (local $i i32)
    (local $aa (ref $limbs)) (local $bb (ref $limbs))
    (local $x i32) (local $y i32)
    (if (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b)))
      (then
        (local.set $x (i31.get_s (ref.cast (ref i31) (local.get $a))))
        (local.set $y (i31.get_s (ref.cast (ref i31) (local.get $b))))
        (return
          (ref.i31
            (if (result i32) (i32.eqz (local.get $op))
              (then (i32.and (local.get $x) (local.get $y)))
              (else
                (if (result i32) (i32.eq (local.get $op) (i32.const 1))
                  (then (i32.or (local.get $x) (local.get $y)))
                  (else (i32.xor (local.get $x) (local.get $y))))))))))
    (local.set $an (array.len (call $int_limbs (local.get $a))))
    (local.set $bn (array.len (call $int_limbs (local.get $b))))
    (local.set $n
      (select (local.get $an) (local.get $bn) (i32.gt_u (local.get $an) (local.get $bn))))
    (if (i32.eq (local.get $n) (i32.const -1)) (then unreachable))
    (local.set $n (i32.add (local.get $n) (i32.const 1)))
    (local.set $aa (call $int_twos (local.get $a) (local.get $n)))
    (local.set $bb (call $int_twos (local.get $b) (local.get $n)))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $x (array.get $limbs (local.get $aa) (local.get $i)))
        (local.set $y (array.get $limbs (local.get $bb) (local.get $i)))
        (array.set $limbs (local.get $aa) (local.get $i)
          (if (result i32) (i32.eqz (local.get $op))
            (then (i32.and (local.get $x) (local.get $y)))
            (else
              (if (result i32) (i32.eq (local.get $op) (i32.const 1))
                (then (i32.or (local.get $x) (local.get $y)))
                (else (i32.xor (local.get $x) (local.get $y)))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (call $int_from_twos (local.get $aa)))

  (func $int_and (param $a eqref) (param $b eqref) (result eqref)
    ;; HAMT bitmap/index masks only inspect the low word. In particular an
    ;; arbitrary-size integer AND 31 never needs a two's-complement buffer.
    (if (ref.test (ref i31) (local.get $a))
      (then
        (if (i32.ge_s (i31.get_s (ref.cast (ref i31) (local.get $a))) (i32.const 0))
          (then
            (return (ref.i31
              (i32.and (i31.get_s (ref.cast (ref i31) (local.get $a)))
                (call $int_low_i32 (local.get $b)))))))))
    (if (ref.test (ref i31) (local.get $b))
      (then
        (if (i32.ge_s (i31.get_s (ref.cast (ref i31) (local.get $b))) (i32.const 0))
          (then
            (return (ref.i31
              (i32.and (i31.get_s (ref.cast (ref i31) (local.get $b)))
                (call $int_low_i32 (local.get $a)))))))))
    (call $int_bitop (local.get $a) (local.get $b) (i32.const 0)))
  (func $int_low_i32 (param $v eqref) (result i32)
    (local $word i32)
    (if (ref.test (ref i31) (local.get $v))
      (then (return (i31.get_s (ref.cast (ref i31) (local.get $v))))))
    (local.set $word
      (array.get $limbs
        (struct.get $obj 6 (ref.cast (ref $obj) (local.get $v))) (i32.const 0)))
    (if (result i32) (i32.lt_s (call $int_sign (local.get $v)) (i32.const 0))
      (then (i32.sub (i32.const 0) (local.get $word)))
      (else (local.get $word))))
  (func $int_or (param $a eqref) (param $b eqref) (result eqref)
    (call $int_bitop (local.get $a) (local.get $b) (i32.const 1)))
  (func $int_xor (param $a eqref) (param $b eqref) (result eqref)
    (call $int_bitop (local.get $a) (local.get $b) (i32.const 2)))
  (func $int_not (param $a eqref) (result eqref)
    (if (ref.test (ref i31) (local.get $a))
      (then
        (return (ref.i31 (i32.xor (i31.get_s (ref.cast (ref i31) (local.get $a))) (i32.const -1))))))
    (call $int_sub (call $int_neg (local.get $a)) (ref.i31 (i32.const 1))))

  (func $int_shift_left_positive (param $v eqref) (param $count eqref) (result eqref)
    (local $m (ref $limbs)) (local $out (ref $limbs))
    (local $size i32) (local $word_shift i32) (local $bit_shift i64)
    (local $n i64) (local $amount i64) (local $i i32)
    (local $carry i64) (local $word i64)
    (if (i32.eqz (call $int_sign (local.get $v)))
      (then (return (ref.i31 (i32.const 0)))))
    (if (i32.eqz (call $int_sign (local.get $count)))
      (then (return (local.get $v))))
    ;; Every signed i31 value shifted by at most 33 fits a signed i64.
    ;; Restrict the count before using Wasm's masked shift instruction.
    (if (i32.and (ref.test (ref i31) (local.get $v)) (ref.test (ref i31) (local.get $count)))
      (then
        (if (i32.le_u (i31.get_u (ref.cast (ref i31) (local.get $count))) (i32.const 33))
          (then
            (return (call $int_from_i64
              (i64.shl
                (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $v))))
                (i64.extend_i32_u (i31.get_u (ref.cast (ref i31) (local.get $count)))))))))))
    ;; A GC array has a u32 length. Counts above its maximum bit capacity
    ;; cannot be represented regardless of the available physical memory.
    (if (i32.gt_s
          (call $int_compare (local.get $count) (call $int_from_i64 (i64.const 137438953440)))
          (i32.const 0))
      (then unreachable))
    (local.set $amount (call $int_to_i64 (local.get $count)))
    (local.set $m (call $int_limbs (local.get $v)))
    (local.set $size (array.len (local.get $m)))
    (local.set $word_shift (i32.wrap_i64 (i64.shr_u (local.get $amount) (i64.const 5))))
    (local.set $bit_shift (i64.and (local.get $amount) (i64.const 31)))
    (local.set $n
      (i64.add
        (i64.add (i64.extend_i32_u (local.get $size)) (i64.extend_i32_u (local.get $word_shift)))
        (i64.const 1)))
    (if (i64.gt_u (local.get $n) (i64.const 4294967295)) (then unreachable))
    (local.set $out (array.new_default $limbs (i32.wrap_i64 (local.get $n))))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $size)))
        (local.set $word
          (i64.or
            (i64.shl (i64.extend_i32_u (array.get $limbs (local.get $m) (local.get $i))) (local.get $bit_shift))
            (local.get $carry)))
        (array.set $limbs (local.get $out)
          (i32.add (local.get $word_shift) (local.get $i))
          (i32.wrap_i64 (local.get $word)))
        (local.set $carry (i64.shr_u (local.get $word) (i64.const 32)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (array.set $limbs (local.get $out)
      (i32.add (local.get $word_shift) (local.get $size))
      (i32.wrap_i64 (local.get $carry)))
    (call $int_pack (call $int_sign (local.get $v)) (local.get $out)))

  (func $int_shift_right_positive (param $v eqref) (param $count eqref) (result eqref)
    (local $m (ref $limbs)) (local $out (ref $limbs))
    (local $sign i32) (local $size i32) (local $word_shift i32) (local $bit_shift i32)
    (local $n i32) (local $amount i64) (local $i i32) (local $source i32)
    (local $discarded i32) (local $word i64) (local $result eqref)
    (local.set $sign (call $int_sign (local.get $v)))
    (if (i32.eqz (local.get $sign))
      (then (return (ref.i31 (i32.const 0)))))
    (if (i32.eqz (call $int_sign (local.get $count)))
      (then (return (local.get $v))))
    (if (ref.test (ref i31) (local.get $count))
      (then
        (local.set $word_shift (i31.get_u (ref.cast (ref i31) (local.get $count))))
        (if (ref.test (ref i31) (local.get $v))
          (then
            (if (i32.ge_u (local.get $word_shift) (i32.const 31))
              (then
                (return (ref.i31
                  (select (i32.const -1) (i32.const 0) (i32.lt_s (local.get $sign) (i32.const 0)))))))
            (return (ref.i31
              (i32.shr_s (i31.get_s (ref.cast (ref i31) (local.get $v))) (local.get $word_shift))))))
        ;; Positive 32-bit hashes and signed one-limb integers fit in i64.
        (if
          (i32.eq
            (array.len (struct.get $obj 6 (ref.cast (ref $obj) (local.get $v))))
            (i32.const 1))
          (then
            (if (i32.ge_u (local.get $word_shift) (i32.const 32))
              (then
                (return (ref.i31
                  (select (i32.const -1) (i32.const 0) (i32.lt_s (local.get $sign) (i32.const 0)))))))
            (return (call $int_from_i64
              (i64.shr_s (call $int_to_i64 (local.get $v))
                (i64.extend_i32_u (local.get $word_shift)))))))))
    (local.set $m (call $int_limbs (local.get $v)))
    (local.set $size (array.len (local.get $m)))
    (if (i32.ge_s
          (call $int_compare (local.get $count)
            (call $int_from_i64 (i64.shl (i64.extend_i32_u (local.get $size)) (i64.const 5))))
          (i32.const 0))
      (then
        (return (ref.i31 (select (i32.const -1) (i32.const 0) (i32.lt_s (local.get $sign) (i32.const 0)))))))
    (local.set $amount (call $int_to_i64 (local.get $count)))
    (local.set $word_shift (i32.wrap_i64 (i64.shr_u (local.get $amount) (i64.const 5))))
    (local.set $bit_shift (i32.wrap_i64 (i64.and (local.get $amount) (i64.const 31))))
    (local.set $n (i32.sub (local.get $size) (local.get $word_shift)))
    (local.set $out (array.new_default $limbs (local.get $n)))
    (block $discard_done
      (loop $discard_next
        (br_if $discard_done (i32.ge_u (local.get $i) (local.get $word_shift)))
        (local.set $discarded
          (i32.or (local.get $discarded) (array.get $limbs (local.get $m) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $discard_next)))
    (if (local.get $bit_shift)
      (then
        (local.set $discarded
          (i32.or (local.get $discarded)
            (i32.and
              (array.get $limbs (local.get $m) (local.get $word_shift))
              (i32.shr_u (i32.const -1) (i32.sub (i32.const 32) (local.get $bit_shift))))))))
    (local.set $i (i32.const 0))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $source (i32.add (local.get $word_shift) (local.get $i)))
        (local.set $word
          (i64.shr_u
            (i64.extend_i32_u (array.get $limbs (local.get $m) (local.get $source)))
            (i64.extend_i32_u (local.get $bit_shift))))
        (if (i32.and
              (i32.ne (local.get $bit_shift) (i32.const 0))
              (i32.lt_u (i32.add (local.get $source) (i32.const 1)) (local.get $size)))
          (then
            (local.set $word
              (i64.or (local.get $word)
                (i64.shl
                  (i64.extend_i32_u (array.get $limbs (local.get $m) (i32.add (local.get $source) (i32.const 1))))
                  (i64.extend_i32_u (i32.sub (i32.const 32) (local.get $bit_shift))))))))
        (array.set $limbs (local.get $out) (local.get $i) (i32.wrap_i64 (local.get $word)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (local.set $result (call $int_pack (local.get $sign) (local.get $out)))
    (if (result eqref)
      (i32.and (i32.lt_s (local.get $sign) (i32.const 0)) (i32.ne (local.get $discarded) (i32.const 0)))
      (then (call $int_sub (local.get $result) (ref.i31 (i32.const 1))))
      (else (local.get $result))))

  (func $int_shl (param $v eqref) (param $count eqref) (result eqref)
    (if (result eqref) (i32.lt_s (call $int_sign (local.get $count)) (i32.const 0))
      (then (call $int_shift_right_positive (local.get $v) (call $int_neg (local.get $count))))
      (else (call $int_shift_left_positive (local.get $v) (local.get $count)))))
  (func $int_shr (param $v eqref) (param $count eqref) (result eqref)
    (if (result eqref) (i32.lt_s (call $int_sign (local.get $count)) (i32.const 0))
      (then (call $int_shift_left_positive (local.get $v) (call $int_neg (local.get $count))))
      (else (call $int_shift_right_positive (local.get $v) (local.get $count)))))

  ;; Public standard-library intrinsics preserve the boxed internal ABI.
  (func $int_bitwise_and (param $a eqref) (param $b eqref) (result eqref)
    (call $int_and (local.get $a) (local.get $b)))
  (func $int_bitwise_or (param $a eqref) (param $b eqref) (result eqref)
    (call $int_or (local.get $a) (local.get $b)))
  (func $int_bitwise_xor (param $a eqref) (param $b eqref) (result eqref)
    (call $int_xor (local.get $a) (local.get $b)))
  (func $int_bitwise_not (param $a eqref) (result eqref)
    (call $int_not (local.get $a)))
  (func $int_bitwise_shift_left (param $v eqref) (param $count eqref) (result eqref)
    (call $int_shl (local.get $v) (local.get $count)))
  (func $int_bitwise_shift_right (param $v eqref) (param $count eqref) (result eqref)
    (call $int_shr (local.get $v) (local.get $count)))
"#
    .to_string()
}
