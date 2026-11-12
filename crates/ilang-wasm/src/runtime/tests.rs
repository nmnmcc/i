//! Tests of the whole runtime, run in the pinned Wasmtime.
//!
//! A test module is [`TYPES`], [`STUB_HOST`] in place of [`HOST`], the
//! runtime and the checks. The stub `panic` throws the message as a Wasm
//! exception, so a check can catch a stop and compare its message.

use super::{HOST, TYPES, literal, wat};
use std::fmt::Write as _;

/// Defines the two runtime imports as functions with the same names.
pub(crate) const STUB_HOST: &str = r#"
(tag $test_stop (param i32 i32))
(func $host_panic (param $address i32) (param $length i32)
  (throw $test_stop (local.get $address) (local.get $length)))
(func $host_reference_id (param $r externref) (result i64)
  (if (result i64) (ref.is_null (local.get $r)) (then (i64.const 0)) (else (i64.const 1))))
;; The text of a Float is i code (`lib/i/float_text.i`), which these
;; modules do not have. Here a Float is written as its 64 bits, as a signed
;; decimal number, and `Checks::inspect` expects that form ([`float_bits`]).
(func $float_to_string (param $value eqref) (result eqref)
  (call $int_to_string (call $int_from_i64
    (i64.reinterpret_f64 (call $float_get (local.get $value))))))
"#;

/// The text form of [`STUB_HOST`]: each Float literal of `text`, such as
/// `-1.5` or `1.0e-7`, written as the signed decimal of its 64 bits.
pub(crate) fn float_bits(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let mut j = i;
        if bytes[j] == b'-' {
            j += 1;
        }
        let digits = |mut k: usize| {
            while k < bytes.len() && bytes[k].is_ascii_digit() {
                k += 1;
            }
            k
        };
        let whole = digits(j);
        let previous = start.checked_sub(1).map(|p| bytes[p]);
        let begins = !previous.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'.');
        if begins && whole > j && whole < bytes.len() && bytes[whole] == b'.' {
            let fraction = digits(whole + 1);
            if fraction > whole + 1 {
                let mut end = fraction;
                if end < bytes.len() && bytes[end] == b'e' {
                    let sign = usize::from(bytes.get(end + 1) == Some(&b'-'));
                    let exponent = digits(end + 1 + sign);
                    if exponent > end + 1 + sign {
                        end = exponent;
                    }
                }
                let value: f64 = text[start..end].parse().expect("a Float literal");
                out.extend((value.to_bits() as i64).to_string().bytes());
                i = end;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).expect("UTF-8 is kept")
}

/// A list of checks, each an `i32` expression that must not be 0.
#[derive(Default)]
pub(crate) struct Checks {
    functions: String,
    body: String,
    names: Vec<String>,
}

impl Checks {
    pub(crate) fn check(&mut self, name: impl Into<String>, condition: impl AsRef<str>) {
        self.names.push(name.into());
        writeln!(
            self.body,
            "(if (i32.eqz {}) (then (return (i32.const {}))))",
            condition.as_ref(),
            self.names.len()
        )
        .unwrap();
    }

    /// The value of `expression` is the String `expected`.
    pub(crate) fn string(&mut self, name: impl Into<String>, expression: &str, expected: &str) {
        self.check(
            name,
            format!("(call $equal {expression} {})", literal(expected)),
        );
    }

    /// The text form of `expression` is `expected`.
    pub(crate) fn inspect(&mut self, expression: &str, expected: &str) {
        self.string(
            format!("inspect {expected}"),
            &format!("(call $inspect {expression})"),
            &float_bits(expected),
        );
    }

    /// `body`, which leaves nothing on the stack, stops with this message.
    pub(crate) fn stops(&mut self, name: impl Into<String>, body: &str, expected: &str) {
        let index = self.names.len();
        writeln!(
            self.functions,
            "(func $stop_case_{index} (result i32)
               (local $address i32) (local $length i32) (local $i i32)
               (global.set $stopping (i32.const 0))
               (global.set $sp (i32.const 0))
               (block $caught (result i32 i32)
                 (try_table (catch $test_stop $caught) {body})
                 (return (i32.const 0)))
               (local.set $length) (local.set $address)
               (global.set $stopping (i32.const 0))
               (global.set $sp (i32.const 0))
               (call $equal (call $string_from_mem (local.get $address) (local.get $length)) {}))",
            literal(expected)
        )
        .unwrap();
        self.check(name, format!("(call $stop_case_{index})"));
    }

    /// Extra definitions for the module, such as helper functions.
    pub(crate) fn define(&mut self, definitions: &str) {
        self.functions.push_str(definitions);
    }

    pub(crate) fn module(&self) -> String {
        format!(
            "(module {TYPES} {STUB_HOST} {} {}\n(func (export \"verify\") (result i32)\n{}(i32.const 0)))",
            wat(),
            self.functions,
            self.body
        )
    }

    pub(crate) fn run(self) {
        let failed = run_export(&self.module(), "verify");
        assert_eq!(
            failed,
            0,
            "check failed: {}",
            self.names
                .get(failed.wrapping_sub(1))
                .map(String::as_str)
                .unwrap_or("unknown")
        );
    }
}

