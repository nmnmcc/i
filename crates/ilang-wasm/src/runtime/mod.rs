//! The runtime that the code generator links into every program.
//!
//! It is hand-written WAT. Language values live on the Wasm GC heap; linear
//! memory is used only for buffers at the host boundary (spec Appendix B).
//!
//! # Putting a module together
//!
//! A program module is, in this order:
//!
//! 1. [`TYPES`];
//! 2. [`HOST`], the only runtime imports (`ilang:host/v1` `panic` and
//!    `reference_id`), then the imports of host services and external
//!    functions (the text format puts every import before any definition);
//! 3. [`wat()`], the runtime definitions (it also defines and exports
//!    `memory`, `ilang_alloc` and `ilang_free`);
//! 4. the generated code. It defines and exports `ilang_start`, the entry
//!    points and the host exports, and it adds the `ilang.abi` custom section.
//!
//! The runtime has no Wasm start function and no exception tags of its own.
//!
//! # Values
//!
//! Every value is an `eqref`. Apart from Nil, Int and host references, it is a
//! `$obj` struct whose field 0 is the value tag:
//!
//! | Tag | Value | Representation |
//! |---|---|---|
//! | 0 | Nil | the null reference |
//! | 1 | Bool | field 1 is 0 or 1; use `$bool_new` (or the globals `$value_true` and `$value_false`) |
//! | 2 | Float | field 3; always finite |
//! | 3 | String | field 4: UTF-8 bytes |
//! | 4 | Tuple | field 5: the parts |
//! | 5 | Custom type value | field 1: constructor id, field 2: type id, field 5: the fields, or null when there are none |
//! | 6 | List | field 5: null for `[]`, else `[head, tail]` |
//! | 7 | Function | field 2: the number of parameters, without environment parameters |
//! | 8 | Int | `i31ref` for -2^30..2^30-1, else field 1: sign, field 6: magnitude limbs |
//! | 9 | BitArray | field 1: bit offset, field 2: bit length, field 4: bytes, highest bit first |
//! | 10 | Host reference | a `$hostref` struct (not `$obj`): the `externref` and its `reference_id` |
//! | 11 | Boundary | an `$obj` with tag 11; the other fields belong to the code generator |
//! | 12 | Continuation | an `$obj` with tag 12; the other fields belong to the code generator |
//! | 13 | UtfCodepoint | field 1: the scalar value |
//!
//! `$obj` is not final, so the code generator may declare subtypes of it, for
//! example closures with a typed function reference of their own after the
//! eight `$obj` fields. Functions, boundaries, continuations and host
//! references compare by identity (`ref.eq`; host references by their
//! `reference_id`). Their hash depends only on the tag.
//!
//! Constructor ids 0 to 15 belong to the runtime: 0 `Ok` and 1 `Error` (type
//! id 0, `Result`), 2 `AssertionFailure` (type id 1) and 3 `Assert` (type id
//! 2). The code generator numbers the other constructors from
//! [`FIRST_CONSTRUCTOR`] and installs their names, in order, with
//! `$constructors_install (param (ref $values))`, an array of String values.
//! `Bool` and `Nil` use tags 1 and 0, not constructor ids.
//!
//! # Shadow stack and stop messages (spec §11.7)
//!
//! Stop messages need the source positions of the running calls. The code
//! generator keeps them on a shadow stack of site ids: the growable GC array
//! `$shadow` (type `$frames`, `(array (mut i32))`) with the global `$sp`, the
//! number of frames in use. Slot `sp - 1` belongs to the running function and
//! holds the site of the expression it is working out.
//!
//! - On function entry: `call $shadow_enter (site)`, which grows the array when
//!   it is full. `site` may be -1, which is not printed.
//! - Before a call, an operator that may stop, a `<<`, or a binding that may
//!   stop: `call $shadow_site (site)` (store into slot `sp - 1`).
//! - On a normal return: `call $shadow_leave`.
//! - Before a tail call: `call $shadow_leave`, so the replaced call is gone.
//! - Where an exception is caught (`try_table`), restore the depth saved with
//!   `$shadow_depth` when the handler was entered: `call $shadow_restore`.
//!   A capture (`$capture`) restores it only at its boundary.
//! - When a continuation is resumed or cancelled, each frame that is rebuilt
//!   enters again with the site of the call it was paused at, on top of the
//!   frames of the place of `resume` (spec §9.4).
//!
//! Site ids index the site table, installed once at the very start of
//! `ilang_start` with `$sites_install (param (ref $sites) (ref $values))`:
//!
//! - `$sites` is `(array i32)` with three numbers per site: module index,
//!   line, column. `array.new_data $sites $segment` makes it from a passive
//!   data segment (little-endian i32 triples).
//! - the second array holds, per module index, the module path as a String
//!   value, already with the `location@major ` prefix when the package has a
//!   location (spec §11.7).
//!
//! A site line is `module path:line:column`. Ids out of range are not printed.
//!
//! A stop builds the message (the reason, then one line per position, with a
//! line feed between lines and none at the end), copies it into memory, calls
//! the `panic` import exactly once and executes `unreachable`. These functions
//! take no results and never return; follow a call with `unreachable`:
//!
//! | Function | Reason |
//! |---|---|
//! | `$panic (param $message eqref)` | `panic: ` and the message |
//! | `$stop_assertion (param $message eqref)` | `assertion failed: ` and the message |
//! | `$stop_error (param $error eqref) (param $site i32)` | `error: ` and the text form of the error; only `site` (the entry point's binding) is printed |
//! | `$stop_boundary_inactive` | `control: boundary is not active` |
//! | `$stop_continuation_used` | `control: continuation was already used` |
//! | `$stop_float` | `float: result is not finite` |
//! | `$stop_bits_short` | `bit array: value is too short` |
//! | `$stop_bits_negative` | `bit array: length is negative` |
//! | `$stop_bits_not_whole` | `bit array: bytes are not whole` |
//! | `$stop_bits_float` | `bit array: float is not finite` |
//! | `$stop_host_value` | `host: value is not correct` |
//! | `$stop (param $reason eqref)` | any reason, with every shadow-stack position |
//! | `$stop_at (param $reason eqref) (param $site i32)` | any reason, with one position |
//!
//! Running out of memory or stack traps without a stop message.
//!
//! # Operations
//!
//! Every value parameter and result is an `eqref` unless shown otherwise.
//!
//! | Use | Function |
//! |---|---|
//! | Int literals | `$int_from_i64 (i64)`, `$int_pack (sign i32) (ref $limbs)` |
//! | `+ - * / %`, unary `-` | `$int_add`, `$int_sub`, `$int_mul`, `$int_div`, `$int_rem`, `$int_neg`; `/` and `%` by 0 give 0 |
//! | `< <= > >=` | `$int_compare (a, b) -> i32` (-1, 0, 1) |
//! | Float literals | `$float_new (f64)` (stops when not finite), `$float_get -> f64` |
//! | `+. -. *. /.`, unary `-.` | `$float_add`, `$float_sub`, `$float_mul`, `$float_div` (by 0.0 gives 0.0), `$float_neg`; a result that is not finite stops |
//! | `<. <=. >. >=.` | compare `$float_get` results with `f64.lt` and so on |
//! | `<>` | `$string_append` |
//! | `==`, `!=` | `$equal (a, b) -> i32` |
//! | Bool | `$bool_new (i32)`, `$bool_get -> i32` |
//! | Lists | `$list_nil`, `$list_cons (head, tail)`, `$list_is_empty -> i32`, `$list_head`, `$list_tail` |
//! | Tuples, custom values | `$tuple (ref $values)`, `$record (ctor i32) (type i64) (ref $values)`, `$field (v, i32)` |
//! | `Ok`, `Error` | `$result_ok (v)`, `$result_error (v)` |
//! | Codepoints | `$codepoint_new (i32)` |
//! | Host references | `$host_ref_new (externref)`, `$host_ref_get -> externref` |
//! | String literals | `$string_from_bytes (ref $bytes)` |
//! | String prefix patterns | `$string_starts_with (s, prefix) -> i32`, `$string_suffix (s, start i32)` |
//!
//! The functions of `i/core` (spec §13), with the specified behaviour:
//!
//! | `i/core` | Function |
//! |---|---|
//! | `not` | `$core_not (v)` |
//! | `to_float` | `$core_to_float (int)`: stops when not finite |
//! | `truncate`, `floor`, `ceiling`, `sqrt` | `$core_truncate`, `$core_floor`, `$core_ceiling`, `$core_sqrt` (a negative number stops) |
//! | `float_to_bits`, `float_from_bits` | `$core_float_to_bits`, `$core_float_from_bits` (lowest 64 bits; stops when not finite) |
//! | `bit_and`, `bit_or`, `bit_xor`, `bit_not` | `$core_bit_and (a, b)`, `$core_bit_or`, `$core_bit_xor`, `$core_bit_not (a)` |
//! | `shift_left`, `shift_right` | `$core_shift_left (x, n)`, `$core_shift_right (x, n)`; a negative n shifts the other way |
//! | `concat`, `byte_size`, `byte_slice` | `$core_concat (list)`, `$core_byte_size (s)`, `$core_byte_slice (s, start, length) -> Result` |
//! | `codepoints`, `from_codepoints` | `$core_codepoints (s)`, `$core_from_codepoints (list)` |
//! | `codepoint_to_int`, `int_to_codepoint` | `$core_codepoint_to_int (c)`, `$core_int_to_codepoint (n) -> Result` |
//! | `bit_size`, `bit_array_to_string` | `$core_bit_size (b)`, `$core_bit_array_to_string (b) -> Result` |
//! | `inspect`, `hash` | `$core_inspect (v)`, `$core_hash (v)` (0 to 2^32 - 1) |
//!
//! Bit arrays (spec §8). Construction:
//!
//! | Segment | Function |
//! |---|---|
//! | `<<a, b>>` | `$bits_concat (a, b)`; `$bits_empty` |
//! | integers | `$bits_from_int (v, width i64, little i32)` |
//! | floats | `$bits_from_float (v, width i64, little i32)`: stops when the rounded value is not finite |
//! | `u(n)`, `bits(n)`, `bytes(n)` lengths | `$bits_length (n, unit i64) -> i64`: stops when negative |
//! | `bits(n)`, `bytes(n)` | `$bits_take (v, bits i64)`: stops when the value is too short |
//! | `bytes` | `$bits_whole (v)`: stops when the length is not a multiple of 8 |
//! | strings, codepoints | `$bits_from_utf (v, encoding i32, little i32, codepoint i32)` |
//!
//! Matching uses `$bits_has`, `$bits_width` (a negative result never
//! matches), `$bits_read_int`, `$bits_float_finite`, `$bits_read_float`,
//! `$bits_read_utf`, `$bits_slice` and `$bits_match_equal`; see `bitarrays.rs`.
//!
//! # The host boundary (Appendix B.2, B.3)
//!
//! Values from the host stop with `host: value is not correct` when they are
//! not correct, including buffers outside memory and bit arrays whose bits
//! after the end are not 0:
//!
//! | Type | From the host | To the host |
//! |---|---|---|
//! | Bool | `$abi_bool_in (i32)` | `$abi_bool_out (v) -> i32` |
//! | Float | `$abi_float_in (f64)` | `$abi_float_out (v) -> f64` |
//! | Int | `$abi_int_in (sign, address, count i32)` | `$abi_int_out (v) -> i32 i32 i32` |
//! | String | `$abi_string_in (address, length i32)` | `$abi_string_out (v) -> i32 i32` |
//! | BitArray | `$abi_bits_in (address, bits i32)` | `$abi_bits_out (v) -> i32 i32` |
//! | externref | `$abi_ref_in (externref)` | `$abi_ref_out (v) -> externref` |
//! | error tag | `$abi_tag_in (tag i32, members i32) -> i32` | |
//!
//! The `in` functions copy; they never free. The `out` functions make a new
//! buffer with `ilang_alloc` (address 0 when it would be empty). So for an
//! import, the generated code makes the parameter buffers with the `out`
//! functions, frees them after the call with `$free`, and frees the result
//! buffers after the `in` functions have copied them.
mod abi;
mod bigint;
mod bitarrays;
mod core;
mod stops;
mod values;

