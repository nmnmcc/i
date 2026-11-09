//! `core.bit_array_concat` (spec §13): a list of bit arrays joined in
//! order, with one copy.
//!
//! The first pass adds up the lengths, and the second copies each part into
//! one new array. A part that starts on a byte boundary in both arrays is
//! copied with `array.copy`; any other part is copied eight bits at a time.
//! The new array is zero to begin with and is only written with `or`, so
//! the bits after the end of every part, and of the whole, stay 0.

pub fn wat() -> String {
    WAT.to_owned()
}

const WAT: &str = r#"
(func $core_bit_array_concat (param $list eqref) (result eqref)
  (local $cursor eqref) (local $fields (ref null $values)) (local $part eqref)
  (local $length i64) (local $at i64) (local $data (ref null $bytes)) (local $count i32)
  (local.set $cursor (local.get $list))
  (block $counted (loop $count
    (local.set $fields (struct.get $obj 5 (ref.cast (ref $obj) (local.get $cursor))))
    (br_if $counted (ref.is_null (local.get $fields)))
    (local.set $part (array.get $values (local.get $fields) (i32.const 0)))
    (local.set $length (i64.add (local.get $length) (call $bits_len64 (local.get $part))))
    (local.set $count (i32.add (local.get $count) (i32.const 1)))
    (local.set $cursor (array.get $values (local.get $fields) (i32.const 1)))
    (br $count)))
  (if (i32.eqz (local.get $count)) (then (return (call $bits_empty))))
  ;; One part needs no copy: bit arrays cannot be changed.
  (if (i32.eq (local.get $count) (i32.const 1)) (then (return (local.get $part))))
  (local.set $data (call $bits_alloc (local.get $length)))
  (local.set $cursor (local.get $list))
  (block $written (loop $write
    (local.set $fields (struct.get $obj 5 (ref.cast (ref $obj) (local.get $cursor))))
    (br_if $written (ref.is_null (local.get $fields)))
    (local.set $part (array.get $values (local.get $fields) (i32.const 0)))
    (call $bits_join_part (ref.as_non_null (local.get $data)) (local.get $at) (local.get $part))
    (local.set $at (i64.add (local.get $at) (call $bits_len64 (local.get $part))))
    (local.set $cursor (array.get $values (local.get $fields) (i32.const 1)))
    (br $write)))
  (call $bits_new (ref.as_non_null (local.get $data)) (i32.const 0) (local.get $length)))

;; Put the bits of `part` into `out` from bit `at` on. The bits of `out` from
;; `at` on are 0.
(func $bits_join_part (param $out (ref $bytes)) (param $at i64) (param $part eqref)
  (local $source (ref $bytes)) (local $offset i64) (local $length i64) (local $i i64)
  (local $position i64) (local $q i32) (local $r i32) (local $value i32) (local $take i64)
  (local $p i32) (local $t i32) (local $whole i32)
  (local.set $source (call $bits_data (local.get $part)))
  (local.set $offset (call $bits_offset (local.get $part)))
  (local.set $length (call $bits_len64 (local.get $part)))
  (if (i64.eqz (i64.and (i64.or (local.get $at) (local.get $offset)) (i64.const 7)))
    (then
      (local.set $whole (i32.wrap_i64 (i64.shr_u (local.get $length) (i64.const 3))))
      (array.copy $bytes $bytes
        (local.get $out) (i32.wrap_i64 (i64.shr_u (local.get $at) (i64.const 3)))
        (local.get $source) (i32.wrap_i64 (i64.shr_u (local.get $offset) (i64.const 3)))
        (local.get $whole))
      (local.set $i (i64.shl (i64.extend_i32_u (local.get $whole)) (i64.const 3)))))
  ;; Eight bits at a time: read them from two source bytes, cut off what is
  ;; after the end of the part, and write them over two bytes of `out`.
  (block $done (loop $copy
    (br_if $done (i64.ge_u (local.get $i) (local.get $length)))
    (local.set $position (i64.add (local.get $offset) (local.get $i)))
    (local.set $q (i32.wrap_i64 (i64.shr_u (local.get $position) (i64.const 3))))
    (local.set $r (i32.wrap_i64 (i64.and (local.get $position) (i64.const 7))))
    (local.set $value (i32.shl (array.get_u $bytes (local.get $source) (local.get $q)) (local.get $r)))
    (if (i32.and (i32.ne (local.get $r) (i32.const 0))
                 (i32.lt_u (i32.add (local.get $q) (i32.const 1)) (array.len (local.get $source))))
      (then (local.set $value (i32.or (local.get $value)
        (i32.shr_u (array.get_u $bytes (local.get $source) (i32.add (local.get $q) (i32.const 1)))
                   (i32.sub (i32.const 8) (local.get $r)))))))
    (local.set $take (i64.sub (local.get $length) (local.get $i)))
    (if (i64.gt_u (local.get $take) (i64.const 8)) (then (local.set $take (i64.const 8))))
    (local.set $value (i32.and (local.get $value)
      (i32.and (i32.shl (i32.const 0xff) (i32.sub (i32.const 8) (i32.wrap_i64 (local.get $take))))
               (i32.const 0xff))))
    (local.set $position (i64.add (local.get $at) (local.get $i)))
    (local.set $p (i32.wrap_i64 (i64.shr_u (local.get $position) (i64.const 3))))
    (local.set $t (i32.wrap_i64 (i64.and (local.get $position) (i64.const 7))))
    (array.set $bytes (local.get $out) (local.get $p)
      (i32.or (array.get_u $bytes (local.get $out) (local.get $p))
              (i32.shr_u (local.get $value) (local.get $t))))
    ;; The low bits that do not fit go to the next byte; they are 0 unless
    ;; they are bits of the part, so that byte is then in `out`.
    (if (i32.and (i32.ne (local.get $t) (i32.const 0))
                 (i32.ne (i32.and (i32.shl (local.get $value) (i32.sub (i32.const 8) (local.get $t))) (i32.const 0xff))
                         (i32.const 0)))
      (then (array.set $bytes (local.get $out) (i32.add (local.get $p) (i32.const 1))
        (i32.or (array.get_u $bytes (local.get $out) (i32.add (local.get $p) (i32.const 1)))
                (i32.and (i32.shl (local.get $value) (i32.sub (i32.const 8) (local.get $t))) (i32.const 0xff))))))
    (local.set $i (i64.add (local.get $i) (i64.const 8)))
    (br $copy)))
  )
"#;