/// Assemble and validate a module, run one export and give its i32 result.
pub(crate) fn run_export(source: &str, export: &str) -> usize {
    let binary = wat::parse_str(source).expect("test module must assemble");
    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
        .validate_all(&binary)
        .expect("test module must validate");
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("runtime.wasm");
    std::fs::write(&path, binary).expect("write test module");
    let wasmtime = std::env::var_os("WASMTIME").unwrap_or_else(|| "wasmtime".into());
    let output = std::process::Command::new(wasmtime)
        .args([
            "run",
            "-W",
            "gc=y,exceptions=y,tail-call=y",
            "--invoke",
            export,
        ])
        .arg(&path)
        .output()
        .expect("Wasmtime is required; run the tests in the Devenv shell");
    assert!(
        output.status.success(),
        "{export} trapped: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("the export gives a number")
}

fn int(value: i64) -> String {
    format!("(call $int_from_i64 (i64.const {value}))")
}

/// An Int from its decimal text, for values beyond i64.
fn big(text: &str) -> String {
    format!("(call $parse_decimal {})", literal(text))
}

fn float(value: f64) -> String {
    format!(
        "(call $float_new (f64.reinterpret_i64 (i64.const {})))",
        value.to_bits() as i64
    )
}

fn string(text: &str) -> String {
    literal(text)
}

fn list(items: &[String]) -> String {
    items
        .iter()
        .rev()
        .fold("(call $list_nil)".to_owned(), |tail, head| {
            format!("(call $list_cons {head} {tail})")
        })
}

fn bits(bytes: &[u8], offset: u32, length: u64) -> String {
    let mut data = format!("(array.new_fixed $bytes {}", bytes.len());
    for byte in bytes {
        write!(data, " (i32.const {byte})").unwrap();
    }
    data.push(')');
    format!("(call $bits_new {data} (i32.const {offset}) (i64.const {length}))")
}

const HELPERS: &str = r#"
(func $parse_decimal (param $text eqref) (result eqref)
  (local $value eqref) (local $ok i32)
  (call $int_parse (local.get $text) (ref.i31 (i32.const 10)))
  (local.set $ok) (local.set $value)
  (if (i32.eqz (local.get $ok)) (then unreachable))
  (local.get $value))
(func $function (param $arity i64) (result eqref)
  (struct.new $obj (i32.const 7) (i32.const 0) (local.get $arity) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $tagged (param $tag i32) (result eqref)
  (struct.new $obj (local.get $tag) (i32.const 0) (i64.const 0) (f64.const 0)
    (ref.null $bytes) (ref.null $values) (ref.null $limbs) (ref.null $call)))
(func $same_hash (param $a eqref) (param $b eqref) (result i32)
  (call $equal (call $core_hash (local.get $a)) (call $core_hash (local.get $b))))
(func $both_equal (param $a eqref) (param $b eqref) (result i32)
  (i32.and (i32.and (call $equal (local.get $a) (local.get $b)) (call $equal (local.get $b) (local.get $a)))
    (call $same_hash (local.get $a) (local.get $b))))
(func $different (param $a eqref) (param $b eqref) (result i32)
  (i32.and (i32.eqz (call $equal (local.get $a) (local.get $b)))
    (i32.eqz (call $equal (local.get $b) (local.get $a)))))
(func $install_sites
  (call $sites_install
    (array.new_fixed $sites 12
      (i32.const 0) (i32.const 12) (i32.const 5)
      (i32.const 1) (i32.const 3) (i32.const 1)
      (i32.const 0) (i32.const 40) (i32.const 17)
      (i32.const 7) (i32.const 1) (i32.const 1))
    (array.new_fixed $values 2 {{app_users}} {{json_decode}})))
"#;

fn checks() -> Checks {
    let mut checks = Checks::default();
    checks.define(&super::with_literals(
        HELPERS.to_owned(),
        &[
            ("app_users", "app/users"),
            ("json_decode", "github.com/alice/json@1 json/decode"),
        ],
    ));
    checks
}

#[test]
fn runtime_with_host_imports_is_valid_wasm() {
    let source = format!(
        "(module {TYPES} {HOST} {} {})",
        wat(),
        super::LIBRARY_ABSENT
    );
    let binary = wat::parse_str(source).expect("the runtime must assemble");
    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::all())
        .validate_all(&binary)
        .expect("the runtime must validate");
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(&binary) {
        match payload.unwrap() {
            wasmparser::Payload::ImportSection(section) => {
                for import in section.into_imports() {
                    let import = import.unwrap();
                    imports.push(format!("{}.{}", import.module, import.name));
                }
            }
            wasmparser::Payload::ExportSection(section) => {
                for export in section {
                    exports.push(export.unwrap().name.to_owned());
                }
            }
            wasmparser::Payload::StartSection { .. } => panic!("no Wasm start function"),
            _ => {}
        }
    }
    assert_eq!(
        imports,
        ["ilang:host/v1.panic", "ilang:host/v1.reference_id"]
    );
    assert_eq!(exports, ["memory", "ilang_alloc", "ilang_free"]);
}

#[test]
fn stop_messages_list_the_reason_and_the_shadow_stack() {
    let mut checks = checks();
    checks.stops(
        "no site table",
        &format!("(call $panic {})", string("boom")),
        "panic: boom",
    );
    checks.stops(
        "innermost position first",
        &format!(
            "(call $install_sites)
             (call $shadow_enter (i32.const 0))
             (call $shadow_enter (i32.const 1))
             (call $shadow_site (i32.const 2))
             (call $panic {})",
            string("boom")
        ),
        "panic: boom\napp/users:40:17\napp/users:12:5",
    );
    checks.stops(
        "module path with a location",
        &format!(
            "(call $install_sites)
             (call $shadow_enter (i32.const 0))
             (call $shadow_site (i32.const 1))
             (call $stop_assertion {})",
            string("assert")
        ),
        "assertion failed: assert\ngithub.com/alice/json@1 json/decode:3:1",
    );
    checks.stops(
        "a tail call replaces the frame",
        &format!(
            "(call $install_sites)
             (call $shadow_enter (i32.const 0))
             (call $shadow_enter (i32.const 1))
             (call $shadow_leave)
             (call $shadow_enter (i32.const 2))
             (call $panic {})",
            string("tail")
        ),
        "panic: tail\napp/users:40:17\napp/users:12:5",
    );
    checks.stops(
        "returned frames are gone",
        &format!(
            "(call $install_sites)
             (call $shadow_enter (i32.const 0))
             (call $shadow_enter (i32.const 1))
             (call $shadow_leave)
             (call $panic {})",
            string("x")
        ),
        "panic: x\napp/users:12:5",
    );
    checks.stops(
        "unknown sites are not printed",
        &format!(
            "(call $install_sites)
             (call $shadow_enter (i32.const 0))
             (call $shadow_enter (i32.const -1))
             (call $shadow_enter (i32.const 4))
             (call $shadow_enter (i32.const 3))
             (call $panic {})",
            string("")
        ),
        "panic: \napp/users:12:5",
    );
    checks.stops(
        "site without a frame",
        "(call $install_sites) (call $shadow_site (i32.const 2)) (call $stop_float)",
        "float: result is not finite\napp/users:40:17",
    );
    checks.stops(
        "the shadow stack grows",
        "(call $install_sites)
         (call $shadow_enter (i32.const 2))
         (loop $more
           (call $shadow_enter (i32.const -1))
           (local.set $i (i32.add (local.get $i) (i32.const 1)))
           (br_if $more (i32.lt_u (local.get $i) (i32.const 5000))))
         (call $shadow_site (i32.const 0))
         (call $stop_bits_short)",
        "bit array: value is too short\napp/users:12:5\napp/users:40:17",
    );
    checks.check(
        "depth is kept and restored",
        "(block (result i32)
           (global.set $sp (i32.const 0))
           (call $shadow_enter (i32.const 0))
           (call $shadow_enter (i32.const 1))
           (call $shadow_restore (i32.const 1))
           (i32.eq (call $shadow_depth) (i32.const 1)))",
    );
    checks.stops(
        "an error at an entry point has only the entry point's position",
        &format!(
            "(call $install_sites)
             (call $shadow_enter (i32.const 0))
             (call $shadow_enter (i32.const 1))
             (call $stop_error (call $result_ok {}) (i32.const 2))",
            string("x\n")
        ),
        "error: Ok(\"x\\n\")\napp/users:40:17",
    );
    for (function, reason) in [
        ("$stop_boundary_inactive", "control: boundary is not active"),
        (
            "$stop_continuation_used",
            "control: continuation was already used",
        ),
        ("$stop_float", "float: result is not finite"),
        ("$stop_bits_short", "bit array: value is too short"),
        ("$stop_bits_negative", "bit array: length is negative"),
        ("$stop_bits_not_whole", "bit array: bytes are not whole"),
        ("$stop_bits_float", "bit array: float is not finite"),
        ("$stop_host_value", "host: value is not correct"),
    ] {
        checks.stops(
            format!("reason of {function}"),
            &format!("(call $install_sites) (call $shadow_enter (i32.const 0)) (call {function})"),
            &format!("{reason}\napp/users:12:5"),
        );
    }
    checks.stops(
        "any reason with $stop_at",
        &format!(
            "(call $install_sites) (call $stop_at {} (i32.const 1))",
            string("why")
        ),
        "why\ngithub.com/alice/json@1 json/decode:3:1",
    );
    checks.run();
}

