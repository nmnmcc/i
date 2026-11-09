//! Bit strings use a GC byte array and a bit-granular view. All indexing is
//! relative to the view; byte order is a property of a segment, not the host.
//!
//! Pattern code must use `bits_has`, `bits_float_finite`, or `bits_read_utf`
//! before extracting a potentially invalid segment. Failed UTF decoding is a
//! normal match failure, whereas invalid construction stops the program with
//! one of the `bit array:` reasons of spec §11.7.

pub fn wat() -> String {
    WAT.to_owned()
}

const WAT: &str = r#"
  (func $bits_new (param $data (ref $bytes)) (param $offset i32) (param $length i64) (result eqref)
    (struct.new $obj
      (i32.const 9) (local.get $offset) (local.get $length) (f64.const 0)
      (local.get $data) (ref.null $values) (ref.null $limbs) (ref.null $call)))

  (func $bits_data (param $value eqref) (result (ref $bytes))
    (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $value)))))

  (func $bits_len64 (param $value eqref) (result i64)
    (struct.get $obj 2 (ref.cast (ref $obj) (local.get $value))))

  (func $bits_offset (param $value eqref) (result i64)
    (i64.extend_i32_u (struct.get $obj 1 (ref.cast (ref $obj) (local.get $value)))))

  (func $bits_has (param $value eqref) (param $start i64) (param $length i64) (result i32)
    (local $total i64)
    (local.set $total (call $bits_len64 (local.get $value)))
    (if (i64.lt_s (local.get $start) (i64.const 0)) (then (return (i32.const 0))))
    (if (i64.lt_s (local.get $length) (i64.const 0)) (then (return (i32.const 0))))
    (if (i64.gt_u (local.get $start) (local.get $total)) (then (return (i32.const 0))))
    (i64.le_u (local.get $length) (i64.sub (local.get $total) (local.get $start))))

  (func $bits_alloc (param $length i64) (result (ref $bytes))
    (local $bytes i64)
    (if (i64.lt_s (local.get $length) (i64.const 0)) (then unreachable))
    (local.set $bytes (i64.div_u (local.get $length) (i64.const 8)))
    (if (i64.ne (i64.and (local.get $length) (i64.const 7)) (i64.const 0))
      (then (local.set $bytes (i64.add (local.get $bytes) (i64.const 1)))))
    (if (i64.gt_u (local.get $bytes) (i64.const 4294967295)) (then unreachable))
    (array.new_default $bytes (i32.wrap_i64 (local.get $bytes))))

  (func $bits_empty (result eqref)
    (call $bits_new (array.new_default $bytes (i32.const 0)) (i32.const 0) (i64.const 0)))

  (func $bits_size (param $value eqref) (result eqref)
    (call $int_from_i64 (call $bits_len64 (local.get $value))))

  ;; A negative sentinel makes invalid dynamic pattern sizes ordinary match
  ;; failures (spec §8.4). Construction uses $bits_length instead, which stops
  ;; for a negative size. clamp_negative is kept for older callers.
  (func $bits_width (param $size eqref) (param $unit i64) (param $clamp_negative i32) (result i64)
    (local $limbs (ref null $limbs)) (local $count i32) (local $value i64)
    (if (i32.lt_s (call $int_sign (local.get $size)) (i32.const 0))
      (then (return (if (result i64) (local.get $clamp_negative)
        (then (i64.const 0)) (else (i64.const -1))))))
    (if (i64.le_s (local.get $unit) (i64.const 0))
      (then (return (i64.const -1))))
    (if (ref.test (ref i31) (local.get $size))
      (then (local.set $value (i64.extend_i32_s (i31.get_s (ref.cast (ref i31) (local.get $size))))))
      (else
        (local.set $limbs (call $int_limbs (local.get $size)))
        (local.set $count (array.len (local.get $limbs)))
        (if (i32.gt_u (local.get $count) (i32.const 2))
          (then (return (i64.const -1))))
        (if (i32.eq (local.get $count) (i32.const 2)) (then
          (if (i32.lt_s (array.get $limbs (local.get $limbs) (i32.const 1)) (i32.const 0))
            (then (return (i64.const -1))))))
        (local.set $value (call $int_to_i64 (local.get $size)))))
    (if (i64.gt_u (local.get $value) (i64.div_u (i64.const 9223372036854775807) (local.get $unit)))
      (then (return (i64.const -1))))
    (i64.mul (local.get $value) (local.get $unit)))

  (func $bits_get (param $value eqref) (param $index i64) (result i32)
    (local $absolute i64)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $index) (i64.const 1)))
      (then unreachable))
    (local.set $absolute (i64.add (call $bits_offset (local.get $value)) (local.get $index)))
    (i32.and
      (i32.shr_u
        (array.get_u $bytes (call $bits_data (local.get $value))
          (i32.wrap_i64 (i64.shr_u (local.get $absolute) (i64.const 3))))
        (i32.sub (i32.const 7) (i32.wrap_i64 (i64.and (local.get $absolute) (i64.const 7)))))
      (i32.const 1)))

  (func $bits_set (param $data (ref $bytes)) (param $index i64) (param $value i32)
    (local $byte i32) (local $shift i32) (local $old i32)
    (local.set $byte (i32.wrap_i64 (i64.shr_u (local.get $index) (i64.const 3))))
    (local.set $shift (i32.sub (i32.const 7) (i32.wrap_i64 (i64.and (local.get $index) (i64.const 7)))))
    (local.set $old (array.get_u $bytes (local.get $data) (local.get $byte)))
    (array.set $bytes (local.get $data) (local.get $byte)
      (i32.or
        (i32.and (local.get $old) (i32.xor (i32.shl (i32.const 1) (local.get $shift)) (i32.const -1)))
        (i32.shl (i32.and (local.get $value) (i32.const 1)) (local.get $shift)))))

  (func $bits_slice (param $value eqref) (param $start i64) (param $length i64) (result eqref)
    (local $absolute i64) (local $data (ref null $bytes)) (local $i i64)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (local.get $length)))
      (then unreachable))
    (local.set $absolute (i64.add (call $bits_offset (local.get $value)) (local.get $start)))
    (if (i64.le_u (local.get $absolute) (i64.const 4294967295))
      (then (return (call $bits_new (call $bits_data (local.get $value))
        (i32.wrap_i64 (local.get $absolute)) (local.get $length)))))
    ;; Very large backing arrays can have a view offset too large for the
    ;; object's u32 field. Normalize such a view rather than truncating it.
    (local.set $data (call $bits_alloc (local.get $length)))
    (block $done (loop $copy
      (br_if $done (i64.ge_u (local.get $i) (local.get $length)))
      (call $bits_set (ref.as_non_null (local.get $data)) (local.get $i)
        (call $bits_get (local.get $value) (i64.add (local.get $start) (local.get $i))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $copy)))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $length)))

  (func $bits_concat (param $left eqref) (param $right eqref) (result eqref)
    (local $a i64) (local $b i64) (local $length i64)
    (local $data (ref null $bytes)) (local $i i64)
    (local.set $a (call $bits_len64 (local.get $left)))
    (local.set $b (call $bits_len64 (local.get $right)))
    (if (i64.eqz (local.get $a)) (then (return (local.get $right))))
    (if (i64.eqz (local.get $b)) (then (return (local.get $left))))
    (local.set $length (i64.add (local.get $a) (local.get $b)))
    (if (i64.lt_u (local.get $length) (local.get $a)) (then unreachable))
    (local.set $data (call $bits_alloc (local.get $length)))
    (if (i64.eqz (i64.and
      (i64.or (i64.or (local.get $a) (local.get $b))
        (i64.or (call $bits_offset (local.get $left)) (call $bits_offset (local.get $right))))
      (i64.const 7)))
      (then
        (array.copy $bytes $bytes (local.get $data) (i32.const 0)
          (call $bits_data (local.get $left))
          (i32.wrap_i64 (i64.shr_u (call $bits_offset (local.get $left)) (i64.const 3)))
          (i32.wrap_i64 (i64.shr_u (local.get $a) (i64.const 3))))
        (array.copy $bytes $bytes (local.get $data)
          (i32.wrap_i64 (i64.shr_u (local.get $a) (i64.const 3)))
          (call $bits_data (local.get $right))
          (i32.wrap_i64 (i64.shr_u (call $bits_offset (local.get $right)) (i64.const 3)))
          (i32.wrap_i64 (i64.shr_u (local.get $b) (i64.const 3)))))
      (else
        (block $left_done (loop $copy_left
          (br_if $left_done (i64.ge_u (local.get $i) (local.get $a)))
          (call $bits_set (ref.as_non_null (local.get $data)) (local.get $i)
            (call $bits_get (local.get $left) (local.get $i)))
          (local.set $i (i64.add (local.get $i) (i64.const 1)))
          (br $copy_left)))
        (local.set $i (i64.const 0))
        (block $right_done (loop $copy_right
          (br_if $right_done (i64.ge_u (local.get $i) (local.get $b)))
          (call $bits_set (ref.as_non_null (local.get $data)) (i64.add (local.get $a) (local.get $i))
            (call $bits_get (local.get $right) (local.get $i)))
          (local.set $i (i64.add (local.get $i) (i64.const 1)))
          (br $copy_right)))))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $length)))

  (func $bits_match_equal (param $left eqref) (param $right eqref) (result i32)
    (local $length i64) (local $i i64) (local $count i64)
    (local $left_byte i32) (local $right_byte i32)
    (local $left_data (ref null $bytes)) (local $right_data (ref null $bytes))
    (local.set $length (call $bits_len64 (local.get $left)))
    (if (i64.ne (local.get $length) (call $bits_len64 (local.get $right)))
      (then (return (i32.const 0))))
    (if (i64.eqz (i64.and
      (i64.or (call $bits_offset (local.get $left)) (call $bits_offset (local.get $right)))
      (i64.const 7))) (then
      (local.set $left_data (call $bits_data (local.get $left)))
      (local.set $right_data (call $bits_data (local.get $right)))
      (local.set $left_byte (i32.wrap_i64 (i64.shr_u (call $bits_offset (local.get $left)) (i64.const 3))))
      (local.set $right_byte (i32.wrap_i64 (i64.shr_u (call $bits_offset (local.get $right)) (i64.const 3))))
      (local.set $count (i64.shr_u (local.get $length) (i64.const 3)))
      (block $bytes_done (loop $compare_bytes
        (br_if $bytes_done (i64.ge_u (local.get $i) (local.get $count)))
        (if (i32.ne
          (array.get_u $bytes (local.get $left_data) (i32.add (local.get $left_byte) (i32.wrap_i64 (local.get $i))))
          (array.get_u $bytes (local.get $right_data) (i32.add (local.get $right_byte) (i32.wrap_i64 (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i64.add (local.get $i) (i64.const 1)))
        (br $compare_bytes)))
      (local.set $i (i64.shl (local.get $i) (i64.const 3)))))
    (block $done (loop $compare
      (br_if $done (i64.ge_u (local.get $i) (local.get $length)))
      (if (i32.ne (call $bits_get (local.get $left) (local.get $i))
                   (call $bits_get (local.get $right) (local.get $i)))
        (then (return (i32.const 0))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $compare)))
    (i32.const 1))

  ;; Which integer bit is stored at a segment's stream position? Little
  ;; endian reverses bytes, retaining MSB-first order in each (partial) byte.
  (func $bits_value_position (param $index i64) (param $width i64) (param $little i32) (result i64)
    (local $base i64) (local $group i64)
    (if (i32.eqz (local.get $little))
      (then (return (i64.sub (i64.sub (local.get $width) (i64.const 1)) (local.get $index)))))
    (local.set $base (i64.and (local.get $index) (i64.const -8)))
    (local.set $group (i64.sub (local.get $width) (local.get $base)))
    (if (i64.gt_u (local.get $group) (i64.const 8)) (then (local.set $group (i64.const 8))))
    (i64.sub (i64.sub (i64.add (local.get $base) (local.get $group)) (i64.const 1))
      (i64.sub (local.get $index) (local.get $base))))

  (func $bits_magnitude_bit (param $limbs (ref $limbs)) (param $position i64) (result i32)
    (local $limb i64)
    (local.set $limb (i64.shr_u (local.get $position) (i64.const 5)))
    (if (i64.ge_u (local.get $limb) (i64.extend_i32_u (array.len (local.get $limbs))))
      (then (return (i32.const 0))))
    (i32.and
      (i32.shr_u (array.get $limbs (local.get $limbs) (i32.wrap_i64 (local.get $limb)))
        (i32.wrap_i64 (i64.and (local.get $position) (i64.const 31))))
      (i32.const 1)))

  (func $bits_from_int (param $value eqref) (param $width i64) (param $little i32) (result eqref)
    (local $data (ref null $bytes)) (local $limbs (ref null $limbs))
    (local $negative i32) (local $trail i64) (local $limb i32)
    (local $i i64) (local $position i64) (local $bit i32)
    (local.set $data (call $bits_alloc (local.get $width)))
    (local.set $limbs (call $int_limbs (local.get $value)))
    (local.set $negative (i32.lt_s (call $int_sign (local.get $value)) (i32.const 0)))
    (if (local.get $negative) (then
      (block $found (loop $find
        (if (i32.ne (array.get $limbs (local.get $limbs) (local.get $limb)) (i32.const 0))
          (then (br $found)))
        (local.set $limb (i32.add (local.get $limb) (i32.const 1)))
        (br $find)))
      (local.set $trail
        (i64.add (i64.mul (i64.extend_i32_u (local.get $limb)) (i64.const 32))
          (i64.extend_i32_u (i32.ctz (array.get $limbs (local.get $limbs) (local.get $limb))))))))
    (block $done (loop $write
      (br_if $done (i64.ge_u (local.get $i) (local.get $width)))
      (local.set $position (call $bits_value_position (local.get $i) (local.get $width) (local.get $little)))
      (local.set $bit (call $bits_magnitude_bit (ref.as_non_null (local.get $limbs)) (local.get $position)))
      (if (local.get $negative) (then
        (if (i64.gt_u (local.get $position) (local.get $trail))
          (then (local.set $bit (i32.xor (local.get $bit) (i32.const 1)))))))
      (call $bits_set (ref.as_non_null (local.get $data)) (local.get $i) (local.get $bit))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $write)))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $width)))

  (func $bits_read_int (param $value eqref) (param $start i64) (param $width i64)
      (param $signed i32) (param $little i32) (result eqref)
    (local $limbs (ref null $limbs)) (local $count i64) (local $index i32)
    (local $i i64) (local $position i64) (local $negative i32)
    (local $carry i64) (local $word i64) (local $remainder i32)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (local.get $width)))
      (then unreachable))
    (local.set $count (i64.div_u (i64.add (local.get $width) (i64.const 31)) (i64.const 32)))
    (if (i64.gt_u (local.get $count) (i64.const 4294967295)) (then unreachable))
    (local.set $limbs (array.new_default $limbs (i32.wrap_i64 (local.get $count))))
    (block $done (loop $read
      (br_if $done (i64.ge_u (local.get $i) (local.get $width)))
      (local.set $position (call $bits_value_position (local.get $i) (local.get $width) (local.get $little)))
      (if (call $bits_get (local.get $value) (i64.add (local.get $start) (local.get $i))) (then
        (local.set $index (i32.wrap_i64 (i64.shr_u (local.get $position) (i64.const 5))))
        (array.set $limbs (local.get $limbs) (local.get $index)
          (i32.or (array.get $limbs (local.get $limbs) (local.get $index))
            (i32.shl (i32.const 1) (i32.wrap_i64 (i64.and (local.get $position) (i64.const 31))))))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $read)))
    (if (i32.and (local.get $signed) (i64.ne (local.get $width) (i64.const 0)))
      (then (local.set $negative (call $bits_magnitude_bit (ref.as_non_null (local.get $limbs))
        (i64.sub (local.get $width) (i64.const 1))))))
    (if (local.get $negative) (then
      (local.set $index (i32.const 0))
      (local.set $carry (i64.const 1))
      (local.set $remainder (i32.wrap_i64 (i64.and (local.get $width) (i64.const 31))))
      (block $neg_done (loop $negate
        (br_if $neg_done (i32.ge_u (local.get $index) (array.len (local.get $limbs))))
        (local.set $word (i64.extend_i32_u
          (i32.xor (array.get $limbs (local.get $limbs) (local.get $index)) (i32.const -1))))
        (if (i32.and
          (i32.eq (local.get $index) (i32.sub (array.len (local.get $limbs)) (i32.const 1)))
          (i32.ne (local.get $remainder) (i32.const 0)))
          (then (local.set $word (i64.and (local.get $word)
            (i64.sub (i64.shl (i64.const 1) (i64.extend_i32_u (local.get $remainder))) (i64.const 1))))))
        (local.set $word (i64.add (local.get $word) (local.get $carry)))
        (array.set $limbs (local.get $limbs) (local.get $index) (i32.wrap_i64 (local.get $word)))
        (local.set $carry (i64.shr_u (local.get $word) (i64.const 32)))
        (local.set $index (i32.add (local.get $index) (i32.const 1)))
        (br $negate)))))
    (call $int_pack (if (result i32) (local.get $negative) (then (i32.const -1)) (else (i32.const 1)))
      (ref.as_non_null (local.get $limbs))))

  (func $bits_from_u64 (param $value i64) (param $width i64) (param $little i32) (result eqref)
    (local $data (ref null $bytes)) (local $i i64) (local $position i64)
    (if (i64.gt_u (local.get $width) (i64.const 64)) (then unreachable))
    (local.set $data (call $bits_alloc (local.get $width)))
    (block $done (loop $write
      (br_if $done (i64.ge_u (local.get $i) (local.get $width)))
      (local.set $position (call $bits_value_position (local.get $i) (local.get $width) (local.get $little)))
      (call $bits_set (ref.as_non_null (local.get $data)) (local.get $i)
        (i32.wrap_i64 (i64.and (i64.shr_u (local.get $value) (local.get $position)) (i64.const 1))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $write)))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $width)))

  (func $bits_read_u64 (param $value eqref) (param $start i64) (param $width i64)
      (param $little i32) (result i64)
    (local $result i64) (local $i i64) (local $position i64)
    (if (i32.or (i64.gt_u (local.get $width) (i64.const 64))
      (i32.eqz (call $bits_has (local.get $value) (local.get $start) (local.get $width))))
      (then unreachable))
    (block $done (loop $read
      (br_if $done (i64.ge_u (local.get $i) (local.get $width)))
      (local.set $position (call $bits_value_position (local.get $i) (local.get $width) (local.get $little)))
      (local.set $result (i64.or (local.get $result)
        (i64.shl (i64.extend_i32_u (call $bits_get (local.get $value)
          (i64.add (local.get $start) (local.get $i)))) (local.get $position))))
      (local.set $i (i64.add (local.get $i) (i64.const 1)))
      (br $read)))
    (local.get $result))

  (func $bits_round_shift (param $value i64) (param $shift i64) (result i64)
    (local $truncated i64) (local $remainder i64) (local $half i64)
    (local.set $truncated (i64.shr_u (local.get $value) (local.get $shift)))
    (local.set $remainder (i64.and (local.get $value)
      (i64.sub (i64.shl (i64.const 1) (local.get $shift)) (i64.const 1))))
    (local.set $half (i64.shl (i64.const 1) (i64.sub (local.get $shift) (i64.const 1))))
    (if (i32.or (i64.gt_u (local.get $remainder) (local.get $half))
      (i32.and (i64.eq (local.get $remainder) (local.get $half))
        (i64.ne (i64.and (local.get $truncated) (i64.const 1)) (i64.const 0))))
      (then (local.set $truncated (i64.add (local.get $truncated) (i64.const 1)))))
    (local.get $truncated))

  ;; Direct f64-to-half rounding avoids the double rounding of f64 -> f32 -> f16.
  (func $bits_half_encode (param $value f64) (result i64)
    (local $raw i64) (local $sign i64) (local $exponent i32)
    (local $significand i64) (local $rounded i64)
    (local.set $raw (i64.reinterpret_f64 (local.get $value)))
    (local.set $sign (i64.and (i64.shr_u (local.get $raw) (i64.const 48)) (i64.const 32768)))
    (local.set $exponent (i32.sub
      (i32.wrap_i64 (i64.and (i64.shr_u (local.get $raw) (i64.const 52)) (i64.const 2047)))
      (i32.const 1023)))
    (if (i32.gt_s (local.get $exponent) (i32.const 15))
      (then (return (i64.or (local.get $sign) (i64.const 31744)))))
    (if (i32.lt_s (local.get $exponent) (i32.const -25))
      (then (return (local.get $sign))))
    (local.set $significand (i64.or (i64.and (local.get $raw) (i64.const 4503599627370495))
      (i64.const 4503599627370496)))
    (if (i32.lt_s (local.get $exponent) (i32.const -14)) (then
      (return (i64.or (local.get $sign) (call $bits_round_shift (local.get $significand)
        (i64.extend_i32_u (i32.sub (i32.const 28) (local.get $exponent))))))))
    (local.set $rounded (call $bits_round_shift (local.get $significand) (i64.const 42)))
    (if (i64.eq (local.get $rounded) (i64.const 2048)) (then
      (local.set $rounded (i64.const 1024))
      (local.set $exponent (i32.add (local.get $exponent) (i32.const 1)))))
    (if (i32.gt_s (local.get $exponent) (i32.const 15))
      (then (return (i64.or (local.get $sign) (i64.const 31744)))))
    (i64.or (local.get $sign)
      (i64.or (i64.shl (i64.extend_i32_u (i32.add (local.get $exponent) (i32.const 15))) (i64.const 10))
        (i64.and (local.get $rounded) (i64.const 1023)))))

  (func $bits_half_decode (param $raw i64) (result f64)
    (local $exponent i64) (local $fraction i64) (local $sign i64)
    (local.set $exponent (i64.and (i64.shr_u (local.get $raw) (i64.const 10)) (i64.const 31)))
    (local.set $fraction (i64.and (local.get $raw) (i64.const 1023)))
    (local.set $sign (i64.shl (i64.and (local.get $raw) (i64.const 32768)) (i64.const 48)))
    (if (i64.eqz (local.get $exponent)) (then
      (return (f64.copysign
        (f64.mul (f64.convert_i64_u (local.get $fraction)) (f64.const 0x1p-24))
        (f64.reinterpret_i64 (local.get $sign))))))
    (if (i64.eq (local.get $exponent) (i64.const 31)) (then
      (return (f64.reinterpret_i64 (i64.or (local.get $sign)
        (i64.or (i64.const 9218868437227405312) (i64.shl (local.get $fraction) (i64.const 42))))))))
    (f64.reinterpret_i64 (i64.or (local.get $sign)
      (i64.or (i64.shl (i64.add (local.get $exponent) (i64.const 1008)) (i64.const 52))
        (i64.shl (local.get $fraction) (i64.const 42))))))

  (func $bits_from_float (param $value eqref) (param $width i64) (param $little i32) (result eqref)
    (local $number f64) (local $raw i64)
    (local.set $number (call $float_get (local.get $value)))
    (if (i64.eq (local.get $width) (i64.const 16))
      (then (local.set $raw (call $bits_half_encode (local.get $number))))
      (else (if (i64.eq (local.get $width) (i64.const 32))
        (then (local.set $raw (i64.extend_i32_u (i32.reinterpret_f32 (f32.demote_f64 (local.get $number))))))
        (else (if (i64.eq (local.get $width) (i64.const 64))
          (then (local.set $raw (i64.reinterpret_f64 (local.get $number))))
          (else unreachable))))))
    ;; Rounding to a smaller format may overflow to infinity (spec §8.3).
    (if (i32.or
          (i32.and (i64.eq (local.get $width) (i64.const 16))
            (i64.eq (i64.and (local.get $raw) (i64.const 31744)) (i64.const 31744)))
          (i32.and (i64.eq (local.get $width) (i64.const 32))
            (i64.eq (i64.and (local.get $raw) (i64.const 2139095040)) (i64.const 2139095040))))
      (then (call $stop_bits_float) unreachable))
    (call $bits_from_u64 (local.get $raw) (local.get $width) (local.get $little)))

  ;; The length of a segment `u(n)`, `bits(n)` or `bytes(n)` in bits, for
  ;; construction: n times unit. A negative n stops (spec §8.3); a length
  ;; that no bit array can have traps as out of memory.
  (func $bits_length (param $size eqref) (param $unit i64) (result i64)
    (if (i32.lt_s (call $int_sign (local.get $size)) (i32.const 0))
      (then (call $stop_bits_negative) unreachable))
    (if (i32.gt_s (call $int_compare (local.get $size)
          (call $int_from_i64 (i64.div_u (i64.const 34359738360) (local.get $unit)))) (i32.const 0))
      (then unreachable))
    (i64.mul (call $int_to_i64 (local.get $size)) (local.get $unit)))

  ;; The length of a `bits(n)` or `bytes(n)` segment in bits: n times unit.
  ;; A negative n stops (spec §8.3). A length that no bit array can have is
  ;; 2^62, so that `$bits_take` stops because the value is too short.
  (func $bits_take_length (param $size eqref) (param $unit i64) (result i64)
    (if (i32.lt_s (call $int_sign (local.get $size)) (i32.const 0))
      (then (call $stop_bits_negative) unreachable))
    (if (i32.gt_s (call $int_compare (local.get $size)
          (call $int_from_i64 (i64.div_u (i64.const 4611686018427387904) (local.get $unit)))) (i32.const 0))
      (then (return (i64.const 4611686018427387904))))
    (i64.mul (call $int_to_i64 (local.get $size)) (local.get $unit)))

  ;; The first `length` bits of a `bits(n)` or `bytes(n)` segment value.
  (func $bits_take (param $value eqref) (param $length i64) (result eqref)
    (if (i64.lt_u (call $bits_len64 (local.get $value)) (local.get $length))
      (then (call $stop_bits_short) unreachable))
    (call $bits_slice (local.get $value) (i64.const 0) (local.get $length)))

  ;; The value of a `bytes` segment, which must be whole bytes.
  (func $bits_whole (param $value eqref) (result eqref)
    (if (i64.ne (i64.and (call $bits_len64 (local.get $value)) (i64.const 7)) (i64.const 0))
      (then (call $stop_bits_not_whole) unreachable))
    (local.get $value))

  (func $bits_float_finite (param $value eqref) (param $start i64) (param $width i64)
      (param $little i32) (result i32)
    (local $raw i64)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (local.get $width)))
      (then (return (i32.const 0))))
    (if (i32.eqz (i32.or (i64.eq (local.get $width) (i64.const 16))
      (i32.or (i64.eq (local.get $width) (i64.const 32)) (i64.eq (local.get $width) (i64.const 64)))))
      (then (return (i32.const 0))))
    (local.set $raw (call $bits_read_u64 (local.get $value) (local.get $start) (local.get $width) (local.get $little)))
    (if (i64.eq (local.get $width) (i64.const 16))
      (then (return (i64.ne (i64.and (local.get $raw) (i64.const 31744)) (i64.const 31744)))))
    (if (i64.eq (local.get $width) (i64.const 32))
      (then (return (i64.ne (i64.and (local.get $raw) (i64.const 2139095040)) (i64.const 2139095040)))))
    (i64.ne (i64.and (local.get $raw) (i64.const 9218868437227405312)) (i64.const 9218868437227405312)))

  (func $bits_read_float (param $value eqref) (param $start i64) (param $width i64)
      (param $little i32) (result eqref)
    (local $raw i64)
    (if (i32.eqz (call $bits_float_finite (local.get $value) (local.get $start) (local.get $width) (local.get $little)))
      (then unreachable))
    (local.set $raw (call $bits_read_u64 (local.get $value) (local.get $start) (local.get $width) (local.get $little)))
    (if (i64.eq (local.get $width) (i64.const 16))
      (then (return (call $float_new (call $bits_half_decode (local.get $raw))))))
    (if (i64.eq (local.get $width) (i64.const 32))
      (then (return (call $float_new (f64.promote_f32 (f32.reinterpret_i32 (i32.wrap_i64 (local.get $raw))))))))
    (call $float_new (f64.reinterpret_i64 (local.get $raw))))

  (func $bits_from_string (param $value eqref) (result eqref)
    (local $data (ref null $bytes))
    (local.set $data (struct.get $obj 4 (ref.cast (ref $obj) (local.get $value))))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0)
      (i64.mul (i64.extend_i32_u (array.len (local.get $data))) (i64.const 8))))

  (func $bits_byte (param $value eqref) (param $start i64) (result i32)
    (local $absolute i64)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (i64.const 8)))
      (then unreachable))
    (local.set $absolute (i64.add (call $bits_offset (local.get $value)) (local.get $start)))
    (if (i64.eqz (i64.and (local.get $absolute) (i64.const 7))) (then
      (return (array.get_u $bytes (call $bits_data (local.get $value))
        (i32.wrap_i64 (i64.shr_u (local.get $absolute) (i64.const 3)))))))
    (i32.wrap_i64 (call $bits_read_u64 (local.get $value) (local.get $start) (i64.const 8) (i32.const 0))))

  (func $bits_utf8_size (param $value eqref) (param $start i64) (result i32)
    (local $first i32) (local $second i32) (local $byte i32) (local $count i32) (local $i i32)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (i64.const 8)))
      (then (return (i32.const 0))))
    (local.set $first (call $bits_byte (local.get $value) (local.get $start)))
    (if (i32.lt_u (local.get $first) (i32.const 128)) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $first) (i32.const 194)) (i32.le_u (local.get $first) (i32.const 223)))
      (then (local.set $count (i32.const 2)))
      (else (if (i32.and (i32.ge_u (local.get $first) (i32.const 224)) (i32.le_u (local.get $first) (i32.const 239)))
        (then (local.set $count (i32.const 3)))
        (else (if (i32.and (i32.ge_u (local.get $first) (i32.const 240)) (i32.le_u (local.get $first) (i32.const 244)))
          (then (local.set $count (i32.const 4)))
          (else (return (i32.const 0))))))))
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start)
      (i64.mul (i64.extend_i32_u (local.get $count)) (i64.const 8))))
      (then (return (i32.const 0))))
    (local.set $second (call $bits_byte (local.get $value) (i64.add (local.get $start) (i64.const 8))))
    (if (i32.or
      (i32.and (i32.eq (local.get $first) (i32.const 224)) (i32.lt_u (local.get $second) (i32.const 160)))
      (i32.or
        (i32.and (i32.eq (local.get $first) (i32.const 237)) (i32.ge_u (local.get $second) (i32.const 160)))
        (i32.or
          (i32.and (i32.eq (local.get $first) (i32.const 240)) (i32.lt_u (local.get $second) (i32.const 144)))
          (i32.and (i32.eq (local.get $first) (i32.const 244)) (i32.ge_u (local.get $second) (i32.const 144))))))
      (then (return (i32.const 0))))
    (local.set $i (i32.const 1))
    (block $done (loop $continuations
      (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
      (local.set $byte (call $bits_byte (local.get $value)
        (i64.add (local.get $start) (i64.mul (i64.extend_i32_u (local.get $i)) (i64.const 8)))))
      (if (i32.ne (i32.and (local.get $byte) (i32.const 192)) (i32.const 128))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $continuations)))
    (local.get $count))

  (func $bits_codepoint_new (param $codepoint i32) (result eqref)
    (call $codepoint_new (local.get $codepoint)))

  (func $bits_codepoint_get (param $value eqref) (result i32)
    (struct.get $obj 1 (ref.cast (ref $obj) (local.get $value))))

  (func $bits_read_utf8 (param $value eqref) (param $start i64) (result eqref)
    (local $count i32) (local $codepoint i32) (local $i i32) (local $mask i32)
    (local.set $count (call $bits_utf8_size (local.get $value) (local.get $start)))
    (if (i32.eqz (local.get $count)) (then unreachable))
    (local.set $mask (if (result i32) (i32.eq (local.get $count) (i32.const 1))
      (then (i32.const 127))
      (else (i32.sub (i32.shl (i32.const 1) (i32.sub (i32.const 7) (local.get $count))) (i32.const 1)))))
    (local.set $codepoint (i32.and (call $bits_byte (local.get $value) (local.get $start)) (local.get $mask)))
    (local.set $i (i32.const 1))
    (block $done (loop $read
      (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
      (local.set $codepoint (i32.or (i32.shl (local.get $codepoint) (i32.const 6))
        (i32.and (call $bits_byte (local.get $value)
          (i64.add (local.get $start) (i64.mul (i64.extend_i32_u (local.get $i)) (i64.const 8)))) (i32.const 63))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $read)))
    (call $bits_codepoint_new (local.get $codepoint)))

  (func $bits_utf16_size (param $value eqref) (param $start i64) (param $little i32) (result i32)
    (local $first i32) (local $second i32)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (i64.const 16)))
      (then (return (i32.const 0))))
    (local.set $first (i32.wrap_i64 (call $bits_read_u64 (local.get $value) (local.get $start)
      (i64.const 16) (local.get $little))))
    (if (i32.and (i32.ge_u (local.get $first) (i32.const 55296)) (i32.le_u (local.get $first) (i32.const 56319)))
      (then
        (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (i64.const 32)))
          (then (return (i32.const 0))))
        (local.set $second (i32.wrap_i64 (call $bits_read_u64 (local.get $value)
          (i64.add (local.get $start) (i64.const 16)) (i64.const 16) (local.get $little))))
        (if (i32.and (i32.ge_u (local.get $second) (i32.const 56320)) (i32.le_u (local.get $second) (i32.const 57343)))
          (then (return (i32.const 4))) (else (return (i32.const 0))))))
    (if (i32.and (i32.ge_u (local.get $first) (i32.const 56320)) (i32.le_u (local.get $first) (i32.const 57343)))
      (then (return (i32.const 0))))
    (i32.const 2))

  (func $bits_read_utf16 (param $value eqref) (param $start i64) (param $little i32) (result eqref)
    (local $count i32) (local $codepoint i32) (local $second i32)
    (local.set $count (call $bits_utf16_size (local.get $value) (local.get $start) (local.get $little)))
    (if (i32.eqz (local.get $count)) (then unreachable))
    (local.set $codepoint (i32.wrap_i64 (call $bits_read_u64 (local.get $value) (local.get $start)
      (i64.const 16) (local.get $little))))
    (if (i32.eq (local.get $count) (i32.const 4)) (then
      (local.set $second (i32.wrap_i64 (call $bits_read_u64 (local.get $value)
        (i64.add (local.get $start) (i64.const 16)) (i64.const 16) (local.get $little))))
      (local.set $codepoint (i32.add (i32.const 65536)
        (i32.or (i32.shl (i32.sub (local.get $codepoint) (i32.const 55296)) (i32.const 10))
          (i32.sub (local.get $second) (i32.const 56320)))))))
    (call $bits_codepoint_new (local.get $codepoint)))

  (func $bits_valid_codepoint (param $codepoint i32) (result i32)
    (i32.and (i32.le_u (local.get $codepoint) (i32.const 1114111))
      (i32.eqz (i32.and (i32.ge_u (local.get $codepoint) (i32.const 55296))
        (i32.le_u (local.get $codepoint) (i32.const 57343))))))

  (func $bits_utf32_size (param $value eqref) (param $start i64) (param $little i32) (result i32)
    (if (i32.eqz (call $bits_has (local.get $value) (local.get $start) (i64.const 32)))
      (then (return (i32.const 0))))
    (if (result i32) (call $bits_valid_codepoint
      (i32.wrap_i64 (call $bits_read_u64 (local.get $value) (local.get $start) (i64.const 32) (local.get $little))))
      (then (i32.const 4)) (else (i32.const 0))))

  (func $bits_read_utf32 (param $value eqref) (param $start i64) (param $little i32) (result eqref)
    (if (i32.eqz (call $bits_utf32_size (local.get $value) (local.get $start) (local.get $little)))
      (then unreachable))
    (call $bits_codepoint_new (i32.wrap_i64 (call $bits_read_u64 (local.get $value) (local.get $start)
      (i64.const 32) (local.get $little)))))

  (func $bits_read_utf (param $value eqref) (param $start i64) (param $encoding i32) (param $little i32)
      (result eqref i64 i32)
    (local $count i32) (local $codepoint eqref)
    (if (i32.eq (local.get $encoding) (i32.const 8))
      (then
        (local.set $count (call $bits_utf8_size (local.get $value) (local.get $start)))
        (if (local.get $count) (then (local.set $codepoint (call $bits_read_utf8 (local.get $value) (local.get $start))))))
      (else (if (i32.eq (local.get $encoding) (i32.const 16))
        (then
          (local.set $count (call $bits_utf16_size (local.get $value) (local.get $start) (local.get $little)))
          (if (local.get $count) (then (local.set $codepoint (call $bits_read_utf16 (local.get $value) (local.get $start) (local.get $little))))))
        (else (if (i32.eq (local.get $encoding) (i32.const 32))
          (then
            (local.set $count (call $bits_utf32_size (local.get $value) (local.get $start) (local.get $little)))
            (if (local.get $count) (then (local.set $codepoint (call $bits_read_utf32 (local.get $value) (local.get $start) (local.get $little))))))
          (else unreachable))))))
    (local.get $codepoint)
    (i64.mul (i64.extend_i32_u (local.get $count)) (i64.const 8))
    (i32.ne (local.get $count) (i32.const 0)))

  (func $bits_put_word (param $data (ref $bytes)) (param $index i32) (param $word i32) (param $little i32)
    (array.set $bytes (local.get $data) (local.get $index)
      (if (result i32) (local.get $little) (then (local.get $word))
        (else (i32.shr_u (local.get $word) (i32.const 8)))))
    (array.set $bytes (local.get $data) (i32.add (local.get $index) (i32.const 1))
      (if (result i32) (local.get $little) (then (i32.shr_u (local.get $word) (i32.const 8)))
        (else (local.get $word)))))

  (func $bits_from_codepoint (param $value eqref) (param $encoding i32) (param $little i32) (result eqref)
    (local $codepoint i32) (local $count i32) (local $data (ref null $bytes))
    (local.set $codepoint (call $bits_codepoint_get (local.get $value)))
    (if (i32.eqz (call $bits_valid_codepoint (local.get $codepoint))) (then unreachable))
    (if (i32.eq (local.get $encoding) (i32.const 32))
      (then (return (call $bits_from_u64 (i64.extend_i32_u (local.get $codepoint)) (i64.const 32) (local.get $little)))))
    (if (i32.eq (local.get $encoding) (i32.const 16)) (then
      (local.set $count (if (result i32) (i32.lt_u (local.get $codepoint) (i32.const 65536))
        (then (i32.const 2)) (else (i32.const 4))))
      (local.set $data (array.new_default $bytes (local.get $count)))
      (if (i32.eq (local.get $count) (i32.const 2))
        (then (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $codepoint) (local.get $little)))
        (else
          (local.set $codepoint (i32.sub (local.get $codepoint) (i32.const 65536)))
          (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.const 0)
            (i32.add (i32.const 55296) (i32.shr_u (local.get $codepoint) (i32.const 10))) (local.get $little))
          (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.const 2)
            (i32.add (i32.const 56320) (i32.and (local.get $codepoint) (i32.const 1023))) (local.get $little))))
      (return (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0)
        (i64.mul (i64.extend_i32_u (local.get $count)) (i64.const 8))))))
    (if (i32.ne (local.get $encoding) (i32.const 8)) (then unreachable))
    (local.set $count
      (if (result i32) (i32.lt_u (local.get $codepoint) (i32.const 128)) (then (i32.const 1))
        (else (if (result i32) (i32.lt_u (local.get $codepoint) (i32.const 2048)) (then (i32.const 2))
          (else (if (result i32) (i32.lt_u (local.get $codepoint) (i32.const 65536))
            (then (i32.const 3)) (else (i32.const 4))))))))
    (local.set $data (array.new_default $bytes (local.get $count)))
    (if (i32.eq (local.get $count) (i32.const 1))
      (then (array.set $bytes (local.get $data) (i32.const 0) (local.get $codepoint)))
      (else
        (array.set $bytes (local.get $data) (i32.sub (local.get $count) (i32.const 1))
          (i32.or (i32.const 128) (i32.and (local.get $codepoint) (i32.const 63))))
        (if (i32.eq (local.get $count) (i32.const 2))
          (then (array.set $bytes (local.get $data) (i32.const 0)
            (i32.or (i32.const 192) (i32.shr_u (local.get $codepoint) (i32.const 6)))))
          (else
            (array.set $bytes (local.get $data) (i32.sub (local.get $count) (i32.const 2))
              (i32.or (i32.const 128) (i32.and (i32.shr_u (local.get $codepoint) (i32.const 6)) (i32.const 63))))
            (if (i32.eq (local.get $count) (i32.const 3))
              (then (array.set $bytes (local.get $data) (i32.const 0)
                (i32.or (i32.const 224) (i32.shr_u (local.get $codepoint) (i32.const 12)))))
              (else
                (array.set $bytes (local.get $data) (i32.const 1)
                  (i32.or (i32.const 128) (i32.and (i32.shr_u (local.get $codepoint) (i32.const 12)) (i32.const 63))))
                (array.set $bytes (local.get $data) (i32.const 0)
                  (i32.or (i32.const 240) (i32.shr_u (local.get $codepoint) (i32.const 18))))))))))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0)
      (i64.mul (i64.extend_i32_u (local.get $count)) (i64.const 8))))

  (func $bits_from_wide_string (param $value eqref) (param $encoding i32) (param $little i32) (result eqref)
    (local $input eqref) (local $data (ref null $bytes)) (local $source i64)
    (local $destination i32) (local $count i32) (local $codepoint i32) (local $capacity i64)
    (local.set $input (call $bits_from_string (local.get $value)))
    (local.set $capacity (i64.mul (i64.div_u (call $bits_len64 (local.get $input)) (i64.const 8))
      (i64.extend_i32_u (i32.div_u (local.get $encoding) (i32.const 8)))))
    (if (i64.gt_u (local.get $capacity) (i64.const 4294967295)) (then unreachable))
    (local.set $data (array.new_default $bytes (i32.wrap_i64 (local.get $capacity))))
    (block $done (loop $encode
      (br_if $done (i64.ge_u (local.get $source) (call $bits_len64 (local.get $input))))
      (local.set $count (call $bits_utf8_size (local.get $input) (local.get $source)))
      (if (i32.eqz (local.get $count)) (then unreachable))
      (local.set $codepoint (call $bits_codepoint_get (call $bits_read_utf8 (local.get $input) (local.get $source))))
      (if (i32.eq (local.get $encoding) (i32.const 16))
        (then
          (if (i32.lt_u (local.get $codepoint) (i32.const 65536))
            (then
              (call $bits_put_word (ref.as_non_null (local.get $data)) (local.get $destination) (local.get $codepoint) (local.get $little))
              (local.set $destination (i32.add (local.get $destination) (i32.const 2))))
            (else
              (local.set $codepoint (i32.sub (local.get $codepoint) (i32.const 65536)))
              (call $bits_put_word (ref.as_non_null (local.get $data)) (local.get $destination)
                (i32.add (i32.const 55296) (i32.shr_u (local.get $codepoint) (i32.const 10))) (local.get $little))
              (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.add (local.get $destination) (i32.const 2))
                (i32.add (i32.const 56320) (i32.and (local.get $codepoint) (i32.const 1023))) (local.get $little))
              (local.set $destination (i32.add (local.get $destination) (i32.const 4))))))
        (else
          (if (local.get $little)
            (then
              (call $bits_put_word (ref.as_non_null (local.get $data)) (local.get $destination) (local.get $codepoint) (i32.const 1))
              (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.add (local.get $destination) (i32.const 2))
                (i32.shr_u (local.get $codepoint) (i32.const 16)) (i32.const 1)))
            (else
              (call $bits_put_word (ref.as_non_null (local.get $data)) (local.get $destination)
                (i32.shr_u (local.get $codepoint) (i32.const 16)) (i32.const 0))
              (call $bits_put_word (ref.as_non_null (local.get $data)) (i32.add (local.get $destination) (i32.const 2))
                (local.get $codepoint) (i32.const 0))))
          (local.set $destination (i32.add (local.get $destination) (i32.const 4)))))
      (local.set $source (i64.add (local.get $source) (i64.mul (i64.extend_i32_u (local.get $count)) (i64.const 8))))
      (br $encode)))
    (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0)
      (i64.mul (i64.extend_i32_u (local.get $destination)) (i64.const 8))))

  (func $bits_from_utf16_string (param $value eqref) (param $little i32) (result eqref)
    (call $bits_from_wide_string (local.get $value) (i32.const 16) (local.get $little)))

  (func $bits_from_utf32_string (param $value eqref) (param $little i32) (result eqref)
    (call $bits_from_wide_string (local.get $value) (i32.const 32) (local.get $little)))

  (func $bits_from_utf (param $value eqref) (param $encoding i32) (param $little i32) (param $codepoint i32) (result eqref)
    (if (local.get $codepoint) (then
      (return (call $bits_from_codepoint (local.get $value) (local.get $encoding) (local.get $little)))))
    (if (i32.eq (local.get $encoding) (i32.const 8))
      (then (return (call $bits_from_string (local.get $value)))))
    (if (i32.eqz (i32.or (i32.eq (local.get $encoding) (i32.const 16)) (i32.eq (local.get $encoding) (i32.const 32))))
      (then unreachable))
    (call $bits_from_wide_string (local.get $value) (local.get $encoding) (local.get $little)))

  (func $bits_to_string (param $value eqref) (result eqref)
    (local $length i64) (local $count i32) (local $data (ref null $bytes))
    (local $index i32) (local $position i64) (local $size i32)
    (local.set $length (call $bits_len64 (local.get $value)))
    (if (i64.ne (i64.and (local.get $length) (i64.const 7)) (i64.const 0)) (then unreachable))
    (local.set $count (i32.wrap_i64 (i64.shr_u (local.get $length) (i64.const 3))))
    (local.set $data (array.new_default $bytes (local.get $count)))
    (if (i64.eqz (i64.and (call $bits_offset (local.get $value)) (i64.const 7)))
      (then
        (array.copy $bytes $bytes (local.get $data) (i32.const 0)
          (call $bits_data (local.get $value))
          (i32.wrap_i64 (i64.shr_u (call $bits_offset (local.get $value)) (i64.const 3))) (local.get $count)))
      (else
        (block $copy_done (loop $copy
          (br_if $copy_done (i32.ge_u (local.get $index) (local.get $count)))
          (array.set $bytes (local.get $data) (local.get $index)
            (call $bits_byte (local.get $value) (i64.mul (i64.extend_i32_u (local.get $index)) (i64.const 8))))
          (local.set $index (i32.add (local.get $index) (i32.const 1)))
          (br $copy)))))
    (block $valid (loop $validate
      (br_if $valid (i64.ge_u (local.get $position) (local.get $length)))
      (local.set $size (call $bits_utf8_size (local.get $value) (local.get $position)))
      (if (i32.eqz (local.get $size)) (then unreachable))
      (local.set $position (i64.add (local.get $position) (i64.mul (i64.extend_i32_u (local.get $size)) (i64.const 8))))
      (br $validate)))
    (call $string_from_bytes (ref.as_non_null (local.get $data))))

