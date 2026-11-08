use super::literal;

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

fn record(ctor: i32, type_id: i64, values: &[String]) -> String {
    format!(
        "(call $record (i32.const {ctor}) (i64.const {type_id}) {})",
        fields(values)
    )
}

fn list(values: &[String]) -> String {
    values
        .iter()
        .rev()
        .fold("(call $list_nil)".to_owned(), |tail, head| {
            format!("(call $list_cons {head} {tail})")
        })
}

fn object(tag: i32, aux: i32, extra: i64, fields: &str, bytes: &str) -> String {
    format!(
        "(struct.new $obj (i32.const {tag}) (i32.const {aux}) (i64.const {extra}) \
         (f64.const 0) {bytes} {fields} (ref.null $limbs) (ref.null $call))"
    )
}

fn cases() -> Vec<(String, String)> {
    let empty = "(ref.null $values)";
    let no_bytes = "(ref.null $bytes)";
    let leaf = record(33, 200, &[integer(123), literal("a"), integer(1)]);
    let collision = record(
        34,
        200,
        &[
            integer(456),
            list(&[
                format!("(call $tuple {})", fields(&[literal("b"), integer(2)])),
                format!("(call $tuple {})", fields(&[literal("c"), integer(3)])),
            ]),
        ],
    );
    let branch = record(35, 200, &[integer(3), list(&[leaf.clone(), collision])]);
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
    vec![
        (
            literal("🙂中\u{80}\u{7f}\n\r\t\u{c}\"\\"),
            "\"🙂中\\u{0080}\\u{007F}\\n\\r\\t\\f\\\"\\\\\"".into(),
        ),
        (list(&[integer(11), integer(24)]), "[11, 24]".into()),
        (list(&[]), "[]".into()),
        (
            format!(
                "(call $tuple {})",
                fields(&["(call $bool_new (i32.const 1))".into(), integer(-42)])
            ),
            "#(True, -42)".into(),
        ),
        ("(ref.null eq)".into(), "Nil".into()),
        (record(32, 1, &[]), "Pair".into()),
        (
            record(
                32,
                1,
                &["(call $bool_new (i32.const 0))".into(), literal("name")],
            ),
            "Pair(False, \"name\")".into(),
        ),
        (
            object(
                9,
                3,
                10,
                empty,
                "(array.new_fixed $bytes 2 (i32.const 170) (i32.const 85))",
            ),
            "<<82, 2:size(2)>>".into(),
        ),
        (
            object(9, 0, 0, empty, "(array.new_fixed $bytes 0)"),
            "<<>>".into(),
        ),
        (object(7, 0, 0, empty, no_bytes), "//fn() { ... }".into()),
        (
            object(7, 0, 2, empty, no_bytes),
            "//fn(a, b) { ... }".into(),
        ),
        (
            object(7, 0, 300, empty, no_bytes),
            format!("//fn({parameters}) {{ ... }}"),
        ),
        (
            record(32, 100, &[integer(0), record(36, 200, &[])]),
            "dict.from_list([])".into(),
        ),
        (
            record(32, 100, &[integer(1), leaf]),
            "dict.from_list([#(\"a\", 1)])".into(),
        ),
        (
            record(32, 100, &[integer(3), branch]),
            "dict.from_list([#(\"a\", 1), #(\"b\", 2), #(\"c\", 3)])".into(),
        ),
        (
            object(13, 0x1f642, 0, empty, no_bytes),
            "//utfcodepoint(🙂)".into(),
        ),
        (
            format!("(call $int_shl {} {})", integer(1), integer(120)),
            (1_u128 << 120).to_string(),
        ),
        ("(call $float_new (f64.const -0))".into(), "-0.0".into()),
        (
            object(11, 0, 0, &fields(&[integer(55)]), no_bytes),
            "55".into(),
        ),
        (
            "(struct.new $hostbox (ref.null extern) (i64.const 1))".into(),
            "//wasm(opaque)".into(),
        ),
    ]
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
"#,
    );
    let nested_value = "(local.get $value)".to_owned();
    let first_field = record(32, 1, &[nested_value.clone(), integer(1)]);
    let middle_field = record(32, 1, &[integer(1), nested_value.clone(), integer(1)]);
    let list_head = format!("(call $list_cons {nested_value} (call $list_nil))");
    let leaf = record(
        33,
        200,
        &[integer(123), "(local.get $key)".to_owned(), nested_value],
    );
    let dict_value = record(32, 100, &[integer(1), leaf]);
    for (name, value, prefix, suffix, key) in [
        ("deep_first_field", first_field, "Pair(", ", 1)", None),
        ("deep_middle_field", middle_field, "Pair(1, ", ", 1)", None),
        ("deep_list_heads", list_head, "[", "]", None),
        (
            "deep_dict_values",
            dict_value,
            "dict.from_list([#(\"a\", ",
            ")])",
            Some(literal("a")),
        ),
    ] {
        source.push_str(&format!(
            "(func (export \"{name}\") (result i32)\n\
               (local $i i32) (local $value eqref) (local $key eqref)\n\
               (local.set $value (ref.i31 (i32.const 1)))\n"
        ));
        if let Some(key) = key {
            source.push_str(&format!("(local.set $key {key})\n"));
        }
        source.push_str(&format!(
            "(loop $build\n\
               (local.set $value {value})\n\
               (local.set $i (i32.add (local.get $i) (i32.const 1)))\n\
               (br_if $build (i32.lt_u (local.get $i) (i32.const 10000))))\n\
             (call $verify_deep_inspection (local.get $value) {} {} (i32.const 10000)))\n",
            literal(prefix),
            literal(suffix)
        ));
    }
    source
}

