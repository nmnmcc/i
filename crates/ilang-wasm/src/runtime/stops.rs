//! The shadow stack of source sites and the stop messages of spec §11.7.
//!
//! The protocol for the code generator is in the documentation of the
//! `runtime` module. A stop builds the whole message as a String value, copies
//! it into linear memory, calls the `panic` import once and traps.

pub fn wat() -> String {
    crate::runtime::with_literals(
        WAT.to_owned(),
        &[
            ("panic", "panic: "),
            ("assertion", "assertion failed: "),
            ("error", "error: "),
            ("boundary", "control: boundary is not active"),
            ("continuation", "control: continuation was already used"),
            ("float", "float: result is not finite"),
            ("bits_short", "bit array: value is too short"),
            ("bits_negative", "bit array: length is negative"),
            ("bits_not_whole", "bit array: bytes are not whole"),
            ("bits_float", "bit array: float is not finite"),
            ("host", "host: value is not correct"),
            ("colon", ":"),
            ("newline", "\n"),
        ],
    )
}

const WAT: &str = r#"
(global $shadow (mut (ref $frames)) (array.new_default $frames (i32.const 256)))
(global $sp (mut i32) (i32.const 0))
(global $site_table (mut (ref null $sites)) (ref.null $sites))
(global $site_modules (mut (ref null $values)) (ref.null $values))
(global $stopping (mut i32) (i32.const 0))

(func $sites_install (param $table (ref $sites)) (param $modules (ref $values))
  (global.set $site_table (local.get $table))
  (global.set $site_modules (local.get $modules)))

(func $shadow_grow
  (local $old (ref $frames)) (local $new (ref $frames)) (local $size i32)
  (local.set $old (global.get $shadow))
  (local.set $size (array.len (local.get $old)))
  ;; 2^28 frames is far beyond any Wasm call stack; trap as out of stack.
  (if (i32.ge_u (local.get $size) (i32.const 268435456)) (then unreachable))
  (local.set $new (array.new_default $frames (i32.shl (local.get $size) (i32.const 1))))
  (array.copy $frames $frames (local.get $new) (i32.const 0) (local.get $old) (i32.const 0) (local.get $size))
  (global.set $shadow (local.get $new)))

(func $shadow_enter (param $site i32)
  (local $sp i32)
  (local.set $sp (global.get $sp))
  (if (i32.ge_u (local.get $sp) (array.len (global.get $shadow))) (then (call $shadow_grow)))
  (array.set $frames (global.get $shadow) (local.get $sp) (local.get $site))
  (global.set $sp (i32.add (local.get $sp) (i32.const 1))))

(func $shadow_leave
  (global.set $sp (i32.sub (global.get $sp) (i32.const 1))))

(func $shadow_site (param $site i32)
  (if (i32.eqz (global.get $sp)) (then (return_call $shadow_enter (local.get $site))))
  (array.set $frames (global.get $shadow) (i32.sub (global.get $sp) (i32.const 1)) (local.get $site)))

(func $shadow_depth (result i32) (global.get $sp))

(func $shadow_restore (param $depth i32) (global.set $sp (local.get $depth)))

;; `module path:line:column`, or null when the site is not in the table.
(func $site_text (param $site i32) (result eqref)
  (local $table (ref null $sites)) (local $modules (ref null $values))
  (local $at i32) (local $module i32)
  (local.set $table (global.get $site_table))
  (local.set $modules (global.get $site_modules))
  (if (i32.or (ref.is_null (local.get $table)) (ref.is_null (local.get $modules)))
    (then (return (ref.null eq))))
  (if (i32.lt_s (local.get $site) (i32.const 0)) (then (return (ref.null eq))))
  (if (i32.ge_u (local.get $site) (i32.div_u (array.len (local.get $table)) (i32.const 3)))
    (then (return (ref.null eq))))
  (local.set $at (i32.mul (local.get $site) (i32.const 3)))
  (local.set $module (array.get $sites (local.get $table) (local.get $at)))
  (if (i32.ge_u (local.get $module) (array.len (local.get $modules)))
    (then (return (ref.null eq))))
  (return_call $site_line (local.get $site)))

