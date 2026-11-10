;; Host ABI v3 fixture, written by hand: not compiler output. It has the
;; exports that B.5 names, but with the wrong types, which the host must
;; treat as a fault: `ilang_continue` gives nothing, and
;; `ilang_resume:app:clock/v1:later` has the v2 shape, without the request
;; id first.
(module
  (import "app:clock/v1" "later" (func $later (param i32 f64)))

  (memory (export "memory") 1)
  (global $calls (mut i32) (i32.const 0))

  (func (export "ilang_start"))
  (func (export "ilang_alloc") (param $len i32) (result i32)
    (if (local.get $len) (then unreachable))
    (i32.const 0))
  (func (export "ilang_free") (param $ptr i32) (param $len i32))

  ;; Status 2; ilang_continue has no result.
  (func (export "app/async.runnable") (result i32) (i32.const 2))
  (func (export "ilang_result:app/async.runnable"))
  (func (export "ilang_continue")
    (global.set $calls (i32.add (global.get $calls) (i32.const 1))))
  ;; A request of later, whose resume takes no request id.
  (func (export "app/async.bad_resume") (result i32)
    (call $later (i32.const 0) (f64.const 1))
    (i32.const 1))
  (func (export "ilang_result:app/async.bad_resume"))
  (func (export "ilang_resume:app:clock/v1:later") (param $value f64) (result i32)
    (global.set $calls (i32.add (global.get $calls) (i32.const 1)))
    (i32.const 0))
  ;; A sync export, which the host must refuse once the instance is broken.
  (func (export "app/sync.nil")
    (global.set $calls (i32.add (global.get $calls) (i32.const 1))))
  ;; How many times the host called a function above that it must not call.
  (func (export "calls") (result i32) (global.get $calls))

  (@custom "ilang.abi"
    "{\"version\":3,"
    "\"entry\":null,"
    "\"tests\":[],"
    "\"exports\":{"
      "\"app/async.runnable\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":true},"
      "\"app/async.bad_resume\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":true},"
      "\"app/sync.nil\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":false}},"
    "\"services\":{"
      "\"app:clock/v1\":{\"later\":{\"params\":[\"float\"],\"result\":\"float\",\"errors\":[\"string\"],\"async\":true}}},"
    "\"externals\":{}}")
)