"#;

#[cfg(test)]
mod tests {
    use num_bigint::{BigInt, Sign};
    use std::fmt::Write as _;
    use std::process::Command;

    // These tests execute the emitted runtime with real arbitrary-precision
    // helpers. The executor is a development tool, not a compiler dependency.
    const FLOAT_HELPERS: &str = r#"
      (func $utf_check (param $bits eqref) (param $encoding i32) (param $little i32)
          (param $codepoint i32) (param $size i64) (result i32)
        (local $value eqref) (local $length i64) (local $success i32)
        (call $bits_read_utf (local.get $bits) (i64.const 0)
          (local.get $encoding) (local.get $little))
        (local.set $success) (local.set $length) (local.set $value)
        (if (i32.eq (local.get $codepoint) (i32.const -1)) (then
          (return (i32.and (i32.eqz (local.get $success))
            (i32.and (ref.is_null (local.get $value)) (i64.eqz (local.get $length)))))))
        (if (i32.eqz (local.get $success)) (then (return (i32.const 0))))
        (i32.and
          (i32.eq (call $bits_codepoint_get (local.get $value)) (local.get $codepoint))
          (i64.eq (local.get $length) (local.get $size))))
    "#;

    fn literal(value: &BigInt) -> String {
        let (sign, limbs) = value.to_u32_digits();
        let sign = if sign == Sign::Minus { -1 } else { 1 };
        let mut out = format!(
            "(call $int_pack (i32.const {sign}) (array.new_fixed $limbs {}",
            limbs.len()
        );
        for limb in limbs {
            write!(out, " (i32.const {limb})").unwrap();
        }
        out.push_str("))");
        out
    }

