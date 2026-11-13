//! Shared, boxed WasmGC values. The generated functions never expose GC addresses.

mod inspect;
mod record;
mod strings;

pub fn wat(references: bool) -> String {
    let mut source = String::from(CORE);
    if references {
        source = source
            .replace(HOST_NEW, NOMINAL_HOST_NEW)
            .replace(HOST_ID, NOMINAL_HOST_ID)
            .replace(HOST_COMPARE, NOMINAL_HOST_COMPARE)
            .replace(HOST_HASH, NOMINAL_HOST_HASH);
    }
    source.push_str(&strings::wat());
    source.push_str(&inspect::wat());
    source.push_str(record::WAT);
    source
}

const CORE: &str = r#"
(global $value_false eqref
  (struct.new $obj (i32.const 1) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(global $value_true eqref
  (struct.new $obj (i32.const 1) (i32.const 1) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(global $value_empty_list eqref
  (struct.new $obj (i32.const 6) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $value_tag (param $v eqref) (result i32)
  (if (ref.is_null (local.get $v)) (then (return (i32.const 0))))
  (if (ref.test (ref i31) (local.get $v)) (then (return (i32.const 8))))
  (if (ref.test (ref $hostref) (local.get $v)) (then (return (i32.const 10))))
  (struct.get $val 0 (ref.cast (ref $val) (local.get $v))))
;; A host reference keeps its reference_id, which never changes (spec B.4).
(func $host_ref_new (param $r externref) (result eqref)
  (struct.new $hostref (local.get $r) (call $host_reference_id (local.get $r))))
(func $host_ref_get (param $v eqref) (result externref)
  (struct.get $hostref 0 (ref.cast (ref $hostref) (local.get $v))))
(func $host_id (param $v eqref) (result i64)
  (struct.get $hostref 1 (ref.cast (ref $hostref) (local.get $v))))
(func $codepoint_new (param $c i32) (result eqref)
  (struct.new $obj (i32.const 13) (local.get $c) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $codepoint_get (param $v eqref) (result i32)
  (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))))
(func $bool_new (param $v i32) (result eqref)
  (if (result eqref) (local.get $v)
    (then (global.get $value_true)) (else (global.get $value_false))))
(func $bool_get (param $v eqref) (result i32)
  (struct.get $obj 1 (ref.cast (ref $obj) (local.get $v))))
(func $float_new (param $v f64) (result eqref)
  ;; NaN fails the comparison as well (spec §11.3).
  (if (i32.eqz (f64.le (f64.abs (local.get $v)) (f64.const 0x1.fffffffffffffp+1023)))
    (then (call $stop_float) unreachable))
  (struct.new $obj (i32.const 2) (i32.const 0) (i64.const 0) (local.get $v)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $float_get (param $v eqref) (result f64)
  (struct.get $obj 3 (ref.cast (ref $obj) (local.get $v))))
(func $list_nil (result eqref)
  (global.get $value_empty_list))
(func $list_cons (param $head eqref) (param $tail eqref) (result eqref)
  (struct.new $cons (i32.const 6) (local.get $head) (local.get $tail)))
(func $field (param $v eqref) (param $index i32) (result eqref)
  (array.get $values
    (ref.as_non_null (struct.get $obj 5 (ref.cast (ref $obj) (local.get $v))))
    (local.get $index)))
(func $record (param $ctor i32) (param $type i64) (param $fields (ref $values)) (result eqref)
  (struct.new $obj (i32.const 5) (local.get $ctor) (local.get $type) (f64.const 0)
    (ref.null $bytes) (local.get $fields) (ref.null $limbs) (ref.null $call)))
(func $tuple (param $fields (ref $values)) (result eqref)
  (struct.new $obj (i32.const 4) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (local.get $fields) (ref.null $limbs) (ref.null $call)))
(func $list_is_empty (param $v eqref) (result i32)
  (i32.eqz (ref.test (ref $cons) (local.get $v))))
(func $list_head (param $v eqref) (result eqref)
  (struct.get $cons 1 (ref.cast (ref $cons) (local.get $v))))
(func $list_tail (param $v eqref) (result eqref)
  (struct.get $cons 2 (ref.cast (ref $cons) (local.get $v))))
(func $list_length (param $v eqref) (result eqref) (local $n i64)
  (block $done (loop $each
    (br_if $done (call $list_is_empty (local.get $v)))
    (local.set $n (i64.add (local.get $n) (i64.const 1)))
    (local.set $v (call $list_tail (local.get $v))) (br $each)))
  (call $int_from_i64 (local.get $n)))
(func $list_reverse (param $v eqref) (result eqref) (local $out eqref)
  (local.set $out (call $list_nil))
  (block $done (loop $each
    (br_if $done (call $list_is_empty (local.get $v)))
    (local.set $out (call $list_cons (call $list_head (local.get $v)) (local.get $out)))
    (local.set $v (call $list_tail (local.get $v))) (br $each)))
  (local.get $out))
(func $value_bit (param $v eqref) (param $at i64) (result i32)
  (local $o (ref null $obj)) (local $p i64)
  (local.set $o (ref.cast (ref $obj) (local.get $v)))
  (local.set $p (i64.add (local.get $at) (i64.extend_i32_u (struct.get $obj 1 (local.get $o)))))
  (i32.and
    (i32.shr_u
      (array.get_u $bytes (struct.get $obj 4 (local.get $o))
        (i32.wrap_i64 (i64.shr_u (local.get $p) (i64.const 3))))
      (i32.sub (i32.const 7) (i32.wrap_i64 (i64.and (local.get $p) (i64.const 7)))))
    (i32.const 1)))
;; The siblings still to compare: the parts of two `$values` arrays from
;; index `i`, or, when the arrays are null, the tails of two lists.
(type $compare_frame (struct
  (field (ref null $compare_frame))
  (field (ref null $values)) (field (ref null $values)) (field (mut i32))
  (field eqref) (field eqref)))
(func $value_eq (param $a eqref) (param $b eqref) (result i32)
  (if (ref.eq (local.get $a) (local.get $b)) (then (return (i32.const 1))))
  ;; Two small Ints are equal only when they are the same `i31ref`.
  (if (call $both_small (local.get $a) (local.get $b)) (then (return (i32.const 0))))
  (i32.eqz (call $value_compare_i32 (local.get $a) (local.get $b))))
;; `==` (spec §11.4).
(func $equal (param $a eqref) (param $b eqref) (result i32)
  (call $value_eq (local.get $a) (local.get $b)))
;; Functions, boundaries and continuations are the same only as themselves.
(func $value_identity_kind (param $kind i32) (result i32)
  (i32.or (i32.eq (local.get $kind) (i32.const 7))
    (i32.or (i32.eq (local.get $kind) (i32.const 11)) (i32.or (i32.eq (local.get $kind) (i32.const 12)) (i32.or (i32.eq (local.get $kind) (i32.const 15)) (i32.eq (local.get $kind) (i32.const 17)))))))
(func $compare_i32 (param $a i32) (param $b i32) (result i32)
  (i32.sub (i32.gt_s (local.get $a) (local.get $b)) (i32.lt_s (local.get $a) (local.get $b))))
(func $compare_i64 (param $a i64) (param $b i64) (result i32)
  (i32.sub (i64.gt_s (local.get $a) (local.get $b)) (i64.lt_s (local.get $a) (local.get $b))))
;; Two small Ints that are not the same are compared at once, without
;; a frame.
(func $both_small (param $a eqref) (param $b eqref) (result i32)
  (i32.and (ref.test (ref i31) (local.get $a)) (ref.test (ref i31) (local.get $b))))
(func $compare_small (param $a eqref) (param $b eqref) (result i32)
  (call $compare_i32 (i31.get_s (ref.cast (ref i31) (local.get $a)))
                     (i31.get_s (ref.cast (ref i31) (local.get $b)))))
(func $value_compare_i32 (param $a eqref) (param $b eqref) (result i32)
  (local $ka i32) (local $kb i32) (local $c i32)
  (local $oa (ref null $obj)) (local $ob (ref null $obj))
  (local $xa (ref null $values)) (local $xb (ref null $values))
  (local $pending (ref null $compare_frame))
  (local $ca (ref null $cons)) (local $cb (ref null $cons)) (local $ha eqref) (local $hb eqref)
  (local $i i32) (local $na i32) (local $nb i32) (local $bit i64) (local $ba i64) (local $bb i64)
  ;; A frame records siblings still to compare, never a native call frame.
  ;; Primitive values and single remaining children need no allocation.
  (loop $again
    (local.set $c (i32.const 0))
    (block $compared
      (br_if $compared (ref.eq (local.get $a) (local.get $b)))
      (local.set $ka (call $value_tag (local.get $a)))
      (local.set $kb (call $value_tag (local.get $b)))
      (if (i32.ne (local.get $ka) (local.get $kb))
        (then (return (call $compare_i32 (local.get $ka) (local.get $kb)))))
      (br_if $compared (i32.eqz (local.get $ka)))
      (if (i32.eq (local.get $ka) (i32.const 8)) (then
        (local.set $c (call $int_compare (local.get $a) (local.get $b)))
        (br $compared)))
      (if (i32.eq (local.get $ka) (i32.const 10)) (then
        (local.set $c (call $compare_i64 (call $host_id (local.get $a)) (call $host_id (local.get $b))))
        (br $compared)))
      ;; Identity values that are not ref.eq are different. The order between
      ;; them only has to be consistent within one comparison.
      (if (call $value_identity_kind (local.get $ka)) (then (return (i32.const 1))))
      ;; Lists: [] comes first; else the heads, then the tails. Equal heads
      ;; and small Ints need no frame.
      (if (i32.eq (local.get $ka) (i32.const 6)) (then
        (local.set $na (ref.test (ref $cons) (local.get $a)))
        (local.set $nb (ref.test (ref $cons) (local.get $b)))
        (if (i32.eqz (i32.and (local.get $na) (local.get $nb))) (then
          (local.set $c (call $compare_i32 (local.get $na) (local.get $nb)))
          (br $compared)))
        (local.set $ca (ref.cast (ref $cons) (local.get $a)))
        (local.set $cb (ref.cast (ref $cons) (local.get $b)))
        (local.set $ha (struct.get $cons 1 (local.get $ca)))
        (local.set $hb (struct.get $cons 1 (local.get $cb)))
        (local.set $a (struct.get $cons 2 (local.get $ca)))
        (local.set $b (struct.get $cons 2 (local.get $cb)))
        (br_if $again (ref.eq (local.get $ha) (local.get $hb)))
        (if (call $both_small (local.get $ha) (local.get $hb)) (then
          (return (call $compare_small (local.get $ha) (local.get $hb)))))
        (local.set $pending (struct.new $compare_frame (local.get $pending)
          (ref.null $values) (ref.null $values) (i32.const 0) (local.get $a) (local.get $b)))
        (local.set $a (local.get $ha))
        (local.set $b (local.get $hb))
        (br $again)))
      (local.set $oa (ref.cast (ref $obj) (local.get $a)))
      (local.set $ob (ref.cast (ref $obj) (local.get $b)))
      (if (i32.eq (local.get $ka) (i32.const 2)) (then
        (local.set $c (i32.sub
          (f64.gt (struct.get $obj 3 (local.get $oa)) (struct.get $obj 3 (local.get $ob)))
          (f64.lt (struct.get $obj 3 (local.get $oa)) (struct.get $obj 3 (local.get $ob)))))
        (br $compared)))
      (if (i32.eq (local.get $ka) (i32.const 3)) (then
        (local.set $c (call $string_compare (local.get $a) (local.get $b)))
        (br $compared)))
      (if (i32.eq (local.get $ka) (i32.const 9)) (then
        (local.set $ba (struct.get $obj 2 (local.get $oa)))
        (local.set $bb (struct.get $obj 2 (local.get $ob)))
        (local.set $bit (i64.const 0))
        (block $bits_done (loop $bits_each
          (br_if $bits_done (i32.or (i64.ge_u (local.get $bit) (local.get $ba))
                                   (i64.ge_u (local.get $bit) (local.get $bb))))
          (local.set $c (call $compare_i32 (call $value_bit (local.get $a) (local.get $bit))
                                         (call $value_bit (local.get $b) (local.get $bit))))
          (if (local.get $c) (then (return (local.get $c))))
          (local.set $bit (i64.add (local.get $bit) (i64.const 1))) (br $bits_each)))
        (local.set $c (call $compare_i64 (local.get $ba) (local.get $bb)))
        (br $compared)))
      (local.set $c (call $compare_i64 (struct.get $obj 2 (local.get $oa)) (struct.get $obj 2 (local.get $ob))))
      (if (local.get $c) (then (return (local.get $c))))
      (local.set $c (call $compare_i32 (struct.get $obj 1 (local.get $oa)) (struct.get $obj 1 (local.get $ob))))
      (if (local.get $c) (then (return (local.get $c))))
      (local.set $xa (struct.get $obj 5 (local.get $oa)))
      (local.set $xb (struct.get $obj 5 (local.get $ob)))
      (if (i32.or (ref.is_null (local.get $xa)) (ref.is_null (local.get $xb))) (then
        (local.set $c (i32.sub (ref.is_null (local.get $xb)) (ref.is_null (local.get $xa))))
        (br $compared)))
      (local.set $na (array.len (local.get $xa)))
      (local.set $nb (array.len (local.get $xb)))
      (local.set $i (i32.const 0))
      (block $first_found (loop $first
        (br_if $first_found (i32.or (i32.ge_u (local.get $i) (local.get $na))
                                   (i32.ge_u (local.get $i) (local.get $nb))))
        (br_if $first_found (i32.eqz (ref.eq
          (array.get $values (local.get $xa) (local.get $i))
          (array.get $values (local.get $xb) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $first)))
      (if (i32.or (i32.ge_u (local.get $i) (local.get $na))
                  (i32.ge_u (local.get $i) (local.get $nb))) (then
        (local.set $c (call $compare_i32 (local.get $na) (local.get $nb)))
        (br $compared)))
      (local.set $a (array.get $values (local.get $xa) (local.get $i)))
      (local.set $b (array.get $values (local.get $xb) (local.get $i)))
      (if (call $both_small (local.get $a) (local.get $b)) (then
        (return (call $compare_small (local.get $a) (local.get $b)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if (i32.or (i32.ne (local.get $na) (local.get $nb)) (i32.lt_u (local.get $i) (local.get $na)))
        (then (local.set $pending (struct.new $compare_frame (local.get $pending)
          (local.get $xa) (local.get $xb) (local.get $i) (ref.null eq) (ref.null eq)))))
      (br $again))
    (if (local.get $c) (then (return (local.get $c))))
    (loop $resume
      (if (ref.is_null (local.get $pending)) (then (return (i32.const 0))))
      (if (ref.is_null (struct.get $compare_frame 1 (local.get $pending))) (then
        (local.set $a (struct.get $compare_frame 4 (local.get $pending)))
        (local.set $b (struct.get $compare_frame 5 (local.get $pending)))
        (local.set $pending (struct.get $compare_frame 0 (local.get $pending)))
        (br $again)))
      (local.set $xa (struct.get $compare_frame 1 (local.get $pending)))
      (local.set $xb (struct.get $compare_frame 2 (local.get $pending)))
      (local.set $i (struct.get $compare_frame 3 (local.get $pending)))
      (local.set $na (array.len (local.get $xa)))
      (local.set $nb (array.len (local.get $xb)))
      (block $next_found (loop $next
        (br_if $next_found (i32.or (i32.ge_u (local.get $i) (local.get $na))
                                  (i32.ge_u (local.get $i) (local.get $nb))))
        (br_if $next_found (i32.eqz (ref.eq
          (array.get $values (local.get $xa) (local.get $i))
          (array.get $values (local.get $xb) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $next)))
      (if (i32.or (i32.ge_u (local.get $i) (local.get $na))
                  (i32.ge_u (local.get $i) (local.get $nb))) (then
        (local.set $pending (struct.get $compare_frame 0 (local.get $pending)))
        (local.set $c (call $compare_i32 (local.get $na) (local.get $nb)))
        (if (local.get $c) (then (return (local.get $c))))
        (br $resume)))
      (local.set $a (array.get $values (local.get $xa) (local.get $i)))
      (local.set $b (array.get $values (local.get $xb) (local.get $i)))
      (if (call $both_small (local.get $a) (local.get $b)) (then
        (return (call $compare_small (local.get $a) (local.get $b)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (if (i32.and (i32.eq (local.get $na) (local.get $nb)) (i32.eq (local.get $i) (local.get $na)))
        (then (local.set $pending (struct.get $compare_frame 0 (local.get $pending))))
        (else (struct.set $compare_frame 3 (local.get $pending) (local.get $i))))
      (br $again)))
  unreachable)
(func $value_compare (param $a eqref) (param $b eqref) (result eqref)
  (ref.i31 (call $value_compare_i32 (local.get $a) (local.get $b))))
(func $hash_mix (param $hash i32) (param $word i32) (result i32)
  (i32.mul (i32.xor (local.get $hash) (local.get $word)) (i32.const 16777619)))
(func $value_hash_u32 (param $v eqref) (result i32)
  (local $kind i32) (local $h i32) (local $i i32)
  (local $o (ref null $obj)) (local $xs (ref null $values))
  (local $bs (ref null $bytes)) (local $ls (ref null $limbs))
  (local $n i32) (local $word i64) (local $bit i64)
  ;; The parts still to hash, on a growable stack: linear in the size.
  (local $stack (ref $values)) (local $sp i32) (local $grown (ref $values))
  (local.set $h (i32.const -2128831035))
  (local.set $stack (array.new_default $values (i32.const 16)))
  (array.set $values (local.get $stack) (i32.const 0) (local.get $v))
  (local.set $sp (i32.const 1))
  (block $done (loop $walk
    (br_if $done (i32.eqz (local.get $sp)))
    (local.set $sp (i32.sub (local.get $sp) (i32.const 1)))
    (local.set $v (array.get $values (local.get $stack) (local.get $sp)))
    (local.set $kind (call $value_tag (local.get $v)))
    (local.set $h (call $hash_mix (local.get $h) (local.get $kind)))
    (br_if $walk (i32.eqz (local.get $kind)))
    ;; Identity values hash by their tag alone: equal values are one value.
    (br_if $walk (call $value_identity_kind (local.get $kind)))
    (local.set $i (i32.const 0))
    ;; A small Int: as below, with its one part, without making the parts.
    (if (ref.test (ref i31) (local.get $v)) (then
      (local.set $n (i31.get_s (ref.cast (ref i31) (local.get $v))))
      (local.set $h (call $hash_mix (local.get $h) (call $int_sign (local.get $v))))
      (if (i32.eqz (local.get $n))
        (then (local.set $h (call $hash_mix (local.get $h) (i32.const 0))))
        (else
          (local.set $h (call $hash_mix (local.get $h) (i32.const 1)))
          (if (i32.lt_s (local.get $n) (i32.const 0))
            (then (local.set $n (i32.sub (i32.const 0) (local.get $n)))))
          (local.set $h (call $hash_mix (local.get $h) (local.get $n)))))
      (br $walk)))
    (if (i32.eq (local.get $kind) (i32.const 8)) (then
      (local.set $h (call $hash_mix (local.get $h) (call $int_sign (local.get $v))))
      (local.set $ls (call $int_limbs (local.get $v)))
      (local.set $n (array.len (local.get $ls)))
      (local.set $h (call $hash_mix (local.get $h) (local.get $n)))
      (block $int_done (loop $int_each
        (br_if $int_done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $h (call $hash_mix (local.get $h) (array.get $limbs (local.get $ls) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $int_each)))
      (br $walk)))
    (if (i32.eq (local.get $kind) (i32.const 10)) (then
      (local.set $word (call $host_id (local.get $v)))
      (local.set $h (call $hash_mix (call $hash_mix (local.get $h) (i32.wrap_i64 (local.get $word)))
        (i32.wrap_i64 (i64.shr_u (local.get $word) (i64.const 32)))))
      (br $walk)))
    ;; A list hashes as it did when its cells were `$obj` values with two
    ;; parts: constructor id 0, type id 0, then the parts unless it is [].
    (if (i32.eq (local.get $kind) (i32.const 6)) (then
      (local.set $h (call $hash_mix (local.get $h) (i32.const 0)))
      (local.set $h (call $hash_mix (call $hash_mix (local.get $h) (i32.const 0)) (i32.const 0)))
      (br_if $walk (call $list_is_empty (local.get $v)))
      (local.set $h (call $hash_mix (local.get $h) (i32.const 2)))
      (if (i32.gt_u (i32.add (local.get $sp) (i32.const 2)) (array.len (local.get $stack))) (then
        (if (i32.ge_u (array.len (local.get $stack)) (i32.const 268435456)) (then unreachable))
        (local.set $grown (array.new_default $values
          (i32.add (i32.shl (array.len (local.get $stack)) (i32.const 1)) (i32.const 2))))
        (array.copy $values $values (local.get $grown) (i32.const 0) (local.get $stack) (i32.const 0) (local.get $sp))
        (local.set $stack (local.get $grown))))
      (array.set $values (local.get $stack) (local.get $sp) (call $list_tail (local.get $v)))
      (array.set $values (local.get $stack) (i32.add (local.get $sp) (i32.const 1)) (call $list_head (local.get $v)))
      (local.set $sp (i32.add (local.get $sp) (i32.const 2)))
      (br $walk)))
    (local.set $o (ref.cast (ref $obj) (local.get $v)))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then
      (local.set $word (i64.const 0))
      (if (f64.ne (struct.get $obj 3 (local.get $o)) (f64.const 0))
        (then (local.set $word (i64.reinterpret_f64 (struct.get $obj 3 (local.get $o))))))
      (local.set $h (call $hash_mix (call $hash_mix (local.get $h) (i32.wrap_i64 (local.get $word)))
        (i32.wrap_i64 (i64.shr_u (local.get $word) (i64.const 32)))))
      (br $walk)))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then
      (local.set $bs (struct.get $obj 4 (local.get $o)))
      (local.set $n (array.len (local.get $bs)))
      (block $string_done (loop $string_each
        (br_if $string_done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $h (call $hash_mix (local.get $h) (array.get_u $bytes (local.get $bs) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $string_each)))
      (br $walk)))
    (if (i32.eq (local.get $kind) (i32.const 9)) (then
      (local.set $word (struct.get $obj 2 (local.get $o)))
      (local.set $h (call $hash_mix (call $hash_mix (local.get $h) (i32.wrap_i64 (local.get $word)))
        (i32.wrap_i64 (i64.shr_u (local.get $word) (i64.const 32)))))
      (local.set $bit (i64.const 0))
      (block $bits_done (loop $bits_each
        (br_if $bits_done (i64.ge_u (local.get $bit) (local.get $word)))
        (local.set $h (call $hash_mix (local.get $h) (call $value_bit (local.get $v) (local.get $bit))))
        (local.set $bit (i64.add (local.get $bit) (i64.const 1))) (br $bits_each)))
      (br $walk)))
    (local.set $h (call $hash_mix (local.get $h) (struct.get $obj 1 (local.get $o))))
    (local.set $word (struct.get $obj 2 (local.get $o)))
    (local.set $h (call $hash_mix (call $hash_mix (local.get $h) (i32.wrap_i64 (local.get $word)))
      (i32.wrap_i64 (i64.shr_u (local.get $word) (i64.const 32)))))
    (local.set $xs (struct.get $obj 5 (local.get $o)))
    (br_if $walk (ref.is_null (local.get $xs)))
    (local.set $n (array.len (local.get $xs)))
    (local.set $h (call $hash_mix (local.get $h) (local.get $n)))
    ;; The parts go on the stack last first, so the first is hashed first.
    (if (i32.gt_u (i32.add (local.get $sp) (local.get $n)) (array.len (local.get $stack))) (then
      (if (i32.ge_u (array.len (local.get $stack)) (i32.const 268435456)) (then unreachable))
      (local.set $grown (array.new_default $values
        (i32.add (i32.shl (array.len (local.get $stack)) (i32.const 1)) (local.get $n))))
      (array.copy $values $values (local.get $grown) (i32.const 0) (local.get $stack) (i32.const 0) (local.get $sp))
      (local.set $stack (local.get $grown))))
    (local.set $i (local.get $n))
    (block $fields_done (loop $fields_each
      (br_if $fields_done (i32.eqz (local.get $i)))
      (local.set $i (i32.sub (local.get $i) (i32.const 1)))
      (array.set $values (local.get $stack) (local.get $sp) (array.get $values (local.get $xs) (local.get $i)))
      (local.set $sp (i32.add (local.get $sp) (i32.const 1)))
      (br $fields_each)))
    (br $walk)))
  (local.get $h))
(func $value_hash (param $v eqref) (result eqref)
  (call $int_from_i64 (i64.extend_i32_u (call $value_hash_u32 (local.get $v)))))

;; Keys are identity values. Tag 14 remains reserved for structural records.
(func $key_new (result eqref)
  (struct.new $obj (i32.const 15) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $key_wrap (param $key eqref) (param $value eqref) (result eqref)
  (struct.new $obj (i32.const 16) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (array.new_fixed $values 2 (local.get $key) (local.get $value))
    (ref.null $limbs) (ref.null $call)))
(func $key_read (param $key eqref) (param $value eqref) (result eqref)
  (if (ref.eq (local.get $key) (call $field (local.get $value) (i32.const 0)))
    (then (return_call $result_ok (call $field (local.get $value) (i32.const 1)))))
  (call $result_error (ref.null eq)))

;; Visit each distinct aggregate once, including shared acyclic data. As in
;; BEAM's shared-copy walk, temporarily mark headers and restore them before
;; returning. This walk calls no i code or host and cannot suspend/re-enter.
;; A stop makes the instance unusable; ordinary true/false exits restore all
;; headers. Patched closures still exclude their reserved non-capture slot.
(func $core_portable (param $v eqref) (result eqref)
  (local $stack (ref $values)) (local $grown (ref $values))
  (local $sp i32) (local $tag i32) (local $n i32)
  (local $xs (ref null $values)) (local $o (ref $obj))
  (local $seen (ref $values)) (local $count i32) (local $bad i32)
  ;; A light boundary is its environment array. `$env` has the same Wasm
  ;; array type as `$values`; no other i value is a bare array. Its handle
  ;; can occur in a closure's captures even though source uses only escape.
  (if (ref.test (ref $values) (local.get $v)) (then (return (global.get $value_false))))
  ;; BEAM's shared-copy walk also handles constants before allocating work.
  (local.set $tag (call $value_tag (local.get $v)))
  (if (i32.or (i32.and (i32.ge_u (local.get $tag) (i32.const 10))
                       (i32.le_u (local.get $tag) (i32.const 12)))
               (i32.ge_u (local.get $tag) (i32.const 17)))
    (then (return (global.get $value_false))))
  (if (i32.eqz (i32.or
        (i32.or (i32.eq (local.get $tag) (i32.const 4)) (i32.eq (local.get $tag) (i32.const 5)))
        (i32.or (i32.eq (local.get $tag) (i32.const 6))
          (i32.or (i32.eq (local.get $tag) (i32.const 7))
            (i32.or (i32.eq (local.get $tag) (i32.const 14)) (i32.eq (local.get $tag) (i32.const 16)))))))
    (then (return (global.get $value_true))))
  (if (i32.eq (local.get $tag) (i32.const 6))
    (then (if (call $list_is_empty (local.get $v)) (then (return (global.get $value_true)))))
    (else
      (local.set $xs (struct.get $obj 5 (ref.cast (ref $obj) (local.get $v))))
      (if (ref.is_null (local.get $xs)) (then (return (global.get $value_true))))
      (if (i32.eqz (array.len (local.get $xs))) (then (return (global.get $value_true))))))
  (local.set $seen (array.new_default $values (i32.const 16)))
  (local.set $stack (array.new_default $values (i32.const 16)))
  (array.set $values (local.get $stack) (i32.const 0) (local.get $v))
  (local.set $sp (i32.const 1))
  (block $done (loop $walk
    (br_if $done (i32.eqz (local.get $sp)))
    (local.set $sp (i32.sub (local.get $sp) (i32.const 1)))
    (local.set $v (array.get $values (local.get $stack) (local.get $sp)))
    (if (ref.test (ref $values) (local.get $v))
      (then (local.set $bad (i32.const 1)) (br $done)))
    (local.set $tag (call $value_tag (local.get $v)))
    (br_if $walk (i32.lt_s (local.get $tag) (i32.const 0)))
    (if (i32.or (i32.and (i32.ge_u (local.get $tag) (i32.const 10))
                       (i32.le_u (local.get $tag) (i32.const 12)))
               (i32.ge_u (local.get $tag) (i32.const 17)))
      (then (local.set $bad (i32.const 1)) (br $done)))
    (local.set $xs (ref.null $values))
    (local.set $n (i32.const 0))
    (if (i32.eq (local.get $tag) (i32.const 6))
      (then
        (br_if $walk (call $list_is_empty (local.get $v)))
        (local.set $n (i32.const 2)))
      (else
        (br_if $walk (i32.eqz (i32.or
          (i32.or (i32.eq (local.get $tag) (i32.const 4)) (i32.eq (local.get $tag) (i32.const 5)))
          (i32.or (i32.eq (local.get $tag) (i32.const 7))
            (i32.or (i32.eq (local.get $tag) (i32.const 14)) (i32.eq (local.get $tag) (i32.const 16)))))))
        (local.set $o (ref.cast (ref $obj) (local.get $v)))
        (local.set $xs (struct.get $obj 5 (local.get $o)))
        (br_if $walk (ref.is_null (local.get $xs)))
        (local.set $n (array.len (local.get $xs)))
        (if (i32.and (i32.eq (local.get $tag) (i32.const 7))
                    (i32.and (struct.get $obj 1 (local.get $o)) (i32.const 1)))
          (then
            (local.set $n (i32.sub (local.get $n) (i32.const 1)))))))
    (br_if $walk (i32.eqz (local.get $n)))
    (if (i32.eq (local.get $count) (array.len (local.get $seen))) (then
      (local.set $grown (array.new_default $values (i32.shl (local.get $count) (i32.const 1))))
      (array.copy $values $values (local.get $grown) (i32.const 0)
        (local.get $seen) (i32.const 0) (local.get $count))
      (local.set $seen (local.get $grown))))
    (array.set $values (local.get $seen) (local.get $count) (local.get $v))
    (local.set $count (i32.add (local.get $count) (i32.const 1)))
    (struct.set $val 0 (ref.cast (ref $val) (local.get $v))
      (i32.or (local.get $tag) (i32.const -2147483648)))
    (if (i32.gt_u (i32.add (local.get $sp) (local.get $n)) (array.len (local.get $stack)))
      (then
        (if (i32.ge_u (array.len (local.get $stack)) (i32.const 268435456)) (then unreachable))
        (local.set $grown (array.new_default $values
          (i32.add (i32.shl (array.len (local.get $stack)) (i32.const 1)) (local.get $n))))
        (array.copy $values $values (local.get $grown) (i32.const 0)
          (local.get $stack) (i32.const 0) (local.get $sp))
        (local.set $stack (local.get $grown))))
    (if (i32.eq (local.get $tag) (i32.const 6))
      (then
        (array.set $values (local.get $stack) (local.get $sp) (call $list_tail (local.get $v)))
        (array.set $values (local.get $stack) (i32.add (local.get $sp) (i32.const 1))
          (call $list_head (local.get $v))))
      (else
        (array.copy $values $values (local.get $stack) (local.get $sp)
          (local.get $xs) (i32.const 0) (local.get $n))))
    (local.set $sp (i32.add (local.get $sp) (local.get $n)))
    (br $walk)))
  (block $restored (loop $restore
    (br_if $restored (i32.eqz (local.get $count)))
    (local.set $count (i32.sub (local.get $count) (i32.const 1)))
    (local.set $v (array.get $values (local.get $seen) (local.get $count)))
    (struct.set $val 0 (ref.cast (ref $val) (local.get $v))
      (i32.and (call $value_tag (local.get $v)) (i32.const 2147483647)))
    (br $restore)))
  (call $bool_new (i32.eqz (local.get $bad))))
"#;

const HOST_NEW: &str = r#"(func $host_ref_new (param $r externref) (result eqref)
  (struct.new $hostref (local.get $r) (call $host_reference_id (local.get $r))))"#;
const NOMINAL_HOST_NEW: &str = r#"(func $host_ref_new (param $r externref) (param $type i32) (result eqref)
  (struct.new $hostref (local.get $r) (local.get $type) (call $host_reference_id (local.get $r))))"#;

const HOST_COMPARE: &str = r#"        (local.set $c (call $compare_i64 (call $host_id (local.get $a)) (call $host_id (local.get $b))))"#;
const NOMINAL_HOST_COMPARE: &str = r#"        (local.set $c (call $compare_i32
          (struct.get $hostref 1 (ref.cast (ref $hostref) (local.get $a)))
          (struct.get $hostref 1 (ref.cast (ref $hostref) (local.get $b)))))
        (if (local.get $c) (then (return (local.get $c))))
        (local.set $c (call $compare_i64 (call $host_id (local.get $a)) (call $host_id (local.get $b))))"#;

const HOST_HASH: &str = r#"      (local.set $word (call $host_id (local.get $v)))"#;
const NOMINAL_HOST_HASH: &str = r#"      (local.set $h (call $hash_mix (local.get $h)
        (struct.get $hostref 1 (ref.cast (ref $hostref) (local.get $v)))))
      (local.set $word (call $host_id (local.get $v)))"#;

const HOST_ID: &str = r#"(func $host_id (param $v eqref) (result i64)
  (struct.get $hostref 1 (ref.cast (ref $hostref) (local.get $v))))"#;
const NOMINAL_HOST_ID: &str = r#"(func $host_id (param $v eqref) (result i64)
  (struct.get $hostref 2 (ref.cast (ref $hostref) (local.get $v))))"#;
