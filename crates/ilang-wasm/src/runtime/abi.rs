//! Value encodings at the host boundary (spec Appendix B.2 and B.3).
//!
//! `$abi_*_in` functions read a value that the host gave: export parameters
//! and import results. They copy it onto the GC heap and stop with
//! `host: value is not correct` when it is not correct. They never free.
//! `$abi_*_out` functions encode a value for the host: export results and
//! import parameters. Buffers come from `ilang_alloc`; an empty buffer has
//! the address 0 and is not allocated.
//!
//! Inline Ints (nonzero sign, count 0) have no buffer. Every actual buffer
//! that the host gives, also an empty one (a zero `Int`,
//! a `String` of length 0, a `BitArray` of 0 bits), must be inside memory:
//! its address plus its size is at most the size of memory (B.2). The
//! buffers of the results of an import are freed whenever their address is
//! not 0, also when they are empty: the host may make them with
//! `ilang_alloc(0)` (B.1, B.3).

pub fn wat(references: bool) -> String {
    if references {
        WAT.replace(
            "(func $abi_ref_in (param $value externref) (result eqref)\n  (call $host_ref_new (local.get $value)))",
            "(func $abi_ref_in (param $value externref) (param $type i32) (result eqref)\n  (call $host_ref_new (local.get $value) (local.get $type)))",
        )
    } else {
        WAT.to_owned()
    }
}

const WAT: &str = r#"
;; A buffer must lie inside memory.
(func $abi_buffer_check (param $address i32) (param $length i64)
  (if (i64.gt_u (i64.add (i64.extend_i32_u (local.get $address)) (local.get $length))
                (i64.shl (i64.extend_i32_u (memory.size)) (i64.const 16)))
    (then (call $stop_host_value) unreachable)))

(func $abi_bool_in (param $value i32) (result eqref)
  (if (i32.gt_u (local.get $value) (i32.const 1)) (then (call $stop_host_value) unreachable))
  (call $bool_new (local.get $value)))
(func $abi_bool_out (param $value eqref) (result i32)
  (call $bool_get (local.get $value)))

(func $abi_float_in (param $value f64) (result eqref)
  (if (i32.eqz (f64.le (f64.abs (local.get $value)) (f64.const 0x1.fffffffffffffp+1023)))
    (then (call $stop_host_value) unreachable))
  (call $float_new (local.get $value)))
(func $abi_float_out (param $value eqref) (result f64)
  (call $float_get (local.get $value)))