    fn byte_array(bytes: &[u8]) -> String {
        let mut out = format!("(array.new_fixed $bytes {}", bytes.len());
        for byte in bytes {
            write!(out, " (i32.const {byte})").unwrap();
        }
        out.push(')');
        out
    }

    fn bits(bytes: &[u8], length: usize, offset: usize) -> String {
        format!(
            "(call $bits_new {} (i32.const {offset}) (i64.const {length}))",
            byte_array(bytes)
        )
    }

    fn padded_be(value: &BigInt, length: usize) -> Vec<u8> {
        if length == 0 {
            return Vec::new();
        }
        let (_, bytes) = value.to_bytes_be();
        let mut out = vec![0; length.saturating_sub(bytes.len())];
        out.extend(bytes);
        out
    }

    fn unsigned(value: &BigInt, width: usize) -> BigInt {
        let modulus = BigInt::from(1_u8) << width;
        ((value % &modulus) + &modulus) % &modulus
    }

    // The oracle uses BigInt's byte conversion rather than the runtime's
    // per-bit indexing. Partial bytes are left-aligned in the output stream.
    fn encoding(value: &BigInt, width: usize, little: bool) -> Vec<u8> {
        if width == 0 {
            return Vec::new();
        }
        let value = unsigned(value, width);
        let length = width.div_ceil(8);
        if little {
            let (_, mut bytes) = value.to_bytes_le();
            bytes.resize(length, 0);
            if !width.is_multiple_of(8) {
                *bytes.last_mut().unwrap() <<= 8 - width % 8;
            }
            bytes
        } else {
            padded_be(&(value << ((8 - width % 8) % 8)), length)
        }
    }