fn fixture() -> Vec<u8> {
    // Use production allocation, bigint, string, and value bodies together so
    // tests catch interface and representation changes between runtime modules.
    let mut source = format!(
        "(module\n{}\n{}\n",
        crate::runtime::TYPES,
        crate::runtime::MEMORY
    );
    source.push_str(
        r#"
(type $hostbox (struct (field externref) (field i64)))
(func $host_id (param $v eqref) (result i64)
  (struct.get $hostbox 1 (ref.cast (ref $hostbox) (local.get $v))))
(func $host_equal (param $a eqref) (param $b eqref) (result i32)
  (i64.eq (call $host_id (local.get $a)) (call $host_id (local.get $b))))
(global $written_ptr (mut i32) (i32.const 0))
(global $written_len (mut i32) (i32.const 0))
(global $written_channel (mut i32) (i32.const 0))
(func $host_write (param i32 i32 i32)
  (global.set $written_channel (local.get 0))
  (global.set $written_ptr (local.get 1))
  (global.set $written_len (local.get 2)))
(func $type_is_dict (param i64) (result i32) (i64.eq (local.get 0) (i64.const 100)))
"#,
    );
    source.push_str(&format!(
        "(func $constructor_name (param i32) (result eqref) {})\n",
        literal("Pair")
    ));
    source.push_str(&crate::runtime::bigint::wat());
    source.push_str(crate::runtime::values::CORE);
    source.push_str(&crate::runtime::values::strings::wat());
    source.push_str(&super::wat());
    source.push_str("(func (export \"verify\") (result i32)\n");
    for (index, (value, expected)) in cases().iter().enumerate() {
        source.push_str(&format!(
            "(if (call $string_compare (call $inspect {value}) {}) \
              (then (return (i32.const {}))))\n",
            literal(expected),
            index + 1
        ));
    }
    source.push_str(r#"
(drop (call $echo (ref.i31 (i32.const 42))))
(if (i32.ne (global.get $written_channel) (i32.const 2)) (then (return (i32.const 100))))
(if (i32.ne (global.get $written_len) (i32.const 3)) (then (return (i32.const 101))))
(if (i32.ne (i32.load8_u (global.get $written_ptr)) (i32.const 52)) (then (return (i32.const 102))))
(if (i32.ne (i32.load8_u offset=1 (global.get $written_ptr)) (i32.const 50)) (then (return (i32.const 103))))
(if (i32.ne (i32.load8_u offset=2 (global.get $written_ptr)) (i32.const 10)) (then (return (i32.const 104))))
(i32.const 0))
(func (export "long_list") (result i32)
  (local $i i32) (local $list eqref) (local $out (ref $bytes))
  (local.set $list (call $list_nil))
  (loop $build
    (local.set $list (call $list_cons (ref.i31 (i32.const 1)) (local.get $list)))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br_if $build (i32.lt_u (local.get $i) (i32.const 10000))))
  (local.set $out (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (call $inspect (local.get $list))))))
  (if (i32.ne (array.len (local.get $out)) (i32.const 30000)) (then (return (i32.const 1))))
  (if (i32.ne (array.get_u $bytes (local.get $out) (i32.const 0)) (i32.const 91)) (then (return (i32.const 2))))
  (if (i32.ne (array.get_u $bytes (local.get $out) (i32.const 29999)) (i32.const 93)) (then (return (i32.const 3))))
  (local.set $i (i32.const 1))
  (loop $check
    (if (i32.ne (array.get_u $bytes (local.get $out) (local.get $i)) (i32.const 49)) (then (return (i32.const 4))))
    (local.set $i (i32.add (local.get $i) (i32.const 3)))
    (br_if $check (i32.lt_u (local.get $i) (i32.const 30000))))
  (i32.const 0))
"#);
    source.push_str(&deep_aggregate_fixtures());
    source.push(')');
    wat::parse_str(source).expect("inspection fixture must assemble")
}

#[test]
fn inspection_runtime_is_valid_wasm_gc() {
    wasmparser::Validator::new()
        .validate_all(&fixture())
        .expect("inspection fixture must validate");
}

#[test]
fn all_value_presentations_match() {
    execute("verify");
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
fn deep_list_heads_do_not_use_the_call_stack() {
    execute("deep_list_heads");
}

#[test]
fn deep_dictionary_values_preserve_pending_dict_delimiters() {
    execute("deep_dict_values");
}

fn execute(export: &str) {
    let file = tempfile::NamedTempFile::new().expect("create wasm fixture");
    std::fs::write(file.path(), fixture()).expect("write wasm fixture");
    let wasmtime = std::env::var_os("ILANG_WASMTIME").unwrap_or_else(|| "wasmtime".into());
    let output = std::process::Command::new(wasmtime)
        .args(["run", "--invoke", export])
        .arg(file.path())
        .output()
        .expect("Wasmtime is required; run this test in the Devenv shell");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "0",
        "{export}: inspection case failed"
    );
}