pub const TYPES: &str = r#"
(type $bytes (array (mut i8)))
(type $values (array (mut eqref)))
(type $limbs (array (mut i32)))
(type $call (func (param eqref eqref) (result eqref)))
(type $obj (sub (struct
 (field i32) (field i32) (field i64) (field f64)
 (field (ref null $bytes)) (field (ref null $values))
 (field (ref null $limbs)) (field (ref null $call)))))
(type $hostref (struct (field externref) (field i64)))
(type $sites (array i32))
(type $frames (array (mut i32)))
"#;

/// The runtime imports (spec B.4). Nothing else is imported from `ilang:host/v1`.
pub const HOST: &str = r#"
(import "ilang:host/v1" "panic" (func $host_panic (param i32 i32)))
(import "ilang:host/v1" "reference_id" (func $host_reference_id (param externref) (result i64)))
"#;

/// The first constructor id that the code generator may use.
pub const FIRST_CONSTRUCTOR: u32 = 16;

/// The runtime definitions. They need [`TYPES`] and [`HOST`] in the module.
pub fn wat() -> &'static str {
    static RUNTIME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| {
        [
            MEMORY.to_owned(),
            bigint::wat(),
            bitarrays::wat(),
            values::wat(),
            stops::wat(),
            core::wat(),
            abi::wat(),
        ]
        .concat()
    })
}