    fn unaligned(bytes: &[u8], length: usize) -> String {
        let value = BigInt::from_bytes_be(Sign::Plus, bytes) >> (bytes.len() * 8 - length);
        let with_prefix = ((BigInt::from(5_u8) << length) | value) << ((8 - (length + 3) % 8) % 8);
        bits(
            &padded_be(&with_prefix, (length + 3).div_ceil(8)),
            length,
            3,
        )
    }

    #[derive(Default)]
    struct Corpus {
        conditions: String,
        descriptions: Vec<String>,
    }

    impl Corpus {
        fn check(&mut self, condition: String, description: String) {
            self.descriptions.push(description);
            writeln!(
                self.conditions,
                "(if (i32.eqz {condition}) (then (return (i32.const {}))))",
                self.descriptions.len()
            )
            .unwrap();
        }

        fn integer(&mut self, value: &BigInt, width: usize, little: bool) {
            let encoded = encoding(value, width, little);
            let little = i32::from(little);
            let source = format!(
                "(call $bits_from_int {} (i64.const {width}) (i32.const {little}))",
                literal(value)
            );
            self.check(
                format!(
                    "(call $bits_match_equal {source} {})",
                    bits(&encoded, width, 0)
                ),
                format!("encode value={value}, width={width}, little={little}"),
            );
            for signed in [false, true] {
                let mut expected = unsigned(value, width);
                if signed && width != 0 && expected >= (BigInt::from(1_u8) << (width - 1)) {
                    expected -= BigInt::from(1_u8) << width;
                }
                self.check(
                    format!(
                        "(i32.eqz (call $int_compare (call $bits_read_int {} \
                        (i64.const 0) (i64.const {width}) (i32.const {}) (i32.const {little})) {}))",
                        unaligned(&encoded, width),
                        i32::from(signed),
                        literal(&expected),
                    ),
                    format!(
                        "decode value={value}, width={width}, little={little}, signed={signed}"
                    ),
                );
            }
        }

