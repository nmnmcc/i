;; Host ABI v3 fixture, written by hand: not compiler output. Its
;; `ilang_start` starts a request of the async import `later`, which B.4
;; does not allow: the host must fail the instantiation, and must not run
;; the service.
(module
  (import "app:clock/v1" "later" (func $later (param i32 f64)))

  (memory (export "memory") 1)

  (func (export "ilang_start") (call $later (i32.const 0) (f64.const 1)))
  (func (export "ilang_alloc") (param $len i32) (result i32)
    (if (local.get $len) (then unreachable))
    (i32.const 0))
  (func (export "ilang_free") (param $ptr i32) (param $len i32))
  (func (export "ilang_resume:app:clock/v1:later") (param $id i32) (param $value f64) (result i32)
    (i32.const 0))

  (@custom "ilang.abi"
    "{\"version\":3,"
    "\"entry\":null,"
    "\"tests\":[],"
    "\"exports\":{},"
    "\"services\":{"
      "\"app:clock/v1\":{\"later\":{\"params\":[\"float\"],\"result\":\"float\",\"errors\":[\"string\"],\"async\":true}}},"
    "\"externals\":{}}")
)