;; Callers check the site first; see $site_text.
(func $site_line (param $site i32) (result eqref)
  (local $table (ref $sites)) (local $at i32)
  (local.set $table (ref.as_non_null (global.get $site_table)))
  (local.set $at (i32.mul (local.get $site) (i32.const 3)))
  (call $string_concat
    (call $list_cons
      (array.get $values (ref.as_non_null (global.get $site_modules))
        (array.get $sites (local.get $table) (local.get $at)))
    (call $list_cons {{colon}}
    (call $list_cons (call $int_to_string (call $int_from_i64 (i64.extend_i32_u
      (array.get $sites (local.get $table) (i32.add (local.get $at) (i32.const 1))))))
    (call $list_cons {{colon}}
    (call $list_cons (call $int_to_string (call $int_from_i64 (i64.extend_i32_u
      (array.get $sites (local.get $table) (i32.add (local.get $at) (i32.const 2))))))
    (call $list_nil))))))))

;; Append a line for the site to the reversed list of parts.
(func $stop_add_site (param $parts eqref) (param $site i32) (result eqref)
  (local $line eqref)
  (local.set $line (call $site_text (local.get $site)))
  (if (ref.is_null (local.get $line)) (then (return (local.get $parts))))
  (call $list_cons (local.get $line) (call $list_cons {{newline}} (local.get $parts))))

;; Copy the message into memory, give it to the host once, and trap.
(func $stop_send (param $message eqref)
  (local $length i32)
  (local.set $length (call $string_length_bytes (local.get $message)))
  (call $host_panic (call $string_to_mem (local.get $message)) (local.get $length))
  unreachable)

;; The reason, then every position on the shadow stack, innermost first.
(func $stop (param $reason eqref)
  (local $parts eqref) (local $i i32)
  (if (global.get $stopping) (then unreachable))
  (global.set $stopping (i32.const 1))
  (local.set $parts (call $list_cons (local.get $reason) (call $list_nil)))
  (local.set $i (global.get $sp))
  (if (i32.gt_u (local.get $i) (array.len (global.get $shadow)))
    (then (local.set $i (array.len (global.get $shadow)))))
  (block $done (loop $each
    (br_if $done (i32.eqz (local.get $i)))
    (local.set $i (i32.sub (local.get $i) (i32.const 1)))
    (local.set $parts (call $stop_add_site (local.get $parts)
      (array.get $frames (global.get $shadow) (local.get $i))))
    (br $each)))
  (call $stop_send (call $string_concat (call $list_reverse (local.get $parts))))
  unreachable)

;; The reason and one position.
(func $stop_at (param $reason eqref) (param $site i32)
  (if (global.get $stopping) (then unreachable))
  (global.set $stopping (i32.const 1))
  (call $stop_send (call $string_concat (call $list_reverse
    (call $stop_add_site (call $list_cons (local.get $reason) (call $list_nil)) (local.get $site)))))
  unreachable)

(func $panic (param $message eqref)
  (call $stop (call $string_append {{panic}} (local.get $message))) unreachable)
(func $stop_assertion (param $message eqref)
  (call $stop (call $string_append {{assertion}} (local.get $message))) unreachable)
(func $stop_error (param $error eqref) (param $site i32)
  (call $stop_at (call $string_append {{error}} (call $inspect (local.get $error))) (local.get $site))
  unreachable)
(func $stop_boundary_inactive (call $stop {{boundary}}) unreachable)
(func $stop_continuation_used (call $stop {{continuation}}) unreachable)
(func $stop_float (call $stop {{float}}) unreachable)
(func $stop_bits_short (call $stop {{bits_short}}) unreachable)
(func $stop_bits_negative (call $stop {{bits_negative}}) unreachable)
(func $stop_bits_not_whole (call $stop {{bits_not_whole}}) unreachable)
(func $stop_bits_float (call $stop {{bits_float}}) unreachable)
(func $stop_host_value (call $stop {{host}}) unreachable)
"#;
