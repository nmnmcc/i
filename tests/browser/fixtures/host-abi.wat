;; Host ABI v2 fixture, written by hand: not compiler output. It tests the
;; Rust and JavaScript hosts against Appendix B of docs/spec.md. Its allocator
;; only counts live buffers, so tests can check every free of B.3.
(module
  (type $box (struct (field i32)))
  (type $unary (func (param i32) (result i32)))
  (tag $fail (param i32))

  (import "ilang:host/v1" "panic" (func $panic (param i32 i32)))
  (import "ilang:host/v1" "reference_id" (func $reference_id (param externref) (result i64)))
  ;; Host services (spec §9.7): print(text: String) -> Nil,
  ;; read(key: String) -> String ? String, and the async later(x: Float) -> Float ? String.
  (import "app:console/v1" "print" (func $print (param i32 i32)))
  (import "app:storage/v1" "read" (func $read (param i32 i32) (result i32 i32 i32 i32 i32)))
  (import "app:clock/v1" "later" (func $later (param f64)))
  ;; An external function (spec §10.1): hypot(Float, Float) -> Float.
  (import "app:math/v1" "hypot" (func $hypot (param f64 f64) (result f64)))

  (memory (export "memory") 1 256)
  (global $next (mut i32) (i32.const 4096))
  (global $live (mut i32) (i32.const 0))
  (global $bytes (mut i32) (i32.const 0))
  (global $ready (mut i32) (i32.const 0))
  (global $test_runs (mut i32) (i32.const 0))
  (data (i32.const 128) "fixture main\0a")
  (data (i32.const 160) "assertion failed: fixture\0aapp/test:3:5")
  (data (i32.const 208) "start\0a")
  (data (i32.const 224) "ok")
  (data (i32.const 232) "bad")
  (data (i32.const 240) "\ff\fe")

  ;; B.1: the host calls this once, before every other export but the allocator.
  ;; Imports may be called here.
  (func (export "ilang_start")
    (if (global.get $ready) (then unreachable))
    (call $print_static (i32.const 208) (i32.const 6))
    (global.set $ready (i32.const 1)))
  (func (export "initialized") (result i32) (global.get $ready))
  (func (export "live_allocations") (result i32) (global.get $live))
  (func (export "live_bytes") (result i32) (global.get $bytes))

  ;; Size 0 gives address 0 (B.1), which is never freed.
  (func $alloc (export "ilang_alloc") (param $len i32) (result i32)
    (local $ptr i32) (local $end i32) (local $pages i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (if (i32.lt_s (local.get $len) (i32.const 0)) (then unreachable))
    (local.set $ptr (global.get $next))
    (local.set $end
      (i32.and (i32.add (i32.add (local.get $ptr) (local.get $len)) (i32.const 7)) (i32.const -8)))
    (if (i32.lt_u (local.get $end) (local.get $ptr)) (then unreachable))
    (local.set $pages (i32.shr_u (i32.add (local.get $end) (i32.const 65535)) (i32.const 16)))
    (if (i32.gt_u (local.get $pages) (memory.size))
      (then
        (if (i32.eq (memory.grow (i32.sub (local.get $pages) (memory.size))) (i32.const -1))
          (then unreachable))))
    (global.set $next (local.get $end))
    (global.set $live (i32.add (global.get $live) (i32.const 1)))
    (global.set $bytes (i32.add (global.get $bytes) (local.get $len)))
    (local.get $ptr))

  (func $free (export "ilang_free") (param $ptr i32) (param $len i32)
    (if (i32.eqz (global.get $live)) (then unreachable))
    (if (i32.lt_u (local.get $ptr) (i32.const 4096)) (then unreachable))
    (global.set $live (i32.sub (global.get $live) (i32.const 1)))
    (global.set $bytes (i32.sub (global.get $bytes) (local.get $len))))

  (func $copy (param $ptr i32) (param $len i32) (result i32)
    (local $out i32)
    (local.set $out (call $alloc (local.get $len)))
    (memory.copy (local.get $out) (local.get $ptr) (local.get $len))
    (local.get $out))

  ;; The module makes and frees the parameter buffers of an import (B.3).
  (func $print_static (param $ptr i32) (param $len i32)
    (local $own i32)
    (local.set $own (call $copy (local.get $ptr) (local.get $len)))
    (call $print (local.get $own) (local.get $len))
    (if (local.get $len) (then (call $free (local.get $own) (local.get $len)))))

  ;; Results are new buffers that belong to the host.
  (func (export "string_roundtrip") (param $ptr i32) (param $len i32) (result i32 i32)
    (call $copy (local.get $ptr) (local.get $len))
    (local.get $len))
  (func (export "int_roundtrip")
    (param $sign i32) (param $ptr i32) (param $limbs i32) (result i32 i32 i32)
    (local.get $sign)
    (call $copy (local.get $ptr) (i32.mul (local.get $limbs) (i32.const 4)))
    (local.get $limbs))
  (func (export "bit_array_roundtrip") (param $ptr i32) (param $bits i32) (result i32 i32)
    (call $copy (local.get $ptr)
      (i32.wrap_i64 (i64.shr_u (i64.add (i64.extend_i32_u (local.get $bits)) (i64.const 7)) (i64.const 3))))
    (local.get $bits))
  (func (export "float_roundtrip") (param $value f64) (result f64) (local.get $value))
  (func (export "bool_roundtrip") (param $value i32) (result i32) (local.get $value))
  (func (export "nil_roundtrip"))
  (func (export "opaque_roundtrip") (param $value externref) (result externref) (local.get $value))
  (func (export "same_reference") (param $a externref) (param $b externref) (result i32)
    (i64.eq (call $reference_id (local.get $a)) (call $reference_id (local.get $b))))

  ;; (selector: Float) -> String ? Nil | Bool | Int | Float | String | BitArray.
  ;; Results: tag, the String, then every error member in canonical order.
  (func (export "choose") (param $selector f64)
    (result i32 i32 i32 i32 i32 i32 i32 f64 i32 i32 i32 i32)
    (local $s i32) (local $tag i32) (local $ok_ptr i32) (local $ok_len i32)
    (local $bool i32) (local $sign i32) (local $int_ptr i32) (local $count i32)
    (local $float f64) (local $str_ptr i32) (local $str_len i32)
    (local $bits_ptr i32) (local $bits i32)
    (local.set $s (i32.trunc_f64_s (local.get $selector)))
    (local.set $tag (local.get $s))
    (if (i32.eqz (local.get $s))
      (then
        (local.set $ok_ptr (call $copy (i32.const 224) (i32.const 2)))
        (local.set $ok_len (i32.const 2))))
    (if (i32.eq (local.get $s) (i32.const 2)) (then (local.set $bool (i32.const 1))))
    (if (i32.eq (local.get $s) (i32.const 3))
      (then
        (local.set $sign (i32.const -1))
        (local.set $int_ptr (call $alloc (i32.const 4)))
        (i32.store (local.get $int_ptr) (i32.const 7))
        (local.set $count (i32.const 1))))
    (if (i32.eq (local.get $s) (i32.const 4)) (then (local.set $float (f64.const 2.5))))
    (if (i32.eq (local.get $s) (i32.const 5))
      (then
        (local.set $str_ptr (call $copy (i32.const 232) (i32.const 3)))
        (local.set $str_len (i32.const 3))))
    (if (i32.eq (local.get $s) (i32.const 6))
      (then
        (local.set $bits_ptr (call $alloc (i32.const 1)))
        (i32.store8 (local.get $bits_ptr) (i32.const 160))
        (local.set $bits (i32.const 3))))
    (local.get $tag)
    (local.get $ok_ptr) (local.get $ok_len)
    (local.get $bool)
    (local.get $sign) (local.get $int_ptr) (local.get $count)
    (local.get $float)
    (local.get $str_ptr) (local.get $str_len)
    (local.get $bits_ptr) (local.get $bits))

  ;; (key: String) -> String ? String, through the service read. The result
  ;; buffers of the import belong to the module, which gives them to the host.
  (func (export "read_through") (param $ptr i32) (param $len i32) (result i32 i32 i32 i32 i32)
    (local $own i32) (local $tag i32) (local $a i32) (local $b i32) (local $c i32) (local $d i32)
    (local.set $own (call $copy (local.get $ptr) (local.get $len)))
    (call $read (local.get $own) (local.get $len))
    (local.set $d) (local.set $c) (local.set $b) (local.set $a) (local.set $tag)
    (if (local.get $len) (then (call $free (local.get $own) (local.get $len))))
    (local.get $tag) (local.get $a) (local.get $b) (local.get $c) (local.get $d))

  (func (export "hypot") (param $a f64) (param $b f64) (result f64)
    (call $hypot (local.get $a) (local.get $b)))
  (func (export "print") (param $ptr i32) (param $len i32)
    (call $print_static (local.get $ptr) (local.get $len)))

  ;; A stop (B.5): the message goes to panic, then the module traps.
  (func (export "stop") (param $ptr i32) (param $len i32)
    (call $panic (call $copy (local.get $ptr) (local.get $len)) (local.get $len))
    unreachable)

  ;; Exceptions stay inside the module (B.1).
  (func (export "exceptions") (result f64)
    (block $caught (result i32)
      (try_table (catch $fail $caught) (throw $fail (i32.const 41)))
      (return (f64.const 0)))
    (f64.convert_i32_s (i32.add (i32.const 1))))

  ;; Values that are not correct (B.2).
  (func (export "bad_bool") (result i32) (i32.const 2))
  (func (export "bad_utf8") (result i32 i32)
    (call $copy (i32.const 240) (i32.const 2)) (i32.const 2))
  (func (export "bad_tag") (result i32 i32 i32) (i32.const 5) (i32.const 0) (i32.const 0))

  ;; An async host export (B.5): later(x) twice, the second time with the
  ;; first value plus one. An error ends the waiting at once.
  (global $waits (mut i32) (i32.const 0))
  (global $later_tag (mut i32) (i32.const 0))
  (global $later_value (mut f64) (f64.const 0))
  (global $later_ptr (mut i32) (i32.const 0))
  (global $later_len (mut i32) (i32.const 0))
  (func (export "app/async.later") (param $x f64) (result i32)
    (global.set $waits (i32.const 0))
    (call $later (local.get $x))
    (i32.const 1))
  (func (export "ilang_resume:app:clock/v1:later")
    (param $tag i32) (param $value f64) (param $ptr i32) (param $len i32) (result i32)
    (if (global.get $deep_after_resume)
      (then
        (global.set $deep_after_resume (i32.const 0))
        (global.set $deep_result (call $deep (global.get $deep_n)))
        (return (i32.const 0))))
    (global.set $waits (i32.add (global.get $waits) (i32.const 1)))
    (if (i32.and (i32.eqz (local.get $tag)) (i32.eq (global.get $waits) (i32.const 1)))
      (then
        (call $later (f64.add (local.get $value) (f64.const 1)))
        (return (i32.const 1))))
    (global.set $later_tag (local.get $tag))
    (global.set $later_value (local.get $value))
    (global.set $later_ptr (local.get $ptr))
    (global.set $later_len (local.get $len))
    (i32.const 0))
  (func (export "ilang_result:app/async.later") (result i32 f64 i32 i32)
    (global.get $later_tag) (global.get $later_value)
    (global.get $later_ptr) (global.get $later_len))

  ;; Recursion that is not a tail call, n deep: in a sync call, and in the
  ;; resume of an async call (after later(1.0) is answered).
  (global $deep_after_resume (mut i32) (i32.const 0))
  (global $deep_n (mut i32) (i32.const 0))
  (global $deep_result (mut i32) (i32.const 0))
  (func $deep (param $n i32) (result i32)
    (if (result i32) (i32.eqz (local.get $n))
      (then (i32.const 0))
      (else (i32.add (call $deep (i32.sub (local.get $n) (i32.const 1))) (i32.const 1)))))
  (func (export "app/deep") (param $n f64) (result f64)
    (f64.convert_i32_s (call $deep (i32.trunc_f64_s (local.get $n)))))
  (func (export "app/async.deep") (param $n f64) (result i32)
    (global.set $deep_n (i32.trunc_f64_s (local.get $n)))
    (global.set $deep_after_resume (i32.const 1))
    (call $later (f64.const 1))
    (i32.const 1))
  (func (export "ilang_result:app/async.deep") (result f64)
    (f64.convert_i32_s (global.get $deep_result)))

  ;; The program entry and the tests have no parameters and no results.
  (func (export "app/main.main")
    (call $print_static (i32.const 128) (i32.const 13)))
  (func (export "app/test.passing")
    (if (i32.ne (global.get $ready) (i32.const 1)) (then unreachable)))
  (func (export "app/test.failing")
    (call $panic (i32.const 160) (i32.const 38))
    unreachable)
  (func (export "app/test.trapping") unreachable)
  (func (export "app/test.fresh")
    (if (global.get $test_runs) (then unreachable))
    (global.set $test_runs (i32.const 1)))
  ;; An async test: with a host that answers x, later(1.0) ends with 2.0.
  (func (export "app/async_test.waits") (result i32)
    (global.set $waits (i32.const 0))
    (call $later (f64.const 1))
    (i32.const 1))
  (func (export "ilang_result:app/async_test.waits")
    (if (i32.or (global.get $later_tag) (f64.ne (global.get $later_value) (f64.const 2)))
      (then unreachable)))

  ;; Features that the host must enable (B.1).
  (func $increment (type $unary) (param $value i32) (result i32)
    (i32.add (local.get $value) (i32.const 1)))
  (elem declare func $increment)
  (func (export "feature_probe") (result i32)
    (struct.get $box 0 (struct.new $box (i32.const 41)))
    (ref.func $increment)
    (call_ref $unary))
  (func $tail_count (export "tail_count") (param $n i32) (param $acc i32) (result i32)
    (if (result i32) (i32.eqz (local.get $n))
      (then (local.get $acc))
      (else
        (return_call $tail_count
          (i32.sub (local.get $n) (i32.const 1))
          (i32.add (local.get $acc) (i32.const 1))))))
  (func (export "gc_sum") (param $n i32) (result i32)
    (local $sum i32)
    (loop $again
      (if (local.get $n)
        (then
          (local.set $sum
            (i32.add (local.get $sum) (struct.get $box 0 (struct.new $box (local.get $n)))))
          (local.set $n (i32.sub (local.get $n) (i32.const 1)))
          (br $again))))
    (local.get $sum))

  (@custom "ilang.abi"
    "{\"version\":2,"
    "\"entry\":{\"name\":\"app/main.main\",\"async\":false},"
    "\"tests\":["
      "{\"name\":\"app/test.passing\",\"async\":false},"
      "{\"name\":\"app/test.failing\",\"async\":false},"
      "{\"name\":\"app/test.trapping\",\"async\":false},"
      "{\"name\":\"app/test.fresh\",\"async\":false},"
      "{\"name\":\"app/test.fresh\",\"async\":false},"
      "{\"name\":\"app/async_test.waits\",\"async\":true}],"
    "\"exports\":{"
      "\"string_roundtrip\":{\"params\":[\"string\"],\"result\":\"string\",\"errors\":[],\"async\":false},"
      "\"int_roundtrip\":{\"params\":[\"int\"],\"result\":\"int\",\"errors\":[],\"async\":false},"
      "\"bit_array_roundtrip\":{\"params\":[\"bit_array\"],\"result\":\"bit_array\",\"errors\":[],\"async\":false},"
      "\"float_roundtrip\":{\"params\":[\"float\"],\"result\":\"float\",\"errors\":[],\"async\":false},"
      "\"bool_roundtrip\":{\"params\":[\"bool\"],\"result\":\"bool\",\"errors\":[],\"async\":false},"
      "\"nil_roundtrip\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":false},"
      "\"opaque_roundtrip\":{\"params\":[\"externref\"],\"result\":\"externref\",\"errors\":[],\"async\":false},"
      "\"same_reference\":{\"params\":[\"externref\",\"externref\"],\"result\":\"bool\",\"errors\":[],\"async\":false},"
      "\"choose\":{\"params\":[\"float\"],\"result\":\"string\","
        "\"errors\":[\"nil\",\"bool\",\"int\",\"float\",\"string\",\"bit_array\"],\"async\":false},"
      "\"read_through\":{\"params\":[\"string\"],\"result\":\"string\",\"errors\":[\"string\"],\"async\":false},"
      "\"hypot\":{\"params\":[\"float\",\"float\"],\"result\":\"float\",\"errors\":[],\"async\":false},"
      "\"print\":{\"params\":[\"string\"],\"result\":\"nil\",\"errors\":[],\"async\":false},"
      "\"stop\":{\"params\":[\"string\"],\"result\":\"nil\",\"errors\":[],\"async\":false},"
      "\"exceptions\":{\"params\":[],\"result\":\"float\",\"errors\":[],\"async\":false},"
      "\"bad_bool\":{\"params\":[],\"result\":\"bool\",\"errors\":[],\"async\":false},"
      "\"bad_utf8\":{\"params\":[],\"result\":\"string\",\"errors\":[],\"async\":false},"
      "\"bad_tag\":{\"params\":[],\"result\":\"nil\",\"errors\":[\"string\"],\"async\":false},"
      "\"app/async.later\":{\"params\":[\"float\"],\"result\":\"float\",\"errors\":[\"string\"],\"async\":true},"
      "\"app/deep\":{\"params\":[\"float\"],\"result\":\"float\",\"errors\":[],\"async\":false},"
      "\"app/async.deep\":{\"params\":[\"float\"],\"result\":\"float\",\"errors\":[],\"async\":true}},"
    "\"services\":{"
      "\"app:console/v1\":{\"print\":{\"params\":[\"string\"],\"result\":\"nil\",\"errors\":[],\"async\":false}},"
      "\"app:storage/v1\":{\"read\":{\"params\":[\"string\"],\"result\":\"string\",\"errors\":[\"string\"],\"async\":false}},"
      "\"app:clock/v1\":{\"later\":{\"params\":[\"float\"],\"result\":\"float\",\"errors\":[\"string\"],\"async\":true}}},"
    "\"externals\":{\"app:math/v1\":{\"hypot\":{\"params\":[\"float\",\"float\"],\"result\":\"float\"}}}}")
)
