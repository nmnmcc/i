;; Host-binding fixture. Imported host functions allocate copied result buffers
;; through __wleam_alloc; the outer caller owns both input and result allocations.
(module
  (type $abi_frame
    (struct (field i32) (field i32) (field i32) (field i32)
      (field (ref null $abi_frame))))
  (import "custom" "echo_string"
    (func $echo_string (param i32 i32) (result i32 i32)))
  (import "custom" "echo_int"
    (func $echo_int (param i32 i32 i32) (result i32 i32 i32)))
  (import "custom" "echo_bits"
    (func $echo_bits (param i32 i32) (result i32 i32)))
  (import "custom" "echo_ref"
    (func $echo_ref (param externref) (result externref)))
  (import "custom" "echo_bool"
    (func $echo_bool (param i32) (result i32)))
  (import "custom" "echo_float"
    (func $echo_float (param f64) (result f64)))
  (import "custom" "echo_nil" (func $echo_nil))

  (memory (export "memory") 1 256)
  (global $next (mut i32) (i32.const 4096))
  (global $live (mut i32) (i32.const 0))
  (global $bytes (mut i32) (i32.const 0))
  (global $frames (mut (ref null $abi_frame)) (ref.null $abi_frame))
  (global $frame_count (mut i32) (i32.const 0))
  (global $next_token (mut i32) (i32.const 1))
  (global $ready (mut i32) (i32.const 0))

  (func (export "__wleam_init")
    (global.set $ready (i32.const 1)))
  (func (export "initialized") (result i32) (global.get $ready))
  (func (export "live_allocations") (result i32) (global.get $live))
  (func (export "live_bytes") (result i32) (global.get $bytes))
  (func (export "active_frames") (result i32) (global.get $frame_count))
  (func (export "heap_end") (result i32) (global.get $next))

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

  (func $free (export "__wleam_free") (param $ptr i32) (param $len i32)
    (if (i32.eqz (global.get $live)) (then unreachable))
    (if (i32.lt_s (local.get $len) (i32.const 0)) (then unreachable))
    (if (i32.lt_u (local.get $ptr) (i32.const 4096)) (then unreachable))
    (global.set $live (i32.sub (global.get $live) (i32.const 1)))
    (global.set $bytes (i32.sub (global.get $bytes) (local.get $len))))

  (func (export "echo_string") (param $ptr i32) (param $len i32) (result i32 i32)
    (call $echo_string (local.get $ptr) (local.get $len)))
  (func (export "leaking_throw") (param $ptr i32) (param $len i32) (result i32 i32)
    (local $temporary i32) (local $output_ptr i32) (local $output_len i32)
    (local.set $temporary (call $alloc (i32.const 16)))
    (call $echo_string (local.get $ptr) (local.get $len))
    (local.set $output_len)
    (local.set $output_ptr)
    ;; A host exception skips this guest cleanup; an ABI reset must reclaim it.
    (call $free (local.get $temporary) (i32.const 16))
    (local.get $output_ptr)
    (local.get $output_len))
  (func (export "echo_int")
    (param $sign i32) (param $ptr i32) (param $count i32) (result i32 i32 i32)
    (call $echo_int (local.get $sign) (local.get $ptr) (local.get $count)))
  (func (export "echo_bits") (param $ptr i32) (param $bits i32) (result i32 i32)
    (call $echo_bits (local.get $ptr) (local.get $bits)))
  (func (export "echo_ref") (param $value externref) (result externref)
    (call $echo_ref (local.get $value)))
  (func (export "echo_bool") (param $value i32) (result i32)
    (call $echo_bool (local.get $value)))
  (func (export "echo_float") (param $value f64) (result f64)
    (call $echo_float (local.get $value)))
  (func (export "echo_nil") (call $echo_nil))
)