        fn utf(&mut self, codepoint: char, encoding: u32, little: bool) {
            let bytes: Vec<u8> = match encoding {
                8 => codepoint.to_string().into_bytes(),
                16 => codepoint
                    .encode_utf16(&mut [0_u16; 2])
                    .iter()
                    .flat_map(|unit| {
                        if little {
                            unit.to_le_bytes()
                        } else {
                            unit.to_be_bytes()
                        }
                    })
                    .collect(),
                32 => {
                    if little {
                        (codepoint as u32).to_le_bytes().to_vec()
                    } else {
                        (codepoint as u32).to_be_bytes().to_vec()
                    }
                }
                _ => unreachable!(),
            };
            let codepoint = codepoint as u32;
            let little = i32::from(little);
            self.check(
                format!(
                    "(call $bits_match_equal (call $bits_from_codepoint \
                    (call $bits_codepoint_new (i32.const {codepoint})) \
                    (i32.const {encoding}) (i32.const {little})) {})",
                    bits(&bytes, bytes.len() * 8, 0)
                ),
                format!("encode UTF{encoding}, scalar={codepoint}, little={little}"),
            );
            self.check(
                format!(
                    "(call $utf_check {} (i32.const {encoding}) (i32.const {little}) \
                    (i32.const {codepoint}) (i64.const {}))",
                    unaligned(&bytes, bytes.len() * 8),
                    bytes.len() * 8
                ),
                format!("decode UTF{encoding}, scalar={codepoint}, little={little}"),
            );
        }

