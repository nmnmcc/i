//! The text form of values (spec §11.8), run with the whole runtime.

use crate::runtime::literal;
use crate::runtime::tests::{Checks, STUB_HOST, run_export};

fn integer(value: i64) -> String {
    format!("(call $int_from_i64 (i64.const {value}))")
}

fn fields(values: &[String]) -> String {
    format!(
        "(array.new_fixed $values {} {})",
        values.len(),
        values.join(" ")
    )
}

/// A value of the constructor `Pair`, the first one of the code generator.
fn pair(values: &[String]) -> String {
    format!(
        "(call $record (i32.const 16) (i64.const 40) {})",
        fields(values)
    )
}

fn tuple(values: &[String]) -> String {
    format!("(call $tuple {})", fields(values))
}

fn list(values: &[String]) -> String {
    values
        .iter()
        .rev()
        .fold("(call $list_nil)".to_owned(), |tail, head| {
            format!("(call $list_cons {head} {tail})")
        })
}

fn object(tag: i32, aux: i32, extra: i64, bytes: &str) -> String {
    format!(
        "(struct.new $obj (i32.const {tag}) (i32.const {aux}) (i64.const {extra}) \
         (f64.const 0) {bytes} (ref.null $values) (ref.null $limbs) (ref.null $call))"
    )
}

const INSTALL: &str = "(call $constructors_install (array.new_fixed $values 2 {{pair}} {{other}}))";

fn install() -> String {
    crate::runtime::with_literals(INSTALL.to_owned(), &[("pair", "Pair"), ("other", "Other")])
}

