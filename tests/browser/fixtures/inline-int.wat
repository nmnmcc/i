;; ABI 5 only. Any buffer allocation or free traps, including a free of an
;; inline integer mistaken for an address. Echo crosses an import too.
(module
  (import "custom" "echo_int" (func $echo (param i32 i32 i32) (result i32 i32 i32)))
  (memory (export "memory") 1)
  (func (export "ilang_start"))
  (func (export "ilang_alloc") (param i32) (result i32) unreachable)
  (func (export "ilang_free") (param i32 i32) unreachable)
  (func (export "echo") (param $s i32) (param $v i32) (param $n i32) (result i32 i32 i32)
    (if (local.get $n) (then unreachable))
    (call $echo (local.get $s) (local.get $v) (local.get $n)))
  (func (export "wrong_sign") (result i32 i32 i32) (i32.const 1) (i32.const -1) (i32.const 0))
  (func (export "wrong_zero") (result i32 i32 i32) (i32.const -1) (i32.const 0) (i32.const 0))
  (func (export "wrong_marker") (result i32 i32 i32) (i32.const 2) (i32.const 2) (i32.const 0))
  (@custom "ilang.abi"
    "{\"version\":5,\"entry\":null,\"tests\":[],"
    "\"exports\":{\"echo\":{\"params\":[\"int\"],\"result\":\"int\"},"
    "\"wrong_sign\":{\"params\":[],\"result\":\"int\"},"
    "\"wrong_zero\":{\"params\":[],\"result\":\"int\"},"
    "\"wrong_marker\":{\"params\":[],\"result\":\"int\"}},"
    "\"services\":{},\"externals\":{\"custom\":{\"echo_int\":{\"params\":[\"int\"],\"result\":\"int\"}}}}"))