#[test]
fn equality_is_structural_with_identity_for_functions_and_references() {
    let mut checks = checks();
    let pairs_equal = [
        (
            "nil",
            "(ref.null eq)".to_owned(),
            "(ref.null eq)".to_owned(),
        ),
        ("zeros", float(0.0), float(-0.0)),
        ("floats", float(1.5), float(1.5)),
        (
            "big ints",
            big("123456789012345678901234567890"),
            format!(
                "(call $int_add {} {})",
                big("123456789012345678901234567889"),
                int(1)
            ),
        ),
        ("strings", string("中文🙂"), string("中文🙂")),
        (
            "bit arrays with different offsets",
            bits(&[0b1010_1100, 0b1100_0000], 2, 5),
            bits(&[0b1011_0000], 0, 5),
        ),
        (
            "tuples",
            format!(
                "(call $tuple (array.new_fixed $values 2 {} {}))",
                int(1),
                string("a")
            ),
            format!(
                "(call $tuple (array.new_fixed $values 2 {} {}))",
                int(1),
                string("a")
            ),
        ),
        (
            "lists",
            list(&[int(1), list(&[int(2)])]),
            list(&[int(1), list(&[int(2)])]),
        ),
        (
            "custom values",
            format!("(call $result_ok {})", float(0.0)),
            format!("(call $result_ok {})", float(-0.0)),
        ),
        (
            "codepoints",
            "(call $codepoint_new (i32.const 128512))".to_owned(),
            "(call $codepoint_new (i32.const 128512))".to_owned(),
        ),
        (
            "host references with one reference_id",
            "(struct.new $hostref (ref.null extern) (i32.const 41) (i64.const 7))".to_owned(),
            "(struct.new $hostref (ref.null extern) (i32.const 41) (i64.const 7))".to_owned(),
        ),
        (
            "bools",
            "(call $bool_new (i32.const 1))".to_owned(),
            "(call $core_not (call $bool_new (i32.const 0)))".to_owned(),
        ),
    ];
    for (name, a, b) in pairs_equal {
        checks.check(name, format!("(call $both_equal {a} {b})"));
    }
    let pairs_different = [
        (
            "nil and False",
            "(ref.null eq)".to_owned(),
            "(call $bool_new (i32.const 0))".to_owned(),
        ),
        ("int and float", int(1), float(1.0)),
        ("ints", int(1), int(-1)),
        ("strings", string("a"), string("ab")),
        (
            "bit arrays of other lengths",
            bits(&[0], 0, 7),
            bits(&[0], 0, 8),
        ),
        ("bit arrays", bits(&[0x80], 0, 1), bits(&[0], 0, 1)),
        (
            "tuples of other sizes",
            format!("(call $tuple (array.new_fixed $values 1 {}))", int(1)),
            format!(
                "(call $tuple (array.new_fixed $values 2 {} {}))",
                int(1),
                int(1)
            ),
        ),
        (
            "Ok and Error",
            format!("(call $result_ok {})", int(1)),
            format!("(call $result_error {})", int(1)),
        ),
        ("lists", list(&[int(1)]), list(&[int(1), int(1)])),
        (
            "codepoint and int",
            "(call $codepoint_new (i32.const 65))".to_owned(),
            int(65),
        ),
        (
            "host references",
            "(struct.new $hostref (ref.null extern) (i32.const 41) (i64.const 7))".to_owned(),
            "(struct.new $hostref (ref.null extern) (i32.const 41) (i64.const 8))".to_owned(),
        ),
        (
            "functions from two evaluations",
            "(call $function (i64.const 2))".to_owned(),
            "(call $function (i64.const 2))".to_owned(),
        ),
        (
            "boundaries",
            "(call $tagged (i32.const 11))".to_owned(),
            "(call $tagged (i32.const 11))".to_owned(),
        ),
        (
            "continuations",
            "(call $tagged (i32.const 12))".to_owned(),
            "(call $tagged (i32.const 12))".to_owned(),
        ),
        (
            "lists holding different functions",
            list(&["(call $function (i64.const 0))".to_owned()]),
            list(&["(call $function (i64.const 0))".to_owned()]),
        ),
    ];
    for (name, a, b) in pairs_different {
        checks.check(
            format!("different {name}"),
            format!("(call $different {a} {b})"),
        );
    }
    checks.define(
        "(func $same_function (result i32) (local $f eqref) (local $b eqref) (local $k eqref)
           (local.set $f (call $function (i64.const 1)))
           (local.set $b (call $tagged (i32.const 11)))
           (local.set $k (call $tagged (i32.const 12)))
           (i32.and (i32.and (call $both_equal (local.get $f) (local.get $f))
             (call $both_equal (local.get $b) (local.get $b)))
             (i32.and (call $both_equal (local.get $k) (local.get $k))
               (call $both_equal (call $tuple (array.new_fixed $values 1 (local.get $f)))
                 (call $tuple (array.new_fixed $values 1 (local.get $f)))))))",
    );
    checks.check("a function is itself", "(call $same_function)");
    checks.check(
        "a host reference is made with its reference_id",
        "(call $both_equal (call $host_ref_new (ref.null extern) (i32.const 41)) (struct.new $hostref (ref.null extern) (i32.const 41) (i64.const 0)))",
    );
    for value in [
        int(-1),
        big("-340282366920938463463374607431768211456"),
        string("hash"),
        float(-0.0),
        "(call $function (i64.const 3))".to_owned(),
    ] {
        checks.check(
            format!("hash of {value} is from 0 to 2^32 - 1"),
            format!(
                "(block (result i32) (local.set $h (call $core_hash {value}))
                   (i32.and (i32.ge_s (call $int_sign (local.get $h)) (i32.const 0))
                     (i32.lt_s (call $int_compare (local.get $h) (call $int_from_i64 (i64.const 4294967296))) (i32.const 0))))"
            ),
        );
    }
    checks.body = format!("(local $h eqref)\n{}", checks.body);
    checks.run();
}

