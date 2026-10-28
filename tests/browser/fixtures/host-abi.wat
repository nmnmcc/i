;; Host ABI fixture, not compiler output. Its allocator intentionally only counts
;; live allocations: production allocator behavior is tested using compiler output.
(module
  (type $box (struct (field i32)))
  (type $unary (func (param i32) (result i32)))
  (type $abi_frame
    (struct (field i32) (field i32) (field i32) (field i32)
      (field (ref null $abi_frame))))

  (import "wleam:host/v1" "write" (func $write (param i32 i32 i32)))
  (import "wleam:host/v1" "random" (func $random (param i32 i32) (result i32)))
  (import "wleam:host/v1" "panic" (func $panic (param i32 i32)))
  (import "wleam:host/v1" "reference_id"
    (func $reference_id (param externref) (result i64)))

  (memory (export "memory") 1 256)
  (global $next (mut i32) (i32.const 4096))
  (global $live (mut i32) (i32.const 0))
  (global $bytes (mut i32) (i32.const 0))
  (global $frames (mut (ref null $abi_frame)) (ref.null $abi_frame))
  (global $frame_count (mut i32) (i32.const 0))
  (global $next_token (mut i32) (i32.const 1))
  (global $ready (mut i32) (i32.const 0))
  (global $test_runs (mut i32) (i32.const 0))
  (data (i32.const 128) "fixture main\0a")
  (data (i32.const 160) "fixture assertion failed")

  (func (export "__wleam_init")
    (global.set $ready (i32.const 1)))
  (func (export "initialized") (result i32) (global.get $ready))
  (func (export "live_allocations") (result i32) (global.get $live))
  (func (export "live_bytes") (result i32) (global.get $bytes))
  (func (export "active_frames") (result i32) (global.get $frame_count))
  (func (export "heap_end") (result i32) (global.get $next))

  ;; Frames are GC objects, so saving allocator state does not itself change it.
  (func (export "__wleam_abi_mark") (result i32)
    (local $token i32)
    (local.set $token (global.get $next_token))
    (global.set $next_token (i32.add (local.get $token) (i32.const 1)))
    (if (i32.eqz (global.get $next_token))
      (then (global.set $next_token (i32.const 1))))
    (global.set $frames
      (struct.new $abi_frame
        (local.get $token) (global.get $next) (global.get $live)
        (global.get $bytes) (global.get $frames)))
    (global.set $frame_count (i32.add (global.get $frame_count) (i32.const 1)))
    (local.get $token))

  (func (export "__wleam_abi_reset") (param $token i32)
    (local $frame (ref null $abi_frame))
    (local.set $frame (global.get $frames))
    ;; A null frame or a token other than the top token traps before mutation.
    (if (i32.ne (local.get $token) (struct.get $abi_frame 0 (local.get $frame)))
      (then unreachable))
    (global.set $next (struct.get $abi_frame 1 (local.get $frame)))
    (global.set $live (struct.get $abi_frame 2 (local.get $frame)))
    (global.set $bytes (struct.get $abi_frame 3 (local.get $frame)))
    (global.set $frames (struct.get $abi_frame 4 (local.get $frame)))
    (global.set $frame_count (i32.sub (global.get $frame_count) (i32.const 1))))

  (func $alloc (export "__wleam_alloc") (param $len i32) (result i32)
    (local $ptr i32) (local $end i32) (local $pages i32)
    (if (i32.lt_s (local.get $len) (i32.const 0)) (then unreachable))
    (local.set $ptr (global.get $next))
    ;; Keep nonempty alignment slots for zero-length allocations as well.
    (local.set $end
      (i32.and
        (i32.add (i32.add (local.get $ptr) (local.get $len)) (i32.const 8))
        (i32.const -8)))
    (if (i32.lt_u (local.get $end) (local.get $ptr)) (then unreachable))
    (local.set $pages
      (i32.shr_u (i32.add (local.get $end) (i32.const 65535)) (i32.const 16)))
    (if (i32.gt_u (local.get $pages) (memory.size))
      (then
        (if
          (i32.eq
            (memory.grow (i32.sub (local.get $pages) (memory.size)))
            (i32.const -1))
          (then unreachable))))
    (global.set $next (local.get $end))
    (global.set $live (i32.add (global.get $live) (i32.const 1)))
    (global.set $bytes (i32.add (global.get $bytes) (local.get $len)))
    (local.get $ptr))

  (func (export "__wleam_free") (param $ptr i32) (param $len i32)
    (if (i32.eqz (global.get $live)) (then unreachable))
    (if (i32.lt_s (local.get $len) (i32.const 0)) (then unreachable))
    (if (i32.lt_u (local.get $ptr) (i32.const 4096)) (then unreachable))
    (global.set $live (i32.sub (global.get $live) (i32.const 1)))
    (global.set $bytes (i32.sub (global.get $bytes) (local.get $len))))

  (func $copy (param $ptr i32) (param $len i32) (result i32)
    (local $out i32)
    (local.set $out (call $alloc (local.get $len)))
    (memory.copy (local.get $out) (local.get $ptr) (local.get $len))
    (local.get $out))

  (func (export "string_roundtrip") (param $ptr i32) (param $len i32) (result i32 i32)
    (call $copy (local.get $ptr) (local.get $len))
    (local.get $len))

  ;; Int ABI: sign in {-1, 0, 1}, pointer, count of unsigned little-endian u32 limbs.
  (func (export "int_roundtrip")
    (param $sign i32) (param $ptr i32) (param $limbs i32) (result i32 i32 i32)
    (local.get $sign)
    (call $copy (local.get $ptr) (i32.mul (local.get $limbs) (i32.const 4)))
    (local.get $limbs))

  (func (export "bit_array_roundtrip")
    (param $ptr i32) (param $bits i32) (result i32 i32)
    (call $copy
      (local.get $ptr)
      (i32.div_u (i32.add (local.get $bits) (i32.const 7)) (i32.const 8)))
    (local.get $bits))

  (func (export "float_roundtrip") (param $value f64) (result f64) (local.get $value))
  (func (export "bool_roundtrip") (param $value i32) (result i32) (local.get $value))
  (func (export "nil_roundtrip"))
  (func (export "opaque_roundtrip")
    (param $value externref) (result externref) (local.get $value))
  (func (export "reference_id") (param $value externref) (result i64)
    (call $reference_id (local.get $value)))

  (func (export "emit") (param $channel i32) (param $ptr i32) (param $len i32)
    (call $write (local.get $channel) (local.get $ptr) (local.get $len)))
  (func (export "fill_random") (param $ptr i32) (param $len i32) (result i32)
    (call $random (local.get $ptr) (local.get $len)))
  (func (export "fail") (param $ptr i32) (param $len i32)
    (call $panic (local.get $ptr) (local.get $len))
    unreachable)
  (func (export "trap_with_string") (param i32 i32) unreachable)

  ;; Zero-argument entries also exercise CLI execution and isolated test cases.
  (func (export "main")
    (call $write (i32.const 1) (i32.const 128) (i32.const 13)))
  (func (export "passing_test")
    (if (i32.ne (global.get $ready) (i32.const 1)) (then unreachable)))
  (func (export "failing_test")
    (call $panic (i32.const 160) (i32.const 24))
    unreachable)
  (func (export "trapping_test") unreachable)
  (func (export "fresh_instance_test")
    (if (global.get $test_runs) (then unreachable))
    (global.set $test_runs (i32.const 1)))

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
            (i32.add (local.get $sum)
              (struct.get $box 0 (struct.new $box (local.get $n)))))
          (local.set $n (i32.sub (local.get $n) (i32.const 1)))
          (br $again))))
    (local.get $sum))
)
