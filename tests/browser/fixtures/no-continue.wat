;; Host ABI v3 fixture, written by hand: not compiler output. Each async
;; export breaks a rule of B.5 that the host must catch: one is runnable
;; (status 2) but the module has no ilang_continue, and one starts a request
;; of an import that has no ilang_resume export.
(module
  (import "app:clock/v1" "later" (func $later (param i32 f64)))
  (import "app:clock/v1" "ready" (func $ready (param i32)))
  (import "ilang:host/v1" "cancel" (func $cancel (param i32)))

  (memory (export "memory") 1)
  (global $calls (mut i32) (i32.const 0))

  (func (export "ilang_start"))
  (func (export "ilang_alloc") (param $len i32) (result i32)
    (if (local.get $len) (then unreachable))
    (i32.const 0))
  (func (export "ilang_free") (param $ptr i32) (param $len i32))

  ;; Status 2, with no ilang_continue export.
  (func (export "app/async.runnable") (result i32) (i32.const 2))
  (func (export "ilang_result:app/async.runnable"))
  ;; A request of later, with no ilang_resume:app:clock/v1:later export.
  (func (export "app/async.no_resume") (result i32)
    (call $later (i32.const 0) (f64.const 1))
    (i32.const 1))
  (func (export "ilang_result:app/async.no_resume"))
  ;; A ready answer must not hide a missing or wrongly typed ilang_continue.
  (func (export "app/async.runnable_ready") (result i32)
    (call $ready (i32.const 0))
    (i32.const 2))
  (func (export "ilang_resume:app:clock/v1:ready") (param i32) (result i32)
    (global.set $calls (i32.add (global.get $calls) (i32.const 1)))
    (i32.const 0))
  (func (export "ilang_result:app/async.runnable_ready")
    (global.set $calls (i32.add (global.get $calls) (i32.const 1))))
  ;; Returning 0 or cancelling must not hide an invalid resume export.
  (func (export "app/async.no_resume_done") (result i32)
    (call $later (i32.const 0) (f64.const 1))
    (i32.const 0))
  (func (export "ilang_result:app/async.no_resume_done")
    (global.set $calls (i32.add (global.get $calls) (i32.const 1))))
  (func (export "app/async.no_resume_cancel") (result i32)
    (call $later (i32.const 0) (f64.const 1))
    (call $cancel (i32.const 0))
    (i32.const 0))
  (func (export "ilang_result:app/async.no_resume_cancel")
    (global.set $calls (i32.add (global.get $calls) (i32.const 1))))
  ;; A sync export, which the host must refuse once the instance is broken.
  (func (export "app/sync.nil")
    (global.set $calls (i32.add (global.get $calls) (i32.const 1))))
  (func (export "calls") (result i32) (global.get $calls))

  (@custom "ilang.abi"
    "{\"version\":4,"
    "\"entry\":null,"
    "\"tests\":[],"
    "\"exports\":{"
      "\"app/async.runnable\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":true},"
      "\"app/async.no_resume\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":true},"
      "\"app/async.runnable_ready\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":true},"
      "\"app/async.no_resume_done\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":true},"
      "\"app/async.no_resume_cancel\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":true},"
      "\"app/sync.nil\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":false}},"
    "\"services\":{"
      "\"app:clock/v1\":{\"ready\":{\"params\":[],\"result\":\"nil\",\"errors\":[],\"async\":true},"
      "\"later\":{\"params\":[\"float\"],\"result\":\"float\",\"errors\":[\"string\"],\"async\":true}}},"
    "\"externals\":{}}")
)