        fn execute(self) {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("bitarrays.wasm");
            let source = format!(
                "(module {} {} {FLOAT_HELPERS} {} \
                 (func (export \"check\") (result i32) {} (i32.const 0)))",
                crate::runtime::TYPES,
                crate::runtime::tests::STUB_HOST,
                crate::runtime::wat(),
                self.conditions,
            );
            let module = wat::parse_str(&source).unwrap();
            wasmparser::Validator::new().validate_all(&module).unwrap();
            std::fs::write(&path, module).unwrap();
            let executable = std::env::var_os("WASMTIME").unwrap_or_else(|| "wasmtime".into());
            let output = Command::new(executable)
                .args(["run", "--invoke", "check"])
                .arg(&path)
                .output()
                .expect(
                    "Wasmtime is required for runtime tests; run them in the Devenv shell or set WASMTIME",
                );
            assert!(
                output.status.success(),
                "runtime trapped: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let result: usize = String::from_utf8(output.stdout)
                .unwrap()
                .trim()
                .parse()
                .unwrap();
            assert_eq!(
                result,
                0,
                "{}",
                self.descriptions
                    .get(result.wrapping_sub(1))
                    .map(String::as_str)
                    .unwrap_or("unexpected test result")
            );
        }
    }

    #[test]
    fn integer_segments_preserve_width_endianness_and_sign() {
        let mut corpus = Corpus::default();
        for width in [
            0, 1, 7, 8, 13, 16, 20, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 512,
        ] {
            let boundary: BigInt = BigInt::from(1_u8) << width;
            for value in [
                BigInt::from(0_u8),
                BigInt::from(1_i8),
                BigInt::from(-1_i8),
                &boundary - 1_u8,
                -(&boundary - 1_u8),
                boundary.clone(),
                -boundary,
            ] {
                for little in [false, true] {
                    corpus.integer(&value, width, little);
                }
            }
        }
        for seed in 0_u128..40 {
            let width = (seed as usize * 37 + 11) % 513;
            let value =
                BigInt::from(seed.wrapping_mul(0x9e3779b97f4a7c15)) << (seed as usize * 19 % 300);
            for little in [false, true] {
                corpus.integer(&value, width, little);
                corpus.integer(&(-&value), width, little);
            }
        }
        for width in [0, 1, 9, 32, 511] {
            let bytes = encoding(&BigInt::from(123_u8), width, false);
            let value = bits(&bytes, width, 0);
            for (start, length, valid) in [
                (-1, 0, 0),
                (0, -1, 0),
                (width as i64, 0, 1),
                (width as i64 + 1, 0, 0),
                (0, width as i64, 1),
                (0, width as i64 + 1, 0),
            ] {
                corpus.check(
                    format!(
                        "(i32.eq (call $bits_has {value} (i64.const {start}) \
                        (i64.const {length})) (i32.const {valid}))"
                    ),
                    format!("bounds width={width}, start={start}, length={length}"),
                );
            }
        }
        corpus.execute();
    }

