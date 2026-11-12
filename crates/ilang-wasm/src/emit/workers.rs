//! Native multicore support (multicore.md D8/P1, native.md N7/N10/N11).
//!
//! The scheduler in `i/task` publishes the complete record of a portable task
//! at `go` as bytes. Another instance of the same module (a worker) claims the
//! record once, decodes it into its own heap and runs the task body as a
//! speculation: the body has no scheduler step, host call or stop in it, or
//! the worker abandons the speculation and the maker runs its retained
//! original. A finished speculation gives the encoded completion back to the
//! maker, which ends the task with it. Only the owner of a GC object reads it
//! to encode it (I4); a receiver decodes into its own heap.
//!
//! The codec keeps sharing and cycles (D9): every aggregate it enters gets the
//! next index, marked in its header as `core.portable` does, and a second
//! visit writes a back reference. Values that cannot move (boundaries,
//! continuations, host references, requests, runtime closures without a code
//! number) make the encoder give up; the record then stays on its maker.
//!
//! Format, one marker byte then its operands (little endian):
//! 0 Nil; 1 small Int (i32); 2 back reference (i32); 3 `$obj` (tag i32,
//! field 1 i32, field 2 i64, field 3 f64, bytes, limbs, values); 4 closure
//! (code i32, flags i32, arity i32, captures); 5 non-empty list (count, items);
//! 6 True; 7 False; 8 []. An array is its length (i32, -1 for null) and its
//! items.

/// Imports of the host transport (ABI v3 runtime imports, multicore wave).
pub(super) const IMPORTS: &str = r#"
(import "ilang:host/v1" "mc_publish" (func $host_mc_publish (param i32 i32) (result i64)))
(import "ilang:host/v1" "mc_claim" (func $host_mc_claim (param i64) (result i32)))
(import "ilang:host/v1" "mc_poll" (func $host_mc_poll (param i64) (result i32)))
(import "ilang:host/v1" "mc_take" (func $host_mc_take (param i64) (result i64)))
(import "ilang:host/v1" "mc_forget" (func $host_mc_forget (param i64)))
(import "ilang:host/v1" "mc_share_put" (func $host_mc_share_put (param i64 i64 i64 i32 i32)))
(import "ilang:host/v1" "mc_share_get" (func $host_mc_share_get (param i64 i64 i64) (result i64)))
(import "ilang:host/v1" "mc_share_drop" (func $host_mc_share_drop (param i64 i64 i64)))
"#;

/// The WASI target always runs one worker (D12).
pub(super) const WASI_STUBS: &str = r#"
(func $host_mc_publish (param i32 i32) (result i64) (i64.const 0))
(func $host_mc_claim (param i64) (result i32) (i32.const 1))
(func $host_mc_poll (param i64) (result i32) (i32.const 2))
(func $host_mc_take (param i64) (result i64) unreachable)
(func $host_mc_forget (param i64))
(func $host_mc_share_put (param i64 i64 i64 i32 i32))
(func $host_mc_share_get (param i64 i64 i64) (result i64) (i64.const 0))
(func $host_mc_share_drop (param i64 i64 i64))
"#;

pub(super) const WAT: &str = r#"
(global $mc_enabled (mut i32) (i32.const 0))
(global $enc_ptr (mut i32) (i32.const 0))
(global $enc_len (mut i32) (i32.const 0))
(global $enc_cap (mut i32) (i32.const 0))
(global $enc_fail (mut i32) (i32.const 0))
(global $enc_count (mut i32) (i32.const 0))
(global $enc_seen (mut (ref null $values)) (ref.null $values))
(global $enc_tags (mut (ref null $frames)) (ref.null $frames))
(global $dec_at (mut i32) (i32.const 0))
(global $dec_end (mut i32) (i32.const 0))
(global $dec_table (mut (ref null $values)) (ref.null $values))
(global $dec_count (mut i32) (i32.const 0))
(global $mc_out_ptr (mut i32) (i32.const 0))
(global $mc_out_len (mut i32) (i32.const 0))
(global $mc_out_cap (mut i32) (i32.const 0))

