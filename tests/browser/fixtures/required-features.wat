;; Dependency-free probe for the Wasm features that i needs (spec B.1):
;; garbage collection, typed function references, tail calls, and exception
;; handling with exnref. The export probe gives 42.
(module
  (type $box (struct (field i32)))
  (type $unary (func (param i32) (result i32)))
  (tag $tag (param i32))
  (func $increment (type $unary) (param $value i32) (result i32)
    (i32.add (local.get $value) (i32.const 1)))
  (elem declare func $increment)
  ;; catch_ref gives an exnref, which throw_ref throws again.
  (func $rethrow (param $value i32) (result i32)
    (block $outer (result i32)
      (try_table (catch $tag $outer)
        (block $inner (result i32 exnref)
          (try_table (catch_ref $tag $inner) (throw $tag (local.get $value)))
          (unreachable))
        (throw_ref))
      (unreachable)))
  (func (export "probe") (result i32)
    (call $rethrow (struct.get $box 0 (struct.new $box (i32.const 41))))
    (ref.func $increment)
    (return_call_ref $unary))
)
