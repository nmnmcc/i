//! Shared runtime definitions. Language values live on the Wasm GC heap;
//! linear memory is used only for host transfers.
mod bigint;
mod bitarrays;
mod values;

pub const TYPES: &str = r#"
(type $bytes (array (mut i8)))
(type $values (array (mut eqref)))
(type $limbs (array (mut i32)))
(type $call (func (param eqref eqref) (result eqref)))
(type $obj (struct
 (field i32) (field i32) (field i64) (field f64)
 (field (ref null $bytes)) (field (ref null $values))
 (field (ref null $limbs)) (field (ref null $call))))
"#;

pub const HOST: &str = r#"
(import "wleam:host/v1" "write" (func $host_write (param i32 i32 i32)))
(import "wleam:host/v1" "random" (func $host_random (param i32 i32) (result i32)))
(import "wleam:host/v1" "panic" (func $host_panic (param i32 i32)))
(import "wleam:host/v1" "reference_id" (func $host_reference_id (param externref) (result i64)))
"#;

pub fn wat() -> &'static str {
    static RUNTIME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| {
        [
            MEMORY.to_owned(),
            bigint::wat(),
            bitarrays::wat(),
            values::wat(),
            EXTRA.to_owned(),
        ]
        .concat()
    })
}

const MEMORY: &str = r#"
(memory (export "memory") 1)
(global $heap (mut i32) (i32.const 1024))
(global $free_head (mut i32) (i32.const 0))
(func $alloc (export "__wleam_alloc") (param $len i32) (result i32)
 (local $size i32) (local $p i32) (local $prev i32) (local $end i64) (local $available i64)
 (if (i32.gt_u (local.get $len) (i32.const 2147483632)) (then unreachable))
 (local.set $size (i32.and (i32.add (local.get $len) (i32.const 7)) (i32.const -8)))
 (if (i32.eqz (local.get $size)) (then (local.set $size (i32.const 8))))
 (local.set $p (global.get $free_head))
 (block $not_found (loop $search
   (br_if $not_found (i32.eqz (local.get $p)))
   (if (i32.ge_u (i32.load (local.get $p)) (local.get $size))
     (then
       (if (local.get $prev)
         (then (i32.store offset=4 (local.get $prev) (i32.load offset=4 (local.get $p))))
         (else (global.set $free_head (i32.load offset=4 (local.get $p)))))
       (return (i32.add (local.get $p) (i32.const 8)))))
   (local.set $prev (local.get $p))
   (local.set $p (i32.load offset=4 (local.get $p))) (br $search)))
 (local.set $p (global.get $heap))
 ;; Calculate addresses in i64: a full memory32 has 2^32 bytes, which cannot
 ;; be represented by i32. Keep the next heap pointer representable as well.
 (local.set $end (i64.add (i64.extend_i32_u (local.get $p))
   (i64.add (i64.extend_i32_u (local.get $size)) (i64.const 8))))
 (if (i64.gt_u (local.get $end) (i64.const 4294967295)) (then unreachable))
 (local.set $available (i64.shl (i64.extend_i32_u (memory.size)) (i64.const 16)))
 (if (i64.gt_u (local.get $end) (local.get $available))
   (then
     (if (i32.eq (memory.grow (i32.wrap_i64
       (i64.shr_u (i64.add (i64.sub (local.get $end) (local.get $available)) (i64.const 65535)) (i64.const 16)))) (i32.const -1))
       (then unreachable))))
 (i32.store (local.get $p) (local.get $size))
 (i32.store offset=4 (local.get $p) (i32.const 0))
 (global.set $heap (i32.wrap_i64 (local.get $end)))
 (i32.add (local.get $p) (i32.const 8)))
(func $free (export "__wleam_free") (param $ptr i32) (param $len i32)
 (local $p i32)
 (if (i32.eqz (local.get $ptr)) (then return))
 (local.set $p (i32.sub (local.get $ptr) (i32.const 8)))
 (i32.store offset=4 (local.get $p) (global.get $free_head))
 (global.set $free_head (local.get $p)))
"#;

const EXTRA: &str = r#"
(func $float_add (param $a eqref) (param $b eqref) (result eqref)
 (call $float_new (f64.add (call $float_get (local.get $a)) (call $float_get (local.get $b)))))
(func $float_sub (param $a eqref) (param $b eqref) (result eqref)
 (call $float_new (f64.sub (call $float_get (local.get $a)) (call $float_get (local.get $b)))))
(func $float_mul (param $a eqref) (param $b eqref) (result eqref)
 (call $float_new (f64.mul (call $float_get (local.get $a)) (call $float_get (local.get $b)))))
(func $float_div (param $a eqref) (param $b eqref) (result eqref)
 (if (result eqref) (f64.eq (call $float_get (local.get $b)) (f64.const 0))
   (then (call $float_new (f64.const 0)))
   (else (call $float_new (f64.div (call $float_get (local.get $a)) (call $float_get (local.get $b)))))))
(func $float_ceil (param $v eqref) (result eqref) (call $float_new (f64.ceil (call $float_get (local.get $v)))))
(func $float_floor (param $v eqref) (result eqref) (call $float_new (f64.floor (call $float_get (local.get $v)))))
(func $float_sqrt (param $v eqref) (result eqref) (call $float_new (f64.sqrt (call $float_get (local.get $v)))))
(func $float_random (result eqref)
 (local $p i32) (local $n i64)
 (local.set $p (call $alloc (i32.const 8)))
 (if (call $host_random (local.get $p) (i32.const 8)) (then unreachable))
 (local.set $n (i64.shr_u (i64.load (local.get $p)) (i64.const 11)))
 (call $free (local.get $p) (i32.const 8))
 (call $float_new (f64.div (f64.convert_i64_u (local.get $n)) (f64.const 9007199254740992))))