const MEMORY: &str = r#"
(memory (export "memory") 1)
(global $heap (mut i32) (i32.const 1024))
(global $free_head (mut i32) (i32.const 0))
(func $alloc (export "ilang_alloc") (param $len i32) (result i32)
 (local $size i32) (local $p i32) (local $prev i32) (local $end i64) (local $available i64)
 (if (i32.gt_u (local.get $len) (i32.const 2147483632)) (then unreachable))
 (local.set $size (i32.and (i32.add (local.get $len) (i32.const 7)) (i32.const -8)))
 (if (i32.eqz (local.get $size)) (then (local.set $size (i32.const 8))))
 (local.set $p (global.get $free_head))
 (block $not_found (loop $search
   (br_if $not_found (i32.eqz (local.get $p)))
   (if (i32.ge_u (i32.load (local.get $p)) (local.get $size))
     (then
       (if (local.get $prev)
         (then (i32.store offset=4 (local.get $prev) (i32.load offset=4 (local.get $p))))
         (else (global.set $free_head (i32.load offset=4 (local.get $p)))))
       (return (i32.add (local.get $p) (i32.const 8)))))
   (local.set $prev (local.get $p))
   (local.set $p (i32.load offset=4 (local.get $p))) (br $search)))
 (local.set $p (global.get $heap))
 ;; Calculate addresses in i64: a full memory32 has 2^32 bytes, which cannot
 ;; be represented by i32. Keep the next heap pointer representable as well.
 (local.set $end (i64.add (i64.extend_i32_u (local.get $p))
   (i64.add (i64.extend_i32_u (local.get $size)) (i64.const 8))))
 (if (i64.gt_u (local.get $end) (i64.const 4294967295)) (then unreachable))
 (local.set $available (i64.shl (i64.extend_i32_u (memory.size)) (i64.const 16)))
 (if (i64.gt_u (local.get $end) (local.get $available))
   (then
     (if (i32.eq (memory.grow (i32.wrap_i64
       (i64.shr_u (i64.add (i64.sub (local.get $end) (local.get $available)) (i64.const 65535)) (i64.const 16)))) (i32.const -1))
       (then unreachable))))
 (i32.store (local.get $p) (local.get $size))
 (i32.store offset=4 (local.get $p) (i32.const 0))
 (global.set $heap (i32.wrap_i64 (local.get $end)))
 (i32.add (local.get $p) (i32.const 8)))
(func $free (export "ilang_free") (param $ptr i32) (param $len i32)
 (local $p i32)
 (if (i32.eqz (local.get $ptr)) (then return))
 (local.set $p (i32.sub (local.get $ptr) (i32.const 8)))
 (i32.store offset=4 (local.get $p) (global.get $free_head))
 (global.set $free_head (local.get $p)))
"#;

/// A WAT expression that makes a String value with this text.
pub(crate) fn literal(text: &str) -> String {
    let bytes = text
        .as_bytes()
        .iter()
        .map(|byte| format!(" (i32.const {byte})"))
        .collect::<String>();
    format!(
        "(call $string_from_bytes (array.new_fixed $bytes {}{}))",
        text.len(),
        bytes
    )
}

/// Replace every `{{name}}` in `source` with the String literal of its text.
pub(crate) fn with_literals(mut source: String, texts: &[(&str, &str)]) -> String {
    for (name, text) in texts {
        source = source.replace(&format!("{{{{{name}}}}}"), &literal(text));
    }
    debug_assert!(!source.contains("{{"), "a runtime literal has no text");
    source
}

#[cfg(test)]
mod tests;