fn cases() -> Vec<(String, String)> {
    let no_bytes = "(ref.null $bytes)";
    let parameters = (0..300)
        .map(|i| {
            let letter = char::from(b'a' + (i % 26) as u8);
            if i < 26 {
                letter.to_string()
            } else {
                format!("{letter}{}", i / 26)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut cases =
        vec![
        (
            literal("🙂中\u{80}\u{7f}\n\r\t\u{c}\"\\"),
            "\"🙂中\\u{0080}\\u{007F}\\n\\r\\t\\f\\\"\\\\\"".into(),
        ),
        (
            literal("\u{0}\u{1}\u{8}\u{b}\u{1b}\u{1f} ~\u{9f}\u{a0}é\u{2028}"),
            "\"\\u{0000}\\u{0001}\\u{0008}\\u{000B}\\u{001B}\\u{001F} ~\\u{009F}\u{a0}é\u{2028}\""
                .into(),
        ),
        (literal(""), "\"\"".into()),
        (list(&[integer(11), integer(24)]), "[11, 24]".into()),
        (list(&[]), "[]".into()),
        (
            tuple(&["(call $bool_new (i32.const 1))".into(), integer(-42)]),
            "(True, -42)".into(),
        ),
        (tuple(&[]), "()".into()),
        (tuple(&[tuple(&[integer(1)])]), "((1))".into()),
        ("(ref.null eq)".into(), "Nil".into()),
        (
            "(struct.new $obj (i32.const 5) (i32.const 16) (i64.const 40) (f64.const 0) \
             (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call))"
                .into(),
            "Pair".into(),
        ),
        (pair(&[]), "Pair".into()),
        (
            pair(&["(call $bool_new (i32.const 0))".into(), literal("name")]),
            "Pair(False, \"name\")".into(),
        ),
        (
            format!("(call $record (i32.const 17) (i64.const 41) {})", fields(&[integer(1)])),
            "Other(1)".into(),
        ),
        (
            format!("(call $record (i32.const 18) (i64.const 41) {})", fields(&[])),
            "//wasm(unknown)".into(),
        ),
        (format!("(call $result_ok {})", integer(1)), "Ok(1)".into()),
        (
            "(call $result_error (ref.null eq))".into(),
            "Error(Nil)".into(),
        ),
        (
            format!("(call $record (i32.const 2) (i64.const 1) {})", fields(&[literal("m")])),
            "AssertionFailure(\"m\")".into(),
        ),
        (
            list(&[tuple(&[integer(1), literal("a")]), pair(&[list(&[])])]),
            "[(1, \"a\"), Pair([])]".into(),
        ),
        (
            object(9, 3, 10, "(array.new_fixed $bytes 2 (i32.const 170) (i32.const 85))"),
            "<<82, 2:u2>>".into(),
        ),
        (
            object(9, 0, 3, "(array.new_fixed $bytes 1 (i32.const 160))"),
            "<<5:u3>>".into(),
        ),
        (
            object(9, 0, 16, "(array.new_fixed $bytes 2 (i32.const 0) (i32.const 255))"),
            "<<0, 255>>".into(),
        ),
        (
            object(9, 0, 0, "(array.new_fixed $bytes 0)"),
            "<<>>".into(),
        ),
        (object(7, 0, 0, no_bytes), "//fn() { ... }".into()),
        (object(7, 0, 2, no_bytes), "//fn(a, b) { ... }".into()),
        (
            object(7, 0, 300, no_bytes),
            format!("//fn({parameters}) {{ ... }}"),
        ),
        (object(11, 0, 0, no_bytes), "//boundary".into()),
        (object(12, 0, 0, no_bytes), "//continuation".into()),
        (
            "(call $codepoint_new (i32.const 0x1f642))".into(),
            "//utfcodepoint(🙂)".into(),
        ),
        (
            "(call $codepoint_new (i32.const 10))".into(),
            "//utfcodepoint(\n)".into(),
        ),
        (
            format!("(call $int_shl {} {})", integer(1), integer(120)),
            (1_u128 << 120).to_string(),
        ),
        (integer(-1_073_741_825), "-1073741825".into()),
        ("(call $float_new (f64.const -0))".into(), "-0.0".into()),
        ("(call $float_new (f64.const 1e-7))".into(), "1.0e-7".into()),
        (
            "(struct.new $hostref (ref.null extern) (i64.const 1))".into(),
            "//wasm(opaque)".into(),
        ),
    ];
    cases.push((
        tuple(&[object(7, 0, 1, no_bytes), object(11, 0, 0, no_bytes)]),
        "(//fn(a) { ... }, //boundary)".into(),
    ));
    cases
}

#[test]
fn all_value_presentations_match() {
    let mut checks = Checks::default();
    checks.check(
        "install names",
        format!("(block (result i32) {} (i32.const 1))", install()),
    );
    for (value, expected) in cases() {
        checks.inspect(&value, &expected);
    }
    checks.run();
}

// Construct deeply nested aggregates in Wasm loops, then compare every output
// byte against repeated short prefixes/suffixes. The host never recursively
// constructs values or a 10,000-level WAT expression, so these tests isolate the
// runtime inspector's traversal stack rather than the parser or test generator.
fn deep_aggregate_fixtures() -> String {
    let mut source = String::from(
        r#"
(func $verify_deep_inspection
  (param $value eqref) (param $prefix_value eqref) (param $suffix_value eqref)
  (param $depth i32) (result i32)
  (local $out (ref $bytes)) (local $prefix (ref $bytes)) (local $suffix (ref $bytes))
  (local $prefix_bytes i32) (local $suffix_bytes i32) (local $prefix_end i32)
  (local $suffix_start i32) (local $end i32) (local $i i32)
  (local.set $out (call $string_bytes (call $inspect (local.get $value))))
  (local.set $prefix (call $string_bytes (local.get $prefix_value)))
  (local.set $suffix (call $string_bytes (local.get $suffix_value)))
  (local.set $prefix_bytes (array.len (local.get $prefix)))
  (local.set $suffix_bytes (array.len (local.get $suffix)))
  (local.set $prefix_end (i32.mul (local.get $depth) (local.get $prefix_bytes)))
  (local.set $suffix_start (i32.add (local.get $prefix_end) (i32.const 1)))
  (local.set $end
    (i32.add (local.get $suffix_start) (i32.mul (local.get $depth) (local.get $suffix_bytes))))
  (if (i32.ne (array.len (local.get $out)) (local.get $end))
    (then (return (i32.const 1))))
  (block $prefix_done (loop $check_prefix
    (br_if $prefix_done (i32.ge_u (local.get $i) (local.get $prefix_end)))
    (if (i32.ne (array.get_u $bytes (local.get $out) (local.get $i))
          (array.get_u $bytes (local.get $prefix)
            (i32.rem_u (local.get $i) (local.get $prefix_bytes))))
      (then (return (i32.const 2))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $check_prefix)))
  (if (i32.ne (array.get_u $bytes (local.get $out) (local.get $prefix_end)) (i32.const 49))
    (then (return (i32.const 3))))
  (local.set $i (local.get $suffix_start))
  (block $suffix_done (loop $check_suffix
    (br_if $suffix_done (i32.ge_u (local.get $i) (local.get $end)))
    (if (i32.ne (array.get_u $bytes (local.get $out) (local.get $i))
          (array.get_u $bytes (local.get $suffix)
            (i32.rem_u (i32.sub (local.get $i) (local.get $suffix_start)) (local.get $suffix_bytes))))
      (then (return (i32.const 4))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $check_suffix)))
  (i32.const 0))
(func (export "long_list") (result i32)
  (local $i i32) (local $list eqref) (local $out (ref $bytes))
  (local.set $list (call $list_nil))
  (loop $build
    (local.set $list (call $list_cons (ref.i31 (i32.const 1)) (local.get $list)))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br_if $build (i32.lt_u (local.get $i) (i32.const 10000))))
  (local.set $out (call $string_bytes (call $inspect (local.get $list))))
  (if (i32.ne (array.len (local.get $out)) (i32.const 30000)) (then (return (i32.const 1))))
  (if (i32.ne (array.get_u $bytes (local.get $out) (i32.const 0)) (i32.const 91)) (then (return (i32.const 2))))
  (if (i32.ne (array.get_u $bytes (local.get $out) (i32.const 29999)) (i32.const 93)) (then (return (i32.const 3))))
  (local.set $i (i32.const 1))
  (loop $check
    (if (i32.ne (array.get_u $bytes (local.get $out) (local.get $i)) (i32.const 49)) (then (return (i32.const 4))))
    (local.set $i (i32.add (local.get $i) (i32.const 3)))
    (br_if $check (i32.lt_u (local.get $i) (i32.const 30000))))
  (i32.const 0))
"#,
    );
    let nested_value = "(local.get $value)".to_owned();
    let first_field = pair(&[nested_value.clone(), integer(1)]);
    let middle_field = pair(&[integer(1), nested_value.clone(), integer(1)]);
    let tuple_field = tuple(&[nested_value.clone(), integer(1)]);
    let list_head = format!("(call $list_cons {nested_value} (call $list_nil))");
    for (name, value, prefix, suffix) in [
        ("deep_first_field", first_field, "Pair(", ", 1)"),
        ("deep_middle_field", middle_field, "Pair(1, ", ", 1)"),
        ("deep_tuple_field", tuple_field, "(", ", 1)"),
        ("deep_list_heads", list_head, "[", "]"),
    ] {
        source.push_str(&format!(
            "(func (export \"{name}\") (result i32)\n\
               (local $i i32) (local $value eqref)\n\
               {}\n\
               (local.set $value (ref.i31 (i32.const 1)))\n\
               (loop $build\n\
                 (local.set $value {value})\n\
                 (local.set $i (i32.add (local.get $i) (i32.const 1)))\n\
                 (br_if $build (i32.lt_u (local.get $i) (i32.const 10000))))\n\
               (call $verify_deep_inspection (local.get $value) {} {} (i32.const 10000)))\n",
            install(),
            literal(prefix),
            literal(suffix)
        ));
    }
    source
}

fn execute(export: &str) {
    let source = format!(
        "(module {} {STUB_HOST} {} {})",
        crate::runtime::TYPES,
        crate::runtime::wat(),
        deep_aggregate_fixtures()
    );
    assert_eq!(
        run_export(&source, export),
        0,
        "{export}: inspection case failed"
    );
}

#[test]
fn long_lists_are_inspected_without_recursive_traversal() {
    execute("long_list");
}

#[test]
fn deep_first_record_fields_do_not_use_the_call_stack() {
    execute("deep_first_field");
}

#[test]
fn deep_middle_record_fields_preserve_pending_siblings() {
    execute("deep_middle_field");
}

#[test]
fn deep_tuple_fields_do_not_use_the_call_stack() {
    execute("deep_tuple_field");
}

#[test]
fn deep_list_heads_do_not_use_the_call_stack() {
    execute("deep_list_heads");
}
