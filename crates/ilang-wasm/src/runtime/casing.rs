//! `core.to_upper` and `core.to_lower` (spec §13): the simple case mappings
//! of Unicode, one codepoint to one codepoint, with no locale.
//!
//! The tables are in `case_tables.rs`, made by `scripts/gen-case-tables.py`.
//! Each one goes into the module as a passive data segment of little-endian
//! i32 triples, one per range:
//!
//! - `first`;
//! - `last`, with bit 24 set when the range has step 2;
//! - `delta`, added to the codepoint.
//!
//! The first call makes an `(array i32)` from the segment and keeps it in a
//! global. A lookup is a binary search for the last range whose `first` is
//! not greater than the codepoint. ASCII letters do not use the tables.

use super::case_tables::{TO_LOWER, TO_UPPER, UNICODE_VERSION};
use std::fmt::Write as _;

pub fn wat() -> String {
    let mut out = format!(
        ";; Simple case mappings of Unicode {UNICODE_VERSION}.\n(type $case_ranges (array i32))\n"
    );
    segment(&mut out, "case_upper", TO_UPPER);
    segment(&mut out, "case_lower", TO_LOWER);
    out.push_str(WAT);
    out
}

/// The data segment, the global and the getter of one table.
fn segment(out: &mut String, name: &str, ranges: &[(u32, u32, u32, i32)]) {
    let mut data = String::new();
    for &(first, last, step, delta) in ranges {
        debug_assert!(last < 1 << 24 && (step == 1 || step == 2));
        let last = last | u32::from(step == 2) << 24;
        for word in [first, last, delta as u32] {
            for byte in word.to_le_bytes() {
                write!(data, "\\{byte:02x}").unwrap();
            }
        }
    }
    let words = ranges.len() * 3;
    write!(
        out,
        r#"
(data ${name}_data "{data}")
(global ${name} (mut (ref null $case_ranges)) (ref.null $case_ranges))
(func ${name}_table (result (ref $case_ranges))
  (if (ref.is_null (global.get ${name}))
    (then (global.set ${name}
      (array.new_data $case_ranges ${name}_data (i32.const 0) (i32.const {words})))))
  (ref.as_non_null (global.get ${name})))
"#
    )
    .unwrap();
}

const WAT: &str = r#"
;; The codepoint that `cp` maps to by the table, or `cp` when no range has it.
(func $case_map (param $table (ref $case_ranges)) (param $cp i32) (result i32)
  (local $low i32) (local $high i32) (local $middle i32) (local $at i32) (local $last i32)
  (local.set $high (i32.div_u (array.len (local.get $table)) (i32.const 3)))
  ;; Ranges before $low start at or before cp; ranges from $high start after it.
  (block $found (loop $search
    (br_if $found (i32.ge_u (local.get $low) (local.get $high)))
    (local.set $middle (i32.shr_u (i32.add (local.get $low) (local.get $high)) (i32.const 1)))
    (if (i32.le_u (array.get $case_ranges (local.get $table) (i32.mul (local.get $middle) (i32.const 3)))
                  (local.get $cp))
      (then (local.set $low (i32.add (local.get $middle) (i32.const 1))))
      (else (local.set $high (local.get $middle))))
    (br $search)))
  (if (i32.eqz (local.get $low)) (then (return (local.get $cp))))
  (local.set $at (i32.mul (i32.sub (local.get $low) (i32.const 1)) (i32.const 3)))
  (local.set $last (array.get $case_ranges (local.get $table) (i32.add (local.get $at) (i32.const 1))))
  (if (i32.gt_u (local.get $cp) (i32.and (local.get $last) (i32.const 0xffffff)))
    (then (return (local.get $cp))))
  (if (i32.and (i32.shr_u (local.get $last) (i32.const 24))
        (i32.sub (local.get $cp) (array.get $case_ranges (local.get $table) (local.get $at))))
    (then (return (local.get $cp))))
  (i32.add (local.get $cp) (array.get $case_ranges (local.get $table) (i32.add (local.get $at) (i32.const 2)))))

;; The scalar value at `at`, a character boundary of correct UTF-8 with at
;; least one more character.
(func $case_decode (param $b (ref $bytes)) (param $at i32) (result i32)
  (local $lead i32)
  (local.set $lead (array.get_u $bytes (local.get $b) (local.get $at)))
  (if (i32.lt_u (local.get $lead) (i32.const 0x80)) (then (return (local.get $lead))))
  (if (i32.lt_u (local.get $lead) (i32.const 0xe0))
    (then (return (i32.or
      (i32.shl (i32.and (local.get $lead) (i32.const 0x1f)) (i32.const 6))
      (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 1))) (i32.const 0x3f))))))
  (if (i32.lt_u (local.get $lead) (i32.const 0xf0))
    (then (return (i32.or (i32.or
      (i32.shl (i32.and (local.get $lead) (i32.const 0x0f)) (i32.const 12))
      (i32.shl (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 1))) (i32.const 0x3f)) (i32.const 6)))
      (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 2))) (i32.const 0x3f))))))
  (i32.or (i32.or (i32.or
    (i32.shl (i32.and (local.get $lead) (i32.const 0x07)) (i32.const 18))
    (i32.shl (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 1))) (i32.const 0x3f)) (i32.const 12)))
    (i32.shl (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 2))) (i32.const 0x3f)) (i32.const 6)))
    (i32.and (array.get_u $bytes (local.get $b) (i32.add (local.get $at) (i32.const 3))) (i32.const 0x3f))))