#[test]
fn core_operations_follow_the_specification() {
    let mut checks = checks();
    let inspect = [
        // Int division by zero and rounding (spec §11.2).
        (format!("(call $int_div {} {})", int(7), int(0)), "0"),
        (format!("(call $int_rem {} {})", int(7), int(0)), "0"),
        (
            format!(
                "(call $int_div {} {})",
                big("100000000000000000000"),
                int(0)
            ),
            "0",
        ),
        (
            format!(
                "(call $int_rem {} {})",
                big("-100000000000000000000"),
                int(0)
            ),
            "0",
        ),
        (format!("(call $int_div {} {})", int(-7), int(2)), "-3"),
        (format!("(call $int_rem {} {})", int(-7), int(2)), "-1"),
        (format!("(call $int_rem {} {})", int(7), int(-2)), "1"),
        (
            format!(
                "(call $int_div {} {})",
                big("-100000000000000000001"),
                big("10000000000")
            ),
            "-10000000000",
        ),
        (
            format!(
                "(call $int_rem {} {})",
                big("-100000000000000000001"),
                big("10000000000")
            ),
            "-1",
        ),
        (
            format!("(call $int_div {} {})", int(-1073741824), int(-1)),
            "1073741824",
        ),
        // Floats (spec §11.3).
        (
            format!("(call $float_div {} {})", float(1.0), float(0.0)),
            "0.0",
        ),
        (
            format!("(call $float_div {} {})", float(-1.0), float(-0.0)),
            "0.0",
        ),
        (
            format!("(call $float_mul {} {})", float(0.0), float(-1.0)),
            "-0.0",
        ),
        (format!("(call $float_neg {})", float(0.0)), "-0.0"),
        (
            format!("(call $float_sub {} {})", float(0.1), float(0.3)),
            "-0.19999999999999998",
        ),
        // i/core (spec §13).
        (
            "(call $core_not (call $bool_new (i32.const 1)))".to_owned(),
            "False",
        ),
        (
            format!("(call $core_to_float {})", big("9007199254740993")),
            "9.007199254740992e15",
        ),
        (
            format!("(call $core_to_float {})", big("9007199254740995")),
            "9.007199254740996e15",
        ),
        (format!("(call $core_to_float {})", int(-3)), "-3.0"),
        (format!("(call $core_truncate {})", float(-2.7)), "-2"),
        (format!("(call $core_truncate {})", float(-0.5)), "0"),
        (
            format!("(call $core_truncate {})", float(1e20)),
            "100000000000000000000",
        ),
        (format!("(call $core_floor {})", float(-0.5)), "-1.0"),
        (format!("(call $core_ceiling {})", float(-0.5)), "-0.0"),
        (
            format!("(call $core_ceiling {})", float(1.000_000_1)),
            "2.0",
        ),
        (format!("(call $core_sqrt {})", float(-0.0)), "-0.0"),
        (
            format!("(call $core_sqrt {})", float(2.0)),
            "1.4142135623730951",
        ),
        (
            format!("(call $core_float_to_bits {})", float(-0.0)),
            "9223372036854775808",
        ),
        (
            format!("(call $core_float_to_bits {})", float(1.0)),
            "4607182418800017408",
        ),
        (
            format!(
                "(call $core_float_from_bits {})",
                big("9223372036854775808")
            ),
            "-0.0",
        ),
        (
            format!(
                "(call $core_float_from_bits {})",
                big("18446744073709551616")
            ),
            "0.0",
        ),
        (
            format!(
                "(call $core_float_from_bits {})",
                big("23056178292323254272")
            ),
            "1.5",
        ),
        (
            format!(
                "(call $core_float_from_bits {})",
                big("-9223372036854775808")
            ),
            "-0.0",
        ),
        (
            format!("(call $core_float_from_bits {})", int(-4616189618054758400)),
            "-1.0",
        ),
        (format!("(call $core_bit_and {} {})", int(-1), int(5)), "5"),
        (format!("(call $core_bit_or {} {})", int(-8), int(3)), "-5"),
        (format!("(call $core_bit_xor {} {})", int(-1), int(5)), "-6"),
        (format!("(call $core_bit_not {})", int(5)), "-6"),
        (
            format!("(call $core_bit_not {})", big("-18446744073709551616")),
            "18446744073709551615",
        ),
        (
            format!(
                "(call $core_bit_and {} {})",
                big("-18446744073709551616"),
                big("36893488147419103231")
            ),
            "18446744073709551616",
        ),
        (
            format!("(call $core_shift_left {} {})", int(3), int(100)),
            "3802951800684688204490109616128",
        ),
        (
            format!("(call $core_shift_left {} {})", int(-5), int(-1)),
            "-3",
        ),
        (
            format!("(call $core_shift_left {} {})", int(1), int(-1)),
            "0",
        ),
        (
            format!("(call $core_shift_right {} {})", int(-5), int(1)),
            "-3",
        ),
        (
            format!("(call $core_shift_right {} {})", int(-1), int(1000)),
            "-1",
        ),
        (
            format!("(call $core_shift_right {} {})", int(1), int(-3)),
            "8",
        ),
        (
            format!(
                "(call $core_shift_right {} {})",
                big("-36893488147419103233"),
                int(65)
            ),
            "-2",
        ),
        (
            format!(
                "(call $core_concat {})",
                list(&[string("a"), string(""), string("é")])
            ),
            "\"aé\"",
        ),
        (format!("(call $core_concat {})", list(&[])), "\"\""),
        (format!("(call $core_byte_size {})", string("aé🙂")), "7"),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                int(1),
                int(2)
            ),
            "Ok(\"é\")",
        ),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                int(3),
                int(0)
            ),
            "Ok(\"\")",
        ),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                int(0),
                int(0)
            ),
            "Ok(\"\")",
        ),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                int(1),
                int(1)
            ),
            "Error(Nil)",
        ),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                int(2),
                int(1)
            ),
            "Error(Nil)",
        ),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                int(4),
                int(0)
            ),
            "Error(Nil)",
        ),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                int(-1),
                int(1)
            ),
            "Error(Nil)",
        ),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                int(0),
                int(-1)
            ),
            "Error(Nil)",
        ),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                int(2),
                int(2)
            ),
            "Error(Nil)",
        ),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                big("18446744073709551616"),
                int(0)
            ),
            "Error(Nil)",
        ),
        (
            format!(
                "(call $core_byte_slice {} {} {})",
                string("aé"),
                int(0),
                big("18446744073709551619")
            ),
            "Error(Nil)",
        ),
        (
            format!("(call $core_codepoints {})", string("aé🙂")),
            "[//utfcodepoint(a), //utfcodepoint(é), //utfcodepoint(🙂)]",
        ),
        (
            format!(
                "(call $core_from_codepoints (call $core_codepoints {}))",
                string("aé🙂")
            ),
            "\"aé🙂\"",
        ),
        (
            "(call $core_codepoint_to_int (call $codepoint_new (i32.const 1114111)))".to_owned(),
            "1114111",
        ),
        (
            format!("(call $core_int_to_codepoint {})", int(65)),
            "Ok(//utfcodepoint(A))",
        ),
        (
            format!("(call $core_int_to_codepoint {})", int(0)),
            "Ok(//utfcodepoint(\u{0}))",
        ),
        (
            format!("(call $core_int_to_codepoint {})", int(55296)),
            "Error(Nil)",
        ),
        (
            format!("(call $core_int_to_codepoint {})", int(57343)),
            "Error(Nil)",
        ),
        (
            format!("(call $core_int_to_codepoint {})", int(57344)),
            "Ok(//utfcodepoint(\u{e000}))",
        ),
        (
            format!("(call $core_int_to_codepoint {})", int(1114112)),
            "Error(Nil)",
        ),
        (
            format!("(call $core_int_to_codepoint {})", int(-1)),
            "Error(Nil)",
        ),
        (
            format!("(call $core_int_to_codepoint {})", big("4294967361")),
            "Error(Nil)",
        ),
        (
            format!("(call $core_bit_size {})", bits(&[0xff, 0xff], 3, 11)),
            "11",
        ),
        (
            format!(
                "(call $core_bit_array_to_string {})",
                bits(&[0xc3, 0xa9], 0, 16)
            ),
            "Ok(\"é\")",
        ),
        (
            format!(
                "(call $core_bit_array_to_string {})",
                bits(&[0x0c, 0x3a, 0x90], 4, 16)
            ),
            "Ok(\"é\")",
        ),
        (
            format!("(call $core_bit_array_to_string {})", bits(&[0xc3], 0, 8)),
            "Error(Nil)",
        ),
        (
            format!("(call $core_bit_array_to_string {})", bits(&[0x61], 0, 7)),
            "Error(Nil)",
        ),
        (
            format!("(call $core_bit_array_to_string {})", bits(&[], 0, 0)),
            "Ok(\"\")",
        ),
        (
            format!("(call $core_inspect {})", list(&[int(1)])),
            "\"[1]\"",
        ),
    ];
    for (expression, expected) in inspect {
        checks.inspect(&expression, expected);
    }
    checks.check(
        "core.hash gives an Int",
        format!(
            "(i32.ge_s (call $int_sign (call $core_hash {})) (i32.const 0))",
            string("x")
        ),
    );
    for (name, body, reason) in [
        (
            "float overflow",
            format!(
                "(drop (call $float_mul {} {}))",
                float(f64::MAX),
                float(2.0)
            ),
            "float: result is not finite",
        ),
        (
            "float add overflow",
            format!(
                "(drop (call $float_add {} {}))",
                float(f64::MAX),
                float(f64::MAX)
            ),
            "float: result is not finite",
        ),
        (
            "float sub overflow",
            format!(
                "(drop (call $float_sub {} {}))",
                float(-f64::MAX),
                float(f64::MAX)
            ),
            "float: result is not finite",
        ),
        (
            "float div overflow",
            format!(
                "(drop (call $float_div {} {}))",
                float(1e300),
                float(1e-300)
            ),
            "float: result is not finite",
        ),
        (
            "sqrt of a negative number",
            format!("(drop (call $core_sqrt {}))", float(-1.0)),
            "float: result is not finite",
        ),
        (
            "to_float of 2^1024",
            format!("(drop (call $core_to_float {}))", big(&num_text(1024))),
            "float: result is not finite",
        ),
        (
            "to_float rounding to infinity",
            format!(
                "(drop (call $core_to_float (call $int_sub {} {})))",
                big(&num_text(1024)),
                big(&num_text(970))
            ),
            "float: result is not finite",
        ),
        (
            "float_from_bits of -1",
            format!("(drop (call $core_float_from_bits {}))", int(-1)),
            "float: result is not finite",
        ),
        (
            "float_from_bits of infinity",
            format!(
                "(drop (call $core_float_from_bits {}))",
                int(0x7ff0_0000_0000_0000)
            ),
            "float: result is not finite",
        ),
        (
            "negative bit array length",
            format!("(drop (call $bits_length {} (i64.const 8)))", int(-1)),
            "bit array: length is negative",
        ),
        (
            "bits(n) too short",
            format!(
                "(drop (call $bits_take {} (i64.const 9)))",
                bits(&[0], 0, 8)
            ),
            "bit array: value is too short",
        ),
        (
            "bytes not whole",
            format!("(drop (call $bits_whole {}))", bits(&[0], 0, 7)),
            "bit array: bytes are not whole",
        ),
        (
            "f16 overflow",
            format!(
                "(drop (call $bits_from_float {} (i64.const 16) (i32.const 0)))",
                float(65520.0)
            ),
            "bit array: float is not finite",
        ),
        (
            "f32 overflow",
            format!(
                "(drop (call $bits_from_float {} (i64.const 32) (i32.const 1)))",
                float(1e300)
            ),
            "bit array: float is not finite",
        ),
    ] {
        checks.stops(name, &body, reason);
    }
    checks.check(
        "to_float of the largest finite integer",
        format!(
            "(f64.eq (call $float_get (call $core_to_float (call $int_sub {} {}))) (f64.const 0x1.fffffffffffffp+1023))",
            big(&num_text(1024)),
            big(&num_text(971))
        ),
    );
    checks.check(
        "bit array construction accepts exact lengths",
        format!(
            "(i32.and (i64.eq (call $bits_length {} (i64.const 8)) (i64.const 16))
               (i32.and (call $equal (call $bits_take {} (i64.const 3)) {})
                 (call $equal (call $bits_whole {}) {})))",
            int(2),
            bits(&[0b1010_0000], 0, 8),
            bits(&[0b1010_0000], 0, 3),
            bits(&[1], 0, 8),
            bits(&[1], 0, 8)
        ),
    );
    checks.run();
}