    #[test]
    fn half_precision_rounding_and_utf_match_failures() {
        let mut corpus = Corpus::default();
        let halfway_subnormal = 2_f64.powi(-25);
        let halfway_normal = 1.0 + 2_f64.powi(-11);
        for (number, half) in [
            (0.0_f64, 0x0000_u16),
            (-0.0, 0x8000),
            (1.0, 0x3c00),
            (-2.0, 0xc000),
            (65504.0, 0x7bff),
            (65519.0, 0x7bff),
            (65520.0, 0x7c00),
            (-65520.0, 0xfc00),
            (2_f64.powi(-14), 0x0400),
            (2_f64.powi(-24), 0x0001),
            (halfway_subnormal, 0x0000),
            (halfway_subnormal.next_up(), 0x0001),
            (halfway_subnormal * 3.0, 0x0002),
            (halfway_normal, 0x3c00),
            (halfway_normal.next_up(), 0x3c01),
            (1.0 + 3.0 * 2_f64.powi(-11), 0x3c02),
        ] {
            for little in [false, true] {
                for width in [16, 32, 64] {
                    // A half that is not finite stops instead (spec §8.3).
                    if width == 16 && half & 0x7c00 == 0x7c00 {
                        continue;
                    }
                    let bytes = match (width, little) {
                        (16, false) => half.to_be_bytes().to_vec(),
                        (16, true) => half.to_le_bytes().to_vec(),
                        (32, false) => (number as f32).to_bits().to_be_bytes().to_vec(),
                        (32, true) => (number as f32).to_bits().to_le_bytes().to_vec(),
                        (64, false) => number.to_bits().to_be_bytes().to_vec(),
                        (64, true) => number.to_bits().to_le_bytes().to_vec(),
                        _ => unreachable!(),
                    };
                    corpus.check(
                        format!(
                            "(call $bits_match_equal (call $bits_from_float \
                            (call $float_new (f64.reinterpret_i64 (i64.const {}))) \
                            (i64.const {width}) (i32.const {})) {})",
                            number.to_bits(),
                            i32::from(little),
                            bits(&bytes, width, 0)
                        ),
                        format!("float encoding number={number:?}, width={width}, little={little}"),
                    );
                }
            }
        }
        for scalar in [
            0, 0x7f, 0x80, 0x7ff, 0x800, 0xd7ff, 0xe000, 0xffff, 0x10000, 0x10ffff, 0x1f600, 0x4e00,
        ] {
            for encoding in [8, 16, 32] {
                for little in [false, true] {
                    corpus.utf(char::from_u32(scalar).unwrap(), encoding, little);
                }
            }
        }
        let invalid: &[&[u8]] = &[
            b"",
            b"\x80",
            b"\xc0\x80",
            b"\xe0\x80\x80",
            b"\xf0\x80\x80\x80",
            b"\xed\xa0\x80",
            b"\xf4\x90\x80\x80",
            b"\xff",
            b"\xc2",
            b"\xe2\x82",
            b"\xf0\x9f\x98",
            b"\xc2A",
        ];
        for bytes in invalid {
            corpus.check(
                format!(
                    "(call $utf_check {} (i32.const 8) (i32.const 0) (i32.const -1) (i64.const 0))",
                    unaligned(bytes, bytes.len() * 8)
                ),
                format!("invalid UTF8 {bytes:02x?}"),
            );
        }
        for encoding in [16, 32] {
            for little in [false, true] {
                for scalar in [0xd800_u32, 0xdfff, 0x110000] {
                    if encoding == 16 && scalar > 65535 {
                        continue;
                    }
                    let bytes = if encoding == 16 {
                        if little {
                            (scalar as u16).to_le_bytes().to_vec()
                        } else {
                            (scalar as u16).to_be_bytes().to_vec()
                        }
                    } else if little {
                        scalar.to_le_bytes().to_vec()
                    } else {
                        scalar.to_be_bytes().to_vec()
                    };
                    corpus.check(
                        format!(
                            "(call $utf_check {} (i32.const {encoding}) (i32.const {}) \
                            (i32.const -1) (i64.const 0))",
                            unaligned(&bytes, bytes.len() * 8),
                            i32::from(little)
                        ),
                        format!("invalid UTF{encoding} {bytes:02x?}"),
                    );
                }
            }
        }
        corpus.execute();
    }