(func $enc_reserve (param $n i32)
  (local $new i32) (local $cap i32)
  (if (i32.le_u (i32.add (global.get $enc_len) (local.get $n)) (global.get $enc_cap)) (then (return)))
  (local.set $cap (i32.add (i32.shl (global.get $enc_cap) (i32.const 1)) (i32.add (local.get $n) (i32.const 64))))
  (local.set $new (call $alloc (local.get $cap)))
  (if (global.get $enc_len)
    (then (memory.copy (local.get $new) (global.get $enc_ptr) (global.get $enc_len))))
  (if (global.get $enc_cap) (then (call $free (global.get $enc_ptr) (global.get $enc_cap))))
  (global.set $enc_ptr (local.get $new))
  (global.set $enc_cap (local.get $cap)))
(func $enc_u8 (param $b i32)
  (call $enc_reserve (i32.const 1))
  (i32.store8 (i32.add (global.get $enc_ptr) (global.get $enc_len)) (local.get $b))
  (global.set $enc_len (i32.add (global.get $enc_len) (i32.const 1))))
(func $enc_i32 (param $v i32)
  (call $enc_reserve (i32.const 4))
  (i32.store (i32.add (global.get $enc_ptr) (global.get $enc_len)) (local.get $v))
  (global.set $enc_len (i32.add (global.get $enc_len) (i32.const 4))))
(func $enc_i64 (param $v i64)
  (call $enc_reserve (i32.const 8))
  (i64.store (i32.add (global.get $enc_ptr) (global.get $enc_len)) (local.get $v))
  (global.set $enc_len (i32.add (global.get $enc_len) (i32.const 8))))
(func $enc_f64 (param $v f64)
  (call $enc_reserve (i32.const 8))
  (f64.store (i32.add (global.get $enc_ptr) (global.get $enc_len)) (local.get $v))
  (global.set $enc_len (i32.add (global.get $enc_len) (i32.const 8))))

;; Give `v` the next index and mark its header, as `$core_portable` does.
(func $enc_mark (param $v eqref) (param $tag i32)
  (local $grown (ref null $values)) (local $tags (ref null $frames)) (local $n i32)
  (local.set $n (array.len (ref.as_non_null (global.get $enc_seen))))
  (if (i32.eq (global.get $enc_count) (local.get $n)) (then
    (local.set $grown (array.new_default $values (i32.shl (local.get $n) (i32.const 1))))
    (array.copy $values $values (ref.as_non_null (local.get $grown)) (i32.const 0)
      (ref.as_non_null (global.get $enc_seen)) (i32.const 0) (local.get $n))
    (global.set $enc_seen (local.get $grown))
    (local.set $tags (array.new_default $frames (i32.shl (local.get $n) (i32.const 1))))
    (array.copy $frames $frames (ref.as_non_null (local.get $tags)) (i32.const 0)
      (ref.as_non_null (global.get $enc_tags)) (i32.const 0) (local.get $n))
    (global.set $enc_tags (local.get $tags))))
  (array.set $values (ref.as_non_null (global.get $enc_seen)) (global.get $enc_count) (local.get $v))
  (array.set $frames (ref.as_non_null (global.get $enc_tags)) (global.get $enc_count) (local.get $tag))
  (struct.set $val 0 (ref.cast (ref $val) (local.get $v))
    (i32.or (global.get $enc_count) (i32.const -2147483648)))
  (global.set $enc_count (i32.add (global.get $enc_count) (i32.const 1))))