;; Map every character of the string. ASCII letters from `ascii_first` to
;; `ascii_first + 25` move by `ascii_shift`; other characters use the table.
;; A string with nothing to change is given back as it is. Otherwise the
;; part before the first change is copied once, and the rest is mapped into
;; a buffer that grows when needed.
(func $case_string (param $s eqref) (param $table (ref $case_ranges))
  (param $ascii_first i32) (param $ascii_shift i32) (result eqref)
  (local $b (ref $bytes)) (local $n i32) (local $at i32) (local $cp i32) (local $mapped i32)
  (local $out (ref null $bytes)) (local $grown (ref null $bytes)) (local $size i32)
  (local $capacity i64)
  (local.set $b (call $string_bytes (local.get $s)))
  (local.set $n (array.len (local.get $b)))
  ;; Find the first character that changes.
  (block $changed (loop $scan
    (if (i32.ge_u (local.get $at) (local.get $n)) (then (return (local.get $s))))
    (local.set $cp (call $case_decode (local.get $b) (local.get $at)))
    (if (i32.lt_u (local.get $cp) (i32.const 0x80))
      (then
        (br_if $changed (i32.lt_u (i32.sub (local.get $cp) (local.get $ascii_first)) (i32.const 26)))
        (local.set $at (i32.add (local.get $at) (i32.const 1))))
      (else
        (br_if $changed (i32.ne (call $case_map (local.get $table) (local.get $cp)) (local.get $cp)))
        (local.set $at (i32.add (local.get $at) (call $utf8_width (local.get $cp))))))
    (br $scan)))
  ;; Room for the rest at the same size, and some more.
  (local.set $capacity (i64.add (i64.extend_i32_u (local.get $n)) (i64.const 16)))
  (if (i64.gt_u (local.get $capacity) (i64.const 4294967295))
    (then (local.set $capacity (i64.const 4294967295))))
  (local.set $out (array.new_default $bytes (i32.wrap_i64 (local.get $capacity))))
  (array.copy $bytes $bytes (local.get $out) (i32.const 0) (local.get $b) (i32.const 0) (local.get $at))
  (local.set $size (local.get $at))
  (block $done (loop $map
    (br_if $done (i32.ge_u (local.get $at) (local.get $n)))
    (local.set $cp (call $case_decode (local.get $b) (local.get $at)))
    (if (i32.lt_u (local.get $cp) (i32.const 0x80))
      (then
        (local.set $mapped (local.get $cp))
        (if (i32.lt_u (i32.sub (local.get $cp) (local.get $ascii_first)) (i32.const 26))
          (then (local.set $mapped (i32.add (local.get $cp) (local.get $ascii_shift)))))
        (local.set $at (i32.add (local.get $at) (i32.const 1))))
      (else
        (local.set $mapped (call $case_map (local.get $table) (local.get $cp)))
        (local.set $at (i32.add (local.get $at) (call $utf8_width (local.get $cp))))))
    ;; A character takes at most 4 bytes.
    (if (i32.gt_u (i32.add (local.get $size) (i32.const 4)) (array.len (local.get $out)))
      (then
        (local.set $capacity (i64.shl (i64.extend_i32_u (array.len (local.get $out))) (i64.const 1)))
        (if (i64.gt_u (local.get $capacity) (i64.const 4294967295))
          (then (local.set $capacity (i64.const 4294967295))))
        (if (i64.le_u (local.get $capacity) (i64.extend_i32_u (array.len (local.get $out))))
          (then unreachable))
        (local.set $grown (array.new_default $bytes (i32.wrap_i64 (local.get $capacity))))
        (array.copy $bytes $bytes (local.get $grown) (i32.const 0) (local.get $out) (i32.const 0) (local.get $size))
        (local.set $out (local.get $grown))))
    (local.set $size (call $utf8_write (ref.as_non_null (local.get $out)) (local.get $size) (local.get $mapped)))
    (br $map)))
  (local.set $grown (array.new_default $bytes (local.get $size)))
  (array.copy $bytes $bytes (local.get $grown) (i32.const 0) (local.get $out) (i32.const 0) (local.get $size))
  (call $string_from_bytes (ref.as_non_null (local.get $grown))))

(func $core_to_upper (param $s eqref) (result eqref)
  (call $case_string (local.get $s) (call $case_upper_table) (i32.const 97) (i32.const -32)))
(func $core_to_lower (param $s eqref) (result eqref)
  (call $case_string (local.get $s) (call $case_lower_table) (i32.const 65) (i32.const 32)))
"#;
