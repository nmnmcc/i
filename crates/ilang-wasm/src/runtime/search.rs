//! `core.index_of` (spec §13): the byte offset of the first place of one
//! string in another, at or after a given offset.
//!
//! The search is Knuth–Morris–Pratt, so it takes time linear in the sizes of
//! both strings. A one-byte string is found by a plain scan, and a longer one
//! needs a table of one i32 for each of its bytes. A match of correct UTF-8
//! always starts at a character boundary, because its first byte is not a
//! continuation byte.

pub fn wat() -> String {
    WAT.to_owned()
}

const WAT: &str = r#"
(type $search_table (array (mut i32)))

;; Error(Nil) when `from` is negative, after the end, or inside a character,
;; and when `find` is not there. An empty `find` is at `from`.
(func $core_index_of (param $s eqref) (param $find eqref) (param $from eqref) (result eqref)
  (local $text (ref $bytes)) (local $pattern (ref $bytes))
  (local $n i32) (local $m i32) (local $start i32) (local $found i32)
  (local.set $text (call $string_bytes (local.get $s)))
  (local.set $pattern (call $string_bytes (local.get $find)))
  (local.set $n (array.len (local.get $text)))
  (local.set $m (array.len (local.get $pattern)))
  (if (i32.lt_s (call $int_sign (local.get $from)) (i32.const 0))
    (then (return (call $result_error (ref.null eq)))))
  (if (i32.gt_s (call $int_compare (local.get $from) (call $int_from_i64 (i64.extend_i32_u (local.get $n))))
                (i32.const 0))
    (then (return (call $result_error (ref.null eq)))))
  (local.set $start (i32.wrap_i64 (call $int_to_i64 (local.get $from))))
  (if (i32.eqz (call $string_boundary (local.get $text) (local.get $start)))
    (then (return (call $result_error (ref.null eq)))))
  (if (i32.eqz (local.get $m)) (then (return (call $result_ok (local.get $from)))))
  (if (i32.gt_u (local.get $m) (i32.sub (local.get $n) (local.get $start)))
    (then (return (call $result_error (ref.null eq)))))
  (local.set $found
    (if (result i32) (i32.eq (local.get $m) (i32.const 1))
      (then (call $search_byte (local.get $text) (array.get_u $bytes (local.get $pattern) (i32.const 0))
        (local.get $start)))
      (else (call $search_kmp (local.get $text) (local.get $pattern) (local.get $start)))))
  (if (i32.lt_s (local.get $found) (i32.const 0))
    (then (return (call $result_error (ref.null eq)))))
  (call $result_ok (call $int_from_i64 (i64.extend_i32_u (local.get $found)))))

;; The first index at or after `at` that holds `byte`, or -1.
(func $search_byte (param $text (ref $bytes)) (param $byte i32) (param $at i32) (result i32)
  (local $n i32)
  (local.set $n (array.len (local.get $text)))
  (block $none (loop $scan
    (br_if $none (i32.ge_u (local.get $at) (local.get $n)))
    (if (i32.eq (array.get_u $bytes (local.get $text) (local.get $at)) (local.get $byte))
      (then (return (local.get $at))))
    (local.set $at (i32.add (local.get $at) (i32.const 1)))
    (br $scan)))
  (i32.const -1))

;; The first index at or after `at` where `pattern` (two bytes or more) is
;; in `text`, or -1. `border[i]` is the length of the longest proper prefix
;; of pattern[0..=i] that is also its suffix.
(func $search_kmp (param $text (ref $bytes)) (param $pattern (ref $bytes)) (param $at i32) (result i32)
  (local $border (ref $search_table)) (local $n i32) (local $m i32) (local $i i32) (local $k i32)
  (local $byte i32)
  (local.set $n (array.len (local.get $text)))
  (local.set $m (array.len (local.get $pattern)))
  (local.set $border (array.new_default $search_table (local.get $m)))
  (local.set $i (i32.const 1))
  (block $built (loop $build
    (br_if $built (i32.ge_u (local.get $i) (local.get $m)))
    (local.set $byte (array.get_u $bytes (local.get $pattern) (local.get $i)))
    (block $fits (loop $shorter
      (br_if $fits (i32.eqz (local.get $k)))
      (br_if $fits (i32.eq (array.get_u $bytes (local.get $pattern) (local.get $k)) (local.get $byte)))
      (local.set $k (array.get $search_table (local.get $border) (i32.sub (local.get $k) (i32.const 1))))
      (br $shorter)))
    (if (i32.eq (array.get_u $bytes (local.get $pattern) (local.get $k)) (local.get $byte))
      (then (local.set $k (i32.add (local.get $k) (i32.const 1)))))
    (array.set $search_table (local.get $border) (local.get $i) (local.get $k))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $build)))
  ;; $k is the number of pattern bytes that match the text before $at.
  (local.set $k (i32.const 0))
  (block $none (loop $scan
    (br_if $none (i32.ge_u (local.get $at) (local.get $n)))
    (local.set $byte (array.get_u $bytes (local.get $text) (local.get $at)))
    (block $fits (loop $shorter
      (br_if $fits (i32.eqz (local.get $k)))
      (br_if $fits (i32.eq (array.get_u $bytes (local.get $pattern) (local.get $k)) (local.get $byte)))
      (local.set $k (array.get $search_table (local.get $border) (i32.sub (local.get $k) (i32.const 1))))
      (br $shorter)))
    (if (i32.eq (array.get_u $bytes (local.get $pattern) (local.get $k)) (local.get $byte))
      (then (local.set $k (i32.add (local.get $k) (i32.const 1)))))
    (local.set $at (i32.add (local.get $at) (i32.const 1)))
    (if (i32.eq (local.get $k) (local.get $m))
      (then (return (i32.sub (local.get $at) (local.get $m)))))
    (br $scan)))
  (i32.const -1))
"#;