(func $enc_value (param $v eqref) (param $depth i32)
  (local $raw i32) (local $tag i32) (local $o (ref null $obj)) (local $xs (ref null $values))
  (local $b (ref null $bytes)) (local $l (ref null $limbs)) (local $n i32) (local $i i32)
  (local $w eqref) (local $code i32)
  (if (global.get $enc_fail) (then (return)))
  (if (i32.gt_u (local.get $depth) (i32.const 1500)) (then (global.set $enc_fail (i32.const 1)) (return)))
  (if (ref.is_null (local.get $v)) (then (call $enc_u8 (i32.const 0)) (return)))
  (if (ref.test (ref i31) (local.get $v)) (then
    (call $enc_u8 (i32.const 1))
    (call $enc_i32 (i31.get_s (ref.cast (ref i31) (local.get $v))))
    (return)))
  (if (i32.eqz (ref.test (ref $val) (local.get $v))) (then (global.set $enc_fail (i32.const 1)) (return)))
  (local.set $raw (struct.get $val 0 (ref.cast (ref $val) (local.get $v))))
  (if (i32.lt_s (local.get $raw) (i32.const 0)) (then
    (call $enc_u8 (i32.const 2))
    (call $enc_i32 (i32.and (local.get $raw) (i32.const 2147483647)))
    (return)))
  (local.set $tag (local.get $raw))
  (if (i32.or (i32.and (i32.ge_u (local.get $tag) (i32.const 10)) (i32.le_u (local.get $tag) (i32.const 12)))
              (i32.ge_u (local.get $tag) (i32.const 17)))
    (then (global.set $enc_fail (i32.const 1)) (return)))
  (if (i32.eq (local.get $tag) (i32.const 1)) (then
    (call $enc_u8 (select (i32.const 6) (i32.const 7) (call $bool_get (local.get $v))))
    (return)))
  (if (i32.eq (local.get $tag) (i32.const 6)) (then
    (if (call $list_is_empty (local.get $v)) (then (call $enc_u8 (i32.const 8)) (return)))
    (call $enc_mark (local.get $v) (local.get $tag))
    (local.set $w (local.get $v))
    (block $counted (loop $count
      (br_if $counted (call $list_is_empty (local.get $w)))
      (local.set $n (i32.add (local.get $n) (i32.const 1)))
      (local.set $w (call $list_tail (local.get $w)))
      (br $count)))
    (call $enc_u8 (i32.const 5))
    (call $enc_i32 (local.get $n))
    (local.set $w (local.get $v))
    (block $written (loop $item
      (br_if $written (call $list_is_empty (local.get $w)))
      (call $enc_value (call $list_head (local.get $w)) (i32.add (local.get $depth) (i32.const 1)))
      (local.set $w (call $list_tail (local.get $w)))
      (br $item)))
    (return)))
  (local.set $o (ref.cast (ref $obj) (local.get $v)))
  (local.set $xs (struct.get $obj 5 (local.get $o)))
  (if (i32.eq (local.get $tag) (i32.const 7)) (then
    (local.set $code (i32.trunc_f64_u (struct.get $obj 3 (local.get $o))))
    (if (i32.eqz (local.get $code)) (then (global.set $enc_fail (i32.const 1)) (return)))
    (call $enc_mark (local.get $v) (local.get $tag))
    (call $enc_u8 (i32.const 4))
    (call $enc_i32 (local.get $code))
    (call $enc_i32 (struct.get $obj 1 (local.get $o)))
    (call $enc_i32 (i32.wrap_i64 (struct.get $obj 2 (local.get $o))))
    (if (ref.is_null (local.get $xs)) (then (call $enc_i32 (i32.const -1)) (return)))
    (local.set $n (array.len (local.get $xs)))
    ;; A patched closure keeps its walk stamp after the captures.
    (if (i32.and (struct.get $obj 1 (local.get $o)) (i32.const 1))
      (then (local.set $n (i32.sub (local.get $n) (i32.const 1)))))
    (call $enc_i32 (local.get $n))
    (block $done (loop $each
      (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
      (call $enc_value (array.get $values (local.get $xs) (local.get $i)) (i32.add (local.get $depth) (i32.const 1)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $each)))
    (return)))
  (if (i32.eqz (ref.is_null (struct.get $obj 7 (local.get $o))))
    (then (global.set $enc_fail (i32.const 1)) (return)))
  (call $enc_mark (local.get $v) (local.get $tag))
  (call $enc_u8 (i32.const 3))
  (call $enc_i32 (local.get $tag))
  (call $enc_i32 (struct.get $obj 1 (local.get $o)))
  (call $enc_i64 (struct.get $obj 2 (local.get $o)))
  (call $enc_f64 (struct.get $obj 3 (local.get $o)))
  (local.set $b (struct.get $obj 4 (local.get $o)))
  (if (ref.is_null (local.get $b))
    (then (call $enc_i32 (i32.const -1)))
    (else
      (local.set $n (array.len (local.get $b)))
      (call $enc_i32 (local.get $n))
      (call $enc_reserve (local.get $n))
      (local.set $i (i32.const 0))
      (block $done (loop $each
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (i32.store8 (i32.add (i32.add (global.get $enc_ptr) (global.get $enc_len)) (local.get $i))
          (array.get_u $bytes (local.get $b) (local.get $i)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $each)))
      (global.set $enc_len (i32.add (global.get $enc_len) (local.get $n)))))
  (local.set $l (struct.get $obj 6 (local.get $o)))
  (if (ref.is_null (local.get $l))
    (then (call $enc_i32 (i32.const -1)))
    (else
      (local.set $n (array.len (local.get $l)))
      (call $enc_i32 (local.get $n))
      (local.set $i (i32.const 0))
      (block $done (loop $each
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (call $enc_i32 (array.get $limbs (local.get $l) (local.get $i)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $each)))))
  (if (ref.is_null (local.get $xs))
    (then (call $enc_i32 (i32.const -1)))
    (else
      (local.set $n (array.len (local.get $xs)))
      (call $enc_i32 (local.get $n))
      (local.set $i (i32.const 0))
      (block $done (loop $each
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (call $enc_value (array.get $values (local.get $xs) (local.get $i)) (i32.add (local.get $depth) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $each))))))

;; Encode `v` into a new buffer at `$enc_ptr`/`$enc_len` (capacity
;; `$enc_cap`). Gives 0 when `v` cannot move; the buffer is then freed.
(func $codec_encode (param $v eqref) (result i32)
  (local $i i32)
  (global.set $enc_ptr (i32.const 0))
  (global.set $enc_len (i32.const 0))
  (global.set $enc_cap (i32.const 0))
  (global.set $enc_fail (i32.const 0))
  (global.set $enc_count (i32.const 0))
  (global.set $enc_seen (array.new_default $values (i32.const 32)))
  (global.set $enc_tags (array.new_default $frames (i32.const 32)))
  (call $enc_value (local.get $v) (i32.const 0))
  ;; Restore every header, also after a failure.
  (block $done (loop $each
    (br_if $done (i32.ge_u (local.get $i) (global.get $enc_count)))
    (struct.set $val 0
      (ref.cast (ref $val) (array.get $values (ref.as_non_null (global.get $enc_seen)) (local.get $i)))
      (array.get $frames (ref.as_non_null (global.get $enc_tags)) (local.get $i)))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $each)))
  (global.set $enc_seen (ref.null $values))
  (global.set $enc_tags (ref.null $frames))
  (if (global.get $enc_fail) (then
    (if (global.get $enc_cap) (then (call $free (global.get $enc_ptr) (global.get $enc_cap))))
    (global.set $enc_cap (i32.const 0))
    (return (i32.const 0))))
  (i32.const 1))

(func $dec_u8 (result i32)
  (local $at i32)
  (local.set $at (global.get $dec_at))
  (if (i32.ge_u (local.get $at) (global.get $dec_end)) (then unreachable))
  (global.set $dec_at (i32.add (local.get $at) (i32.const 1)))
  (i32.load8_u (local.get $at)))
(func $dec_i32 (result i32)
  (local $at i32)
  (local.set $at (global.get $dec_at))
  (if (i32.gt_u (i32.add (local.get $at) (i32.const 4)) (global.get $dec_end)) (then unreachable))
  (global.set $dec_at (i32.add (local.get $at) (i32.const 4)))
  (i32.load (local.get $at)))
(func $dec_i64 (result i64)
  (local $at i32)
  (local.set $at (global.get $dec_at))
  (if (i32.gt_u (i32.add (local.get $at) (i32.const 8)) (global.get $dec_end)) (then unreachable))
  (global.set $dec_at (i32.add (local.get $at) (i32.const 8)))
  (i64.load (local.get $at)))
(func $dec_f64 (result f64)
  (local $at i32)
  (local.set $at (global.get $dec_at))
  (if (i32.gt_u (i32.add (local.get $at) (i32.const 8)) (global.get $dec_end)) (then unreachable))
  (global.set $dec_at (i32.add (local.get $at) (i32.const 8)))
  (f64.load (local.get $at)))

;; The index of the next decoded aggregate, with `v` as its value.
(func $dec_register (param $v eqref) (result i32)
  (local $grown (ref null $values)) (local $n i32) (local $index i32)
  (local.set $n (array.len (ref.as_non_null (global.get $dec_table))))
  (if (i32.eq (global.get $dec_count) (local.get $n)) (then
    (local.set $grown (array.new_default $values (i32.shl (local.get $n) (i32.const 1))))
    (array.copy $values $values (ref.as_non_null (local.get $grown)) (i32.const 0)
      (ref.as_non_null (global.get $dec_table)) (i32.const 0) (local.get $n))
    (global.set $dec_table (local.get $grown))))
  (local.set $index (global.get $dec_count))
  (array.set $values (ref.as_non_null (global.get $dec_table)) (local.get $index) (local.get $v))
  (global.set $dec_count (i32.add (local.get $index) (i32.const 1)))
  (local.get $index))

(func $dec_value (result eqref)
  (local $m i32) (local $n i32) (local $i i32) (local $index i32) (local $tag i32)
  (local $f1 i32) (local $f2 i64) (local $f3 f64) (local $code i32) (local $flags i32) (local $arity i32)
  (local $xs (ref null $values)) (local $b (ref null $bytes)) (local $l (ref null $limbs))
  (local $v eqref) (local $out eqref)
  (local.set $m (call $dec_u8))
  (if (i32.eqz (local.get $m)) (then (return (ref.null eq))))
  (if (i32.eq (local.get $m) (i32.const 1)) (then (return (ref.i31 (call $dec_i32)))))
  (if (i32.eq (local.get $m) (i32.const 2)) (then
    (local.set $index (call $dec_i32))
    (if (i32.ge_u (local.get $index) (global.get $dec_count)) (then unreachable))
    (return (array.get $values (ref.as_non_null (global.get $dec_table)) (local.get $index)))))
  (if (i32.eq (local.get $m) (i32.const 6)) (then (return (global.get $value_true))))
  (if (i32.eq (local.get $m) (i32.const 7)) (then (return (global.get $value_false))))
  (if (i32.eq (local.get $m) (i32.const 8)) (then (return (global.get $value_empty_list))))
  (if (i32.eq (local.get $m) (i32.const 5)) (then
    (local.set $index (call $dec_register (ref.null eq)))
    (local.set $n (call $dec_i32))
    (local.set $xs (array.new_default $values (local.get $n)))
    (block $done (loop $each
      (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
      (array.set $values (local.get $xs) (local.get $i) (call $dec_value))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $each)))
    (local.set $out (global.get $value_empty_list))
    (block $built (loop $cons
      (br_if $built (i32.eqz (local.get $i)))
      (local.set $i (i32.sub (local.get $i) (i32.const 1)))
      (local.set $out (call $list_cons (array.get $values (local.get $xs) (local.get $i)) (local.get $out)))
      (br $cons)))
    (array.set $values (ref.as_non_null (global.get $dec_table)) (local.get $index) (local.get $out))
    (return (local.get $out))))
  (if (i32.eq (local.get $m) (i32.const 4)) (then
    (local.set $code (call $dec_i32))
    (local.set $flags (call $dec_i32))
    (local.set $arity (call $dec_i32))
    (local.set $n (call $dec_i32))
    (if (i32.ge_s (local.get $n) (i32.const 0))
      (then (local.set $xs (array.new_default $values
        (i32.add (local.get $n) (i32.and (local.get $flags) (i32.const 1)))))))
    (local.set $v (call $codec_make_closure (local.get $arity) (local.get $code) (local.get $flags) (local.get $xs)))
    (drop (call $dec_register (local.get $v)))
    (block $done (loop $each
      (br_if $done (i32.ge_s (local.get $i) (local.get $n)))
      (array.set $values (ref.as_non_null (local.get $xs)) (local.get $i) (call $dec_value))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $each)))
    (return (local.get $v))))
  (if (i32.ne (local.get $m) (i32.const 3)) (then unreachable))
  (local.set $tag (call $dec_i32))
  (local.set $f1 (call $dec_i32))
  (local.set $f2 (call $dec_i64))
  (local.set $f3 (call $dec_f64))
  (local.set $n (call $dec_i32))
  (if (i32.ge_s (local.get $n) (i32.const 0)) (then
    (if (i32.gt_u (i32.add (global.get $dec_at) (local.get $n)) (global.get $dec_end)) (then unreachable))
    (local.set $b (array.new_default $bytes (local.get $n)))
    (block $done (loop $each
      (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
      (array.set $bytes (local.get $b) (local.get $i)
        (i32.load8_u (i32.add (global.get $dec_at) (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $each)))
    (global.set $dec_at (i32.add (global.get $dec_at) (local.get $n)))))
  (local.set $n (call $dec_i32))
  (if (i32.ge_s (local.get $n) (i32.const 0)) (then
    (local.set $l (array.new_default $limbs (local.get $n)))
    (local.set $i (i32.const 0))
    (block $done (loop $each
      (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
      (array.set $limbs (local.get $l) (local.get $i) (call $dec_i32))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $each)))))
  (local.set $n (call $dec_i32))
  (if (i32.ge_s (local.get $n) (i32.const 0))
    (then (local.set $xs (array.new_default $values (local.get $n)))))
  (local.set $v (struct.new $obj (local.get $tag) (local.get $f1) (local.get $f2) (local.get $f3)
    (local.get $b) (local.get $xs) (local.get $l) (ref.null $call)))
  (drop (call $dec_register (local.get $v)))
  (local.set $i (i32.const 0))
  (block $done (loop $each
    (br_if $done (i32.ge_s (local.get $i) (local.get $n)))
    (array.set $values (ref.as_non_null (local.get $xs)) (local.get $i) (call $dec_value))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $each)))
  (local.get $v))

(func $codec_decode (param $ptr i32) (param $len i32) (result eqref)
  (local $v eqref)
  (global.set $dec_at (local.get $ptr))
  (global.set $dec_end (i32.add (local.get $ptr) (local.get $len)))
  (global.set $dec_table (array.new_default $values (i32.const 32)))
  (global.set $dec_count (i32.const 0))
  (local.set $v (call $dec_value))
  (if (i32.ne (global.get $dec_at) (global.get $dec_end)) (then unreachable))
  (global.set $dec_table (ref.null $values))
  (local.get $v))

;; The operations `ilang:worker` of `i/task`.
(func $worker_enabled (result eqref)
  (call $bool_new (global.get $mc_enabled)))
(func $worker_publish (param $job eqref) (result eqref)
  (local $id i64)
  (if (i32.eqz (global.get $mc_enabled)) (then (return (ref.i31 (i32.const 0)))))
  (if (i32.eqz (call $codec_encode (local.get $job))) (then (return (ref.i31 (i32.const 0)))))
  (local.set $id (call $host_mc_publish (global.get $enc_ptr) (global.get $enc_len)))
  (call $free (global.get $enc_ptr) (global.get $enc_cap))
  (global.set $enc_cap (i32.const 0))
  (call $int_from_i64 (local.get $id)))
(func $worker_claim (param $id eqref) (result eqref)
  (call $bool_new (call $host_mc_claim (call $int_to_i64 (local.get $id)))))
(func $worker_poll (param $id eqref) (result eqref)
  (ref.i31 (call $host_mc_poll (call $int_to_i64 (local.get $id)))))
(func $worker_take (param $id eqref) (result eqref)
  (local $r i64) (local $p i32) (local $n i32) (local $v eqref)
  (local.set $r (call $host_mc_take (call $int_to_i64 (local.get $id))))
  (local.set $p (i32.wrap_i64 (i64.shr_u (local.get $r) (i64.const 32))))
  (local.set $n (i32.wrap_i64 (local.get $r)))
  (local.set $v (call $codec_decode (local.get $p) (local.get $n)))
  (call $free (local.get $p) (local.get $n))
  (local.get $v))
(func $worker_forget (param $id eqref) (result eqref)
  (call $host_mc_forget (call $int_to_i64 (local.get $id)))
  (ref.null eq))
;; `scope.share` keeps the encoded value once in the host transport, under
;; (run key id, task, index) of its address, until its scope ends.
(func $worker_key_id (param $k eqref) (result i64)
  (struct.get $obj 2 (ref.cast (ref $obj) (local.get $k))))
(func $worker_share_put (param $run eqref) (param $address eqref) (param $value eqref) (result eqref)
  (if (i32.eqz (global.get $mc_enabled)) (then (return (ref.null eq))))
  (if (i32.eqz (call $codec_encode (local.get $value))) (then (return (ref.null eq))))
  (call $host_mc_share_put (call $worker_key_id (local.get $run))
    (call $int_to_i64 (call $field (local.get $address) (i32.const 0)))
    (call $int_to_i64 (call $field (local.get $address) (i32.const 1)))
    (global.get $enc_ptr) (global.get $enc_len))
  (call $free (global.get $enc_ptr) (global.get $enc_cap))
  (global.set $enc_cap (i32.const 0))
  (ref.null eq))
;; [value] when the shared value is there, else [].
(func $worker_share_get (param $run eqref) (param $address eqref) (result eqref)
  (local $r i64) (local $p i32) (local $n i32) (local $v eqref)
  (local.set $r (call $host_mc_share_get (call $worker_key_id (local.get $run))
    (call $int_to_i64 (call $field (local.get $address) (i32.const 0)))
    (call $int_to_i64 (call $field (local.get $address) (i32.const 1)))))
  (if (i64.eqz (local.get $r)) (then (return (global.get $value_empty_list))))
  (local.set $p (i32.wrap_i64 (i64.shr_u (local.get $r) (i64.const 32))))
  (local.set $n (i32.wrap_i64 (local.get $r)))
  (local.set $v (call $codec_decode (local.get $p) (local.get $n)))
  (call $free (local.get $p) (local.get $n))
  (call $list_cons (local.get $v) (global.get $value_empty_list)))
(func $worker_share_drop (param $run eqref) (param $address eqref) (result eqref)
  (if (global.get $mc_enabled) (then
    (call $host_mc_share_drop (call $worker_key_id (local.get $run))
      (call $int_to_i64 (call $field (local.get $address) (i32.const 0)))
      (call $int_to_i64 (call $field (local.get $address) (i32.const 1))))))
  (ref.null eq))
;; A speculation reached a scheduler step: the worker gives the record back.
(func $worker_abort (result eqref)
  unreachable)

;; The host makes this instance a worker: new keys take ids above `prefix`.
(func (export "ilang_mc_setup") (param $prefix i64)
  (global.set $mc_enabled (i32.const 1))
  (global.set $id_prefix (local.get $prefix)))
;; Run a claimed record `(start, scheduler)`. 1: the encoded completion is at
;; `ilang_mc_out`; 2: the record goes back to its maker.
(func (export "ilang_mc_run") (param $ptr i32) (param $len i32) (result i32)
  (local $job eqref) (local $r eqref) (local $depth i32)
  (local.set $job (call $codec_decode (local.get $ptr) (local.get $len)))
  (local.set $depth (call $shadow_depth))
  (block $failed
    (local.set $r (try_table (result eqref) (catch_all $failed)
      (call $call1 (call $field (local.get $job) (i32.const 0)) (call $field (local.get $job) (i32.const 1))
        (global.get $root_env))))
    (if (i32.eqz (call $codec_encode (local.get $r))) (then (return (i32.const 2))))
    (global.set $mc_out_ptr (global.get $enc_ptr))
    (global.set $mc_out_len (global.get $enc_len))
    (global.set $mc_out_cap (global.get $enc_cap))
    (global.set $enc_cap (i32.const 0))
    (return (i32.const 1)))
  (call $shadow_restore (local.get $depth))
  (i32.const 2))
(func (export "ilang_mc_out_ptr") (result i32) (global.get $mc_out_ptr))
(func (export "ilang_mc_out_len") (result i32) (global.get $mc_out_len))
(func (export "ilang_mc_out_cap") (result i32) (global.get $mc_out_cap))
"#;

/// `$codec_make_closure`: a closure of one of the arities of the module from
/// its code number, and the code table. Code 1 is `$keep_call` (D4), 2 the
/// runtime's `Assert` function, and `3 + i` the function `$f{i}`.
pub(super) fn closures(arities: &std::collections::BTreeSet<u32>, funcs: usize) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let mut names = vec!["$keep_call".to_string(), "$entry_assert_fail".to_string()];
    names.extend((0..funcs).map(|i| format!("$f{i}")));
    let _ = writeln!(
        out,
        "(table $codes {} funcref)\n(elem (table $codes) (offset (i32.const 1)) func {})",
        names.len() + 1,
        names.join(" ")
    );
    out.push_str(
        "(func $codec_make_closure (param $arity i32) (param $code i32) (param $flags i32) \
         (param $caps (ref null $values)) (result eqref)\n",
    );
    for n in arities {
        let _ = writeln!(
            out,
            "  (if (i32.eq (local.get $arity) (i32.const {n})) (then (return (struct.new $clo{n} (i32.const 7) \
             (local.get $flags) (i64.const {n}) (f64.convert_i32_u (local.get $code)) (ref.null $bytes) \
             (local.get $caps) (ref.null $limbs) (ref.null $call) \
             (ref.cast (ref $fn{n}) (table.get $codes (local.get $code)))))))"
        );
    }
    out.push_str("  unreachable)\n");
    out
}