(func $write_string (param $s eqref) (param $channel i32)
 (local $bytes (ref $bytes)) (local $len i32) (local $i i32) (local $p i32)
 (local.set $bytes (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $s)))))
 (local.set $len (array.len (local.get $bytes)))
 (local.set $p (call $alloc (local.get $len)))
 (block $done (loop $copy
  (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
  (i32.store8 (i32.add (local.get $p) (local.get $i)) (array.get_u $bytes (local.get $bytes) (local.get $i)))
  (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $copy)))
 (call $host_write (local.get $channel) (local.get $p) (local.get $len))
 (call $free (local.get $p) (local.get $len)))
(func $io_print (param $s eqref) (result eqref) (call $write_string (local.get $s) (i32.const 1)) (ref.null eq))
(func $io_print_error (param $s eqref) (result eqref) (call $write_string (local.get $s) (i32.const 2)) (ref.null eq))
(func $panic (param $s eqref) (result eqref)
 (local $bytes (ref $bytes)) (local $len i32) (local $i i32) (local $p i32)
 (local.set $bytes (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $s)))))
 (local.set $len (array.len (local.get $bytes))) (local.set $p (call $alloc (local.get $len)))
 (block $done (loop $copy
  (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
  (i32.store8 (i32.add (local.get $p) (local.get $i)) (array.get_u $bytes (local.get $bytes) (local.get $i)))
  (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $copy)))
 (call $host_panic (local.get $p) (local.get $len)) unreachable)
(func $string_has_prefix (param $s eqref) (param $prefix eqref) (result i32)
 (local $a (ref $bytes)) (local $b (ref $bytes)) (local $i i32)
 (local.set $a (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $s)))))
 (local.set $b (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $prefix)))))
 (if (i32.lt_u (array.len (local.get $a)) (array.len (local.get $b))) (then (return (i32.const 0))))
 (block $done (loop $again
  (br_if $done (i32.ge_u (local.get $i) (array.len (local.get $b))))
  (if (i32.ne (array.get_u $bytes (local.get $a) (local.get $i)) (array.get_u $bytes (local.get $b) (local.get $i))) (then (return (i32.const 0))))
  (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $again))) (i32.const 1))
(func $string_suffix (param $s eqref) (param $start i32) (result eqref)
 (call $string_byte_slice (local.get $s) (call $int_from_i64 (i64.extend_i32_u (local.get $start)))
   (call $int_from_i64 (i64.extend_i32_u (i32.sub (array.len (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $s))))) (local.get $start))))))
(func $bit_array_bit_size (param $v eqref) (result eqref) (call $bits_size (local.get $v)))
(func $bit_array_unsafe_to_string (param $v eqref) (result eqref) (call $bits_to_string (local.get $v)))
(func $dynamic_cast (param $v eqref) (result eqref) (local.get $v))
(func $dynamic_array (param $v eqref) (result eqref)
 (i32.const 11) (i32.const 0) (i64.const 0) (f64.const 0) (ref.null $bytes)
 (array.new_fixed $values 1 (local.get $v)) (ref.null $limbs) (ref.null $call) (struct.new $obj))
(func $result (param $ok i32) (param $v eqref) (result eqref)
 (call $record (i32.eqz (local.get $ok)) (i64.const 0) (array.new_fixed $values 1 (local.get $v))))
(func $dynamic_string (param $v eqref) (result eqref)
 (if (result eqref) (i32.eq (call $value_tag (local.get $v)) (i32.const 3))
 (then (call $result (i32.const 1) (local.get $v)))
 (else (call $result (i32.const 0) (call $string_from_mem (i32.const 0) (i32.const 0))))))
(func $dynamic_int (param $v eqref) (result eqref)
 (if (result eqref) (i32.or (ref.test (ref i31) (local.get $v)) (i32.eq (call $value_tag (local.get $v)) (i32.const 8)))
 (then (call $result (i32.const 1) (local.get $v)))
 (else (call $result (i32.const 0) (ref.i31 (i32.const 0))))))
(func $dynamic_float (param $v eqref) (result eqref)
 (if (result eqref) (i32.eq (call $value_tag (local.get $v)) (i32.const 2))
 (then (call $result (i32.const 1) (local.get $v)))
 (else (call $result (i32.const 0) (call $float_new (f64.const 0))))))
(func $dynamic_bit_array (param $v eqref) (result eqref)
 (if (result eqref) (i32.eq (call $value_tag (local.get $v)) (i32.const 9))
 (then (call $result (i32.const 1) (local.get $v)))
 (else (call $result (i32.const 0) (call $bits_empty)))))
(func $dynamic_list (param $v eqref) (result eqref)
 (if (i32.eq (call $value_tag (local.get $v)) (i32.const 6)) (then (return (call $result (i32.const 1) (local.get $v)))))
 (if (i32.eq (call $value_tag (local.get $v)) (i32.const 11)) (then (return (call $result (i32.const 1) (call $field (local.get $v) (i32.const 0))))))
 (call $result (i32.const 0) (ref.null eq)))
(func $dynamic_dict (param $v eqref) (result eqref)
 (if (i32.eq (call $value_tag (local.get $v)) (i32.const 5))
  (then (if (call $type_is_dict (struct.get $obj 2 (ref.cast (ref $obj) (local.get $v))))
    (then (return (call $result (i32.const 1) (local.get $v)))))))
 (call $result (i32.const 0) (ref.null eq)))
"#;
