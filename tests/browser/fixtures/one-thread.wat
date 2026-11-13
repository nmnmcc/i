;; Host ABI v3 fixture, written by hand: not compiler output. Its async entry
;; point and its async test each print, then wait for later three times in a
;; row and print after each answer, so a host can check on which threads the
;; services run.
(module
  (import "app:console/v1" "print" (func $print (param i32 i32)))
  (import "app:clock/v1" "later" (func $later (param i32 f64)))

  (memory (export "memory") 1)
  (global $next (mut i32) (i32.const 4096))
  (global $answers (mut i32) (i32.const 0))
  ;; The id of the outstanding request (B.4).
  (global $id (mut i32) (i32.const 0))
  (data (i32.const 0) "begin\0a")
  (data (i32.const 16) "answer\0a")

  (func (export "ilang_start"))

  ;; A bump allocator that never frees; enough for a few small calls.
  (func (export "ilang_alloc") (param $len i32) (result i32)
    (local $ptr i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (local.set $ptr (global.get $next))
    (global.set $next
      (i32.and (i32.add (i32.add (local.get $ptr) (local.get $len)) (i32.const 7)) (i32.const -8)))
    (if (i32.gt_u (global.get $next) (i32.const 65536)) (then unreachable))
    (local.get $ptr))
  (func (export "ilang_free") (param $ptr i32) (param $len i32))

  (func $begin (result i32)
    (global.set $answers (i32.const 0))
    (call $print (i32.const 0) (i32.const 6))
    (call $later (global.get $id) (f64.const 1))
    (i32.const 1))
  (func (export "app/main.main") (result i32) (call $begin))
  (func (export "app/test.waits") (result i32) (call $begin))

  ;; later(x) answers x; the next request asks for x + 1.
  (func (export "ilang_resume:app:clock/v1:later")
    (param $id i32) (param $tag i32) (param $value f64) (param $ptr i32) (param $len i32) (result i32)
    (if (i32.ne (local.get $id) (global.get $id)) (then unreachable))
    (if (local.get $tag) (then unreachable))
    (global.set $id (i32.add (global.get $id) (i32.const 1)))
    (global.set $answers (i32.add (global.get $answers) (i32.const 1)))
    (call $print (i32.const 16) (i32.const 7))
    (if (i32.lt_u (global.get $answers) (i32.const 3))
      (then
        (call $later (global.get $id) (f64.add (local.get $value) (f64.const 1)))
        (return (i32.const 1))))
    (i32.const 0))

  (func (export "ilang_result:app/main.main")
    (if (i32.ne (global.get $answers) (i32.const 3)) (then unreachable)))
  (func (export "ilang_result:app/test.waits")
    (if (i32.ne (global.get $answers) (i32.const 3)) (then unreachable)))

  (@custom "ilang.abi"
    "{\"version\":4,"
    "\"entry\":{\"name\":\"app/main.main\",\"async\":true},"
    "\"tests\":[{\"name\":\"app/test.waits\",\"async\":true}],"
    "\"exports\":{},"
    "\"services\":{"
      "\"app:console/v1\":{\"print\":{\"params\":[\"string\"],\"result\":\"nil\",\"errors\":[],\"async\":false}},"
      "\"app:clock/v1\":{\"later\":{\"params\":[\"float\"],\"result\":\"float\",\"errors\":[\"string\"],\"async\":true}}},"
    "\"externals\":{}}")
)