;; Sign -1, 0 or 1; count little-endian u32 parts; the last part is not 0;
;; zero is sign 0 and count 0. With count 0 and nonzero sign, address is
;; the signed i32 value; it must have that sign and is never an address.
(func $abi_int_in (param $sign i32) (param $address i32) (param $count i32) (result eqref)
  (local $limbs (ref $limbs)) (local $i i32)
  (if (i32.gt_u (i32.add (local.get $sign) (i32.const 1)) (i32.const 2))
    (then (call $stop_host_value) unreachable))
  (if (i32.and (i32.eqz (local.get $count)) (local.get $sign))
    (then
      (if (i32.ne (local.get $sign)
            (i32.sub (i32.gt_s (local.get $address) (i32.const 0))
                     (i32.lt_s (local.get $address) (i32.const 0))))
        (then (call $stop_host_value) unreachable))
      (return (call $int_from_i64 (i64.extend_i32_s (local.get $address))))))
  (if (i32.ne (i32.eqz (local.get $sign)) (i32.eqz (local.get $count)))
    (then (call $stop_host_value) unreachable))
  (call $abi_buffer_check (local.get $address)
    (i64.shl (i64.extend_i32_u (local.get $count)) (i64.const 2)))
  (if (i32.eqz (local.get $count)) (then (return (ref.i31 (i32.const 0)))))
  (local.set $limbs (array.new_default $limbs (local.get $count)))
  (block $done (loop $each
    (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
    (array.set $limbs (local.get $limbs) (local.get $i)
      (i32.load (i32.add (local.get $address) (i32.shl (local.get $i) (i32.const 2)))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $each)))
  (if (i32.eqz (array.get $limbs (local.get $limbs) (i32.sub (local.get $count) (i32.const 1))))
    (then (call $stop_host_value) unreachable))
  (call $int_pack (local.get $sign) (local.get $limbs)))
(func $abi_int_out (param $value eqref) (result i32 i32 i32)
  (local $limbs (ref $limbs)) (local $count i32) (local $address i32) (local $i i32)
  (local $sign i32) (local $x i32)
  ;; The common i31 path neither creates limbs nor allocates linear memory.
  (if (ref.test (ref i31) (local.get $value))
    (then
      (local.set $x (i31.get_s (ref.cast (ref i31) (local.get $value))))
      (return (call $int_sign (local.get $value)) (local.get $x) (i32.const 0))))
  (local.set $sign (call $int_sign (local.get $value)))
  (local.set $limbs (call $int_limbs (local.get $value)))
  (local.set $count (array.len (local.get $limbs)))
  (if (i32.eqz (local.get $count)) (then (return (i32.const 0) (i32.const 0) (i32.const 0))))
  (if (i32.eq (local.get $count) (i32.const 1))
    (then
      (local.set $x (array.get $limbs (local.get $limbs) (i32.const 0)))
      (if (i32.le_u (local.get $x)
            (select (i32.const 2147483648) (i32.const 2147483647)
              (i32.lt_s (local.get $sign) (i32.const 0))))
        (then (return (local.get $sign)
          (i32.mul (local.get $sign) (local.get $x)) (i32.const 0))))))
  (local.set $address (call $alloc (i32.shl (local.get $count) (i32.const 2))))
  (block $done (loop $each
    (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
    (i32.store (i32.add (local.get $address) (i32.shl (local.get $i) (i32.const 2)))
      (array.get $limbs (local.get $limbs) (local.get $i)))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $each)))
  (local.get $sign) (local.get $address) (local.get $count))

(func $abi_bytes_in (param $address i32) (param $length i32) (result (ref $bytes))
  (local $data (ref $bytes)) (local $i i32)
  (call $abi_buffer_check (local.get $address) (i64.extend_i32_u (local.get $length)))
  (local.set $data (array.new_default $bytes (local.get $length)))
  (block $done (loop $each
    (br_if $done (i32.ge_u (local.get $i) (local.get $length)))
    (array.set $bytes (local.get $data) (local.get $i)
      (i32.load8_u (i32.add (local.get $address) (local.get $i))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $each)))
  (local.get $data))

(func $abi_string_in (param $address i32) (param $length i32) (result eqref)
  (local $data (ref $bytes))
  (local.set $data (call $abi_bytes_in (local.get $address) (local.get $length)))
  (if (i32.eqz (call $utf8_valid (local.get $data))) (then (call $stop_host_value) unreachable))
  (call $string_from_bytes (local.get $data)))
(func $abi_string_out (param $value eqref) (result i32 i32)
  (local $length i32)
  (local.set $length (call $string_length_bytes (local.get $value)))
  (if (i32.eqz (local.get $length)) (then (return (i32.const 0) (i32.const 0))))
  (call $string_to_mem (local.get $value)) (local.get $length))

;; ceil(bits / 8) bytes, highest bit first; the bits after the end are 0.
(func $abi_bits_in (param $address i32) (param $bits i32) (result eqref)
  (local $data (ref $bytes)) (local $count i32) (local $rest i32)
  (local.set $count (i32.wrap_i64 (i64.shr_u
    (i64.add (i64.extend_i32_u (local.get $bits)) (i64.const 7)) (i64.const 3))))
  (local.set $data (call $abi_bytes_in (local.get $address) (local.get $count)))
  (local.set $rest (i32.and (local.get $bits) (i32.const 7)))
  (if (local.get $rest)
    (then
      (if (i32.and
            (array.get_u $bytes (local.get $data) (i32.sub (local.get $count) (i32.const 1)))
            (i32.sub (i32.shl (i32.const 1) (i32.sub (i32.const 8) (local.get $rest))) (i32.const 1)))
        (then (call $stop_host_value) unreachable))))
  (call $bits_new (local.get $data) (i32.const 0) (i64.extend_i32_u (local.get $bits))))
(func $abi_bits_out (param $value eqref) (result i32 i32)
  (local $length i64) (local $count i32) (local $address i32) (local $i i32) (local $rest i64)
  (local.set $length (call $bits_len64 (local.get $value)))
  ;; The interface has 32 bits for the length.
  (if (i64.gt_u (local.get $length) (i64.const 4294967295)) (then unreachable))
  (if (i64.eqz (local.get $length)) (then (return (i32.const 0) (i32.const 0))))
  (local.set $count (i32.wrap_i64 (i64.shr_u (i64.add (local.get $length) (i64.const 7)) (i64.const 3))))
  (local.set $address (call $alloc (local.get $count)))
  (block $done (loop $each
    (br_if $done (i64.gt_u (i64.shl (i64.extend_i32_u (i32.add (local.get $i) (i32.const 1))) (i64.const 3))
                           (local.get $length)))
    (i32.store8 (i32.add (local.get $address) (local.get $i))
      (call $bits_byte (local.get $value) (i64.shl (i64.extend_i32_u (local.get $i)) (i64.const 3))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $each)))
  (local.set $rest (i64.and (local.get $length) (i64.const 7)))
  (if (i64.ne (local.get $rest) (i64.const 0))
    (then
      (i32.store8 (i32.add (local.get $address) (local.get $i))
        (i32.wrap_i64 (i64.shl
          (call $bits_read_u64 (local.get $value)
            (i64.shl (i64.extend_i32_u (local.get $i)) (i64.const 3)) (local.get $rest) (i32.const 0))
          (i64.sub (i64.const 8) (local.get $rest)))))))
  (local.get $address) (i32.wrap_i64 (local.get $length)))

(func $abi_ref_in (param $value externref) (result eqref)
  (call $host_ref_new (local.get $value)))
(func $abi_ref_out (param $value eqref) (result externref)
  (call $host_ref_get (local.get $value)))

;; The results of an import: read, then the buffer is freed (B.3), also
;; when it is empty; `$free` does nothing for the address 0.
(func $abi_int_free (param $sign i32) (param $address i32) (param $count i32)
  (if (i32.or (local.get $count) (i32.eqz (local.get $sign)))
    (then (call $free (local.get $address) (i32.shl (local.get $count) (i32.const 2))))))
(func $abi_int_take (param $sign i32) (param $address i32) (param $count i32) (result eqref)
  (local $v eqref)
  (local.set $v (call $abi_int_in (local.get $sign) (local.get $address) (local.get $count)))
  (call $abi_int_free (local.get $sign) (local.get $address) (local.get $count))
  (local.get $v))
(func $abi_string_take (param $address i32) (param $length i32) (result eqref)
  (local $v eqref)
  (local.set $v (call $abi_string_in (local.get $address) (local.get $length)))
  (call $free (local.get $address) (local.get $length))
  (local.get $v))
(func $abi_bits_take (param $address i32) (param $bits i32) (result eqref)
  (local $v eqref)
  (local.set $v (call $abi_bits_in (local.get $address) (local.get $bits)))
  (call $free (local.get $address)
    (i32.wrap_i64 (i64.shr_u (i64.add (i64.extend_i32_u (local.get $bits)) (i64.const 7)) (i64.const 3))))
  (local.get $v))

;; The error tag of a result: 0, or 1 to the number of error members.
(func $abi_tag_in (param $tag i32) (param $members i32) (result i32)
  (if (i32.gt_u (local.get $tag) (local.get $members)) (then (call $stop_host_value) unreachable))
  (local.get $tag))
"#;
