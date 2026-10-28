;; Dependency-free capability probe for the exact required Wasm feature set.
(module
  (type $box (struct (field i32)))
  (type $unary (func (param i32) (result i32)))
  (func $increment (type $unary) (param $value i32) (result i32)
    (i32.add (local.get $value) (i32.const 1)))
  (elem declare func $increment)
  (func (export "probe") (result i32)
    (struct.get $box 0 (struct.new $box (i32.const 41)))
    (ref.func $increment)
    (return_call_ref $unary))
)