    #[test]
    fn bit_views_concatenate_and_preserve_unicode_strings() {
        let mut corpus = Corpus::default();
        for seed in 0_usize..64 {
            let left_width = seed * 13 % 101;
            let right_width = seed * 29 % 107;
            let left = unsigned(&(BigInt::from(seed + 1) << (seed * 7 % 120)), left_width);
            let right = unsigned(
                &(BigInt::from(seed * 17 + 3) << (seed * 11 % 120)),
                right_width,
            );
            let left_bytes = encoding(&left, left_width, false);
            let right_bytes = encoding(&right, right_width, false);
            let joined = (&left << right_width) | &right;
            let joined_width = left_width + right_width;
            let joined_bytes = encoding(&joined, joined_width, false);
            let source = format!(
                "(call $bits_concat {} {})",
                unaligned(&left_bytes, left_width),
                unaligned(&right_bytes, right_width)
            );
            corpus.check(
                format!(
                    "(call $bits_match_equal {source} {})",
                    bits(&joined_bytes, joined_width, 0)
                ),
                format!("concatenate {left_width} + {right_width} bits"),
            );
            corpus.check(
                format!(
                    "(call $bits_match_equal (call $bits_slice {source} \
                    (i64.const {left_width}) (i64.const {right_width})) {})",
                    bits(&right_bytes, right_width, 0)
                ),
                format!("slice concatenation at bit {left_width}"),
            );
        }
        // Aligned views take the bulk array.copy path, including nonzero
        // offsets into their backing arrays.
        let left = bits(&[0xff, 0x10, 0x20, 0xff], 16, 8);
        let right = bits(&[0xff, 0xff, 0x30, 0x40], 16, 16);
        corpus.check(
            format!(
                "(call $bits_match_equal (call $bits_concat {left} {right}) {})",
                bits(&[0x10, 0x20, 0x30, 0x40], 32, 0)
            ),
            "concatenate aligned nonzero-offset views".into(),
        );
        for value in ["", "hello", "中文", "a\u{301}😀\u{10ffff}"] {
            let bytes = value.as_bytes();
            let source = format!(
                "(call $bits_to_string {})",
                unaligned(bytes, bytes.len() * 8)
            );
            corpus.check(
                format!(
                    "(call $bits_match_equal (call $bits_from_string {source}) {})",
                    bits(bytes, bytes.len() * 8, 0)
                ),
                format!("copy UTF8 string from an unaligned bit view: {value:?}"),
            );
            for encoding in [16, 32] {
                for little in [false, true] {
                    let encoded: Vec<u8> = if encoding == 16 {
                        value
                            .encode_utf16()
                            .flat_map(|unit| {
                                if little {
                                    unit.to_le_bytes()
                                } else {
                                    unit.to_be_bytes()
                                }
                            })
                            .collect()
                    } else {
                        value
                            .chars()
                            .flat_map(|scalar| {
                                if little {
                                    (scalar as u32).to_le_bytes()
                                } else {
                                    (scalar as u32).to_be_bytes()
                                }
                            })
                            .collect()
                    };
                    corpus.check(
                        format!(
                            "(call $bits_match_equal (call $bits_from_utf {source} \
                            (i32.const {encoding}) (i32.const {}) (i32.const 0)) {})",
                            i32::from(little),
                            bits(&encoded, encoded.len() * 8, 0)
                        ),
                        format!("UTF{encoding} string encoding little={little}: {value:?}"),
                    );
                }
            }
        }
        corpus.execute();
    }
}