/// 2^n in decimal.
fn num_text(n: u32) -> String {
    (num_bigint::BigInt::from(1) << n).to_string()
}

#[test]
fn host_boundary_values_round_trip_and_reject_incorrect_values() {
    let mut checks = checks();
    checks.define(
        "(func $int_round_trip (param $v eqref) (result i32)
           (local $sign i32) (local $address i32) (local $count i32)
           (call $abi_int_out (local.get $v))
           (local.set $count) (local.set $address) (local.set $sign)
           (call $equal (local.get $v)
             (call $abi_int_in (local.get $sign) (local.get $address) (local.get $count))))
         (func $string_round_trip (param $v eqref) (result i32)
           (local $address i32) (local $length i32)
           (call $abi_string_out (local.get $v))
           (local.set $length) (local.set $address)
           (call $equal (local.get $v)
             (call $abi_string_in (local.get $address) (local.get $length))))
         (func $bits_round_trip (param $v eqref) (result i32)
           (local $address i32) (local $length i32)
           (call $abi_bits_out (local.get $v))
           (local.set $length) (local.set $address)
           (call $equal (local.get $v)
             (call $abi_bits_in (local.get $address) (local.get $length))))",
    );
    for value in [
        int(0),
        int(-1),
        int(1 << 40),
        big("-340282366920938463463374607431768211457"),
    ] {
        checks.check(
            format!("int {value}"),
            format!("(call $int_round_trip {value})"),
        );
    }
    for value in ["", "a", "中文🙂\0"] {
        checks.check(
            format!("string {value:?}"),
            format!("(call $string_round_trip {})", string(value)),
        );
    }
    for (bytes, offset, length) in [
        (&[][..], 0, 0),
        (&[0xff, 0xff][..], 3, 9),
        (&[0b1010_1010, 0xff, 0x0f][..], 1, 17),
        (&[0x12, 0x34][..], 0, 16),
    ] {
        checks.check(
            format!("bits {bytes:?} {offset} {length}"),
            format!("(call $bits_round_trip {})", bits(bytes, offset, length)),
        );
    }
    checks.check(
        "bits after the end are 0",
        format!(
            "(block (result i32) (call $abi_bits_out {})
               (drop) (local.set $address) (i32.eq (i32.load8_u (local.get $address)) (i32.const 224)))",
            bits(&[0xff, 0xff], 1, 3)
        ),
    );
    // B.3: the results of an import are freed also when they are empty, as
    // a block of `ilang_alloc(0)`; the next one is the same block.
    checks.define(
        "(func $empty_results_are_freed (result i32)
           (local $a i32) (local $ok i32)
           (local.set $a (call $alloc (i32.const 0)))
           (drop (call $abi_string_take (local.get $a) (i32.const 0)))
           (local.set $ok (i32.eq (call $alloc (i32.const 0)) (local.get $a)))
           (drop (call $abi_bits_take (local.get $a) (i32.const 0)))
           (local.set $ok (i32.and (local.get $ok) (i32.eq (call $alloc (i32.const 0)) (local.get $a))))
           (drop (call $abi_int_take (i32.const 0) (local.get $a) (i32.const 0)))
           (local.set $ok (i32.and (local.get $ok) (i32.eq (call $alloc (i32.const 0)) (local.get $a))))
           ;; The address 0 is not freed.
           (drop (call $abi_string_take (i32.const 0) (i32.const 0)))
           (i32.and (local.get $ok) (i32.ne (call $alloc (i32.const 0)) (i32.const 0))))",
    );
    checks.check("empty results are freed", "(call $empty_results_are_freed)");
    checks.check(
        "empty buffers are not allocated",
        "(block (result i32) (call $abi_string_out (call $string_from_bytes (array.new_fixed $bytes 0)))
           (drop) (i32.eqz))",
    );
    checks.check(
        "bool and float",
        "(i32.and (call $equal (call $abi_bool_in (i32.const 1)) (call $bool_new (i32.const 1)))
           (f64.eq (call $abi_float_out (call $abi_float_in (f64.const -2.5))) (f64.const -2.5)))",
    );
    checks.check(
        "error tags",
        "(i32.eq (call $abi_tag_in (i32.const 6) (i32.const 6)) (i32.const 6))",
    );
    for (name, body) in [
        ("bool 2", "(drop (call $abi_bool_in (i32.const 2)))"),
        ("NaN", "(drop (call $abi_float_in (f64.const nan)))"),
        ("infinity", "(drop (call $abi_float_in (f64.const -inf)))"),
        (
            "int sign 2",
            "(drop (call $abi_int_in (i32.const 2) (i32.const 0) (i32.const 0)))",
        ),
        (
            "zero with parts",
            "(drop (call $abi_int_in (i32.const 0) (i32.const 64) (i32.const 1)))",
        ),
        (
            "int without parts",
            "(drop (call $abi_int_in (i32.const 1) (i32.const 0) (i32.const 0)))",
        ),
        (
            "last part 0",
            "(i32.store (i32.const 64) (i32.const 0)) (drop (call $abi_int_in (i32.const 1) (i32.const 64) (i32.const 1)))",
        ),
        (
            "int outside memory",
            "(drop (call $abi_int_in (i32.const 1) (i32.const -4) (i32.const 1)))",
        ),
        (
            "string not UTF-8",
            "(i32.store8 (i32.const 64) (i32.const 255)) (drop (call $abi_string_in (i32.const 64) (i32.const 1)))",
        ),
        (
            "surrogate",
            "(i32.store (i32.const 64) (i32.const 0xa0ed)) (i32.store8 (i32.const 66) (i32.const 0x80)) (drop (call $abi_string_in (i32.const 64) (i32.const 3)))",
        ),
        (
            "string outside memory",
            "(drop (call $abi_string_in (i32.const -1) (i32.const 2)))",
        ),
        // An empty buffer must be inside memory too, for every type (B.2).
        (
            "empty string outside memory",
            "(drop (call $abi_string_in (i32.const -1) (i32.const 0)))",
        ),
        (
            "empty bit array outside memory",
            "(drop (call $abi_bits_in (i32.const -1) (i32.const 0)))",
        ),
        (
            "zero outside memory",
            "(drop (call $abi_int_in (i32.const 0) (i32.const -1) (i32.const 0)))",
        ),
        (
            "bits after the end",
            "(i32.store8 (i32.const 64) (i32.const 1)) (drop (call $abi_bits_in (i32.const 64) (i32.const 7)))",
        ),
        (
            "tag too big",
            "(drop (call $abi_tag_in (i32.const 3) (i32.const 2)))",
        ),
    ] {
        checks.stops(
            format!("host value {name}"),
            body,
            "host: value is not correct",
        );
    }
    checks.body = format!("(local $address i32)\n{}", checks.body);
    checks.run();
}
