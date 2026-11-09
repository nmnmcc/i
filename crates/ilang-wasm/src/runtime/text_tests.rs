//! Tests of `to_upper`, `to_lower`, `index_of` and `bit_array_concat` in the
//! runtime, run in the pinned Wasmtime like the tests in `tests.rs`.

use super::case_tables::UNICODE_VERSION;
use super::tests::{Checks, STUB_HOST, run_export};
use super::{TYPES, literal, wat};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// The simple uppercase and lowercase mappings of the vendored
/// UnicodeData.txt, read again here without the generator.
fn mappings() -> (BTreeMap<u32, u32>, BTreeMap<u32, u32>) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/unicode/UnicodeData.txt");
    let text = std::fs::read_to_string(path).expect("UnicodeData.txt");
    let (mut upper, mut lower) = (BTreeMap::new(), BTreeMap::new());
    for line in text.lines() {
        let fields: Vec<&str> = line.split(';').collect();
        let code = u32::from_str_radix(fields[0], 16).unwrap();
        for (field, table) in [(12, &mut upper), (13, &mut lower)] {
            if !fields[field].is_empty() {
                table.insert(code, u32::from_str_radix(fields[field], 16).unwrap());
            }
        }
    }
    (upper, lower)
}

/// A module whose export `verify` maps every Unicode scalar value with
/// `$case_map` and the table `table`, and compares it with `expected`. It
/// gives 0, or the first wrong codepoint plus 1.
fn exhaustive_module(table: &str, expected: &BTreeMap<u32, u32>) -> String {
    let mut data = String::new();
    for (code, mapped) in expected {
        for word in [*code, *mapped] {
            for byte in word.to_le_bytes() {
                write!(data, "\\{byte:02x}").unwrap();
            }
        }
    }
    let words = expected.len() * 2;
    format!(
        r#"(module {TYPES} {STUB_HOST} {runtime}
(data $expected "{data}")
(func (export "verify") (result i32)
  (local $pairs (ref $case_ranges)) (local $next i32) (local $cp i32) (local $want i32)
  (local.set $pairs (array.new_data $case_ranges $expected (i32.const 0) (i32.const {words})))
  (block $done (loop $each
    (br_if $done (i32.gt_u (local.get $cp) (i32.const 0x10ffff)))
    (local.set $want (local.get $cp))
    (if (i32.lt_u (local.get $next) (array.len (local.get $pairs)))
      (then (if (i32.eq (array.get $case_ranges (local.get $pairs) (local.get $next)) (local.get $cp))
        (then
          (local.set $want (array.get $case_ranges (local.get $pairs) (i32.add (local.get $next) (i32.const 1))))
          (local.set $next (i32.add (local.get $next) (i32.const 2)))))))
    (if (i32.ne (call $case_map (call ${table}) (local.get $cp)) (local.get $want))
      (then (return (i32.add (local.get $cp) (i32.const 1)))))
    (local.set $cp (i32.add (local.get $cp) (i32.const 1)))
    ;; Surrogates are not scalar values.
    (if (i32.eq (local.get $cp) (i32.const 0xd800)) (then (local.set $cp (i32.const 0xe000))))
    (br $each)))
  (if (i32.ne (local.get $next) (array.len (local.get $pairs))) (then (return (i32.const -1))))
  (i32.const 0)))"#,
        runtime = wat(),
    )
}

#[test]
fn case_tables_give_every_simple_mapping_of_unicode_data() {
    assert_eq!(UNICODE_VERSION, "18.0.0");
    let (upper, lower) = mappings();
    for (table, expected) in [("case_upper_table", &upper), ("case_lower_table", &lower)] {
        let failed = run_export(&exhaustive_module(table, expected), "verify");
        assert_eq!(
            failed,
            0,
            "{table}: wrong mapping of codepoint {}",
            failed.wrapping_sub(1)
        );
    }
}

fn int(value: i64) -> String {
    format!("(call $int_from_i64 (i64.const {value}))")
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

#[test]
fn case_mapping_of_strings() {
    let mut checks = Checks::default();
    let cases = [
        ("", "", ""),
        ("Gleam 123!", "GLEAM 123!", "gleam 123!"),
        ("héllo", "HÉLLO", "héllo"),
        ("Ωμέγα", "ΩΜΈΓΑ", "ωμέγα"),
        ("Привет", "ПРИВЕТ", "привет"),
        ("straße", "STRAßE", "straße"),
        ("ẞ", "ẞ", "ß"),
        ("ıi", "II", "ıi"),
        ("İ", "İ", "i"),
        ("ǅ", "Ǆ", "ǆ"),
        ("\u{212A}", "\u{212A}", "k"),
        ("ⱥȺ", "ȺȺ", "ⱥⱥ"),
        ("𐐨𐐀", "𐐀𐐀", "𐐨𐐨"),
        ("ŉ", "ŉ", "ŉ"),
        ("日本", "日本", "日本"),
        ("a😀b", "A😀B", "a😀b"),
    ];
    for (input, upper, lower) in cases {
        checks.string(
            format!("to_upper {input}"),
            &format!("(call $core_to_upper {})", literal(input)),
            upper,
        );
        checks.string(
            format!("to_lower {input}"),
            &format!("(call $core_to_lower {})", literal(input)),
            lower,
        );
    }
    // A string that grows past the room given at first.
    let long = "ɐ".repeat(40);
    checks.string(
        "to_upper grows",
        &format!("(call $core_to_upper {})", literal(&long)),
        &"Ɐ".repeat(40),
    );
    let long = format!("{}{}", "x".repeat(30), "ⱥ".repeat(30));
    checks.string(
        "to_lower after a long prefix",
        &format!("(call $core_to_lower {})", literal(&long.to_uppercase())),
        &long,
    );
    checks.define(
        "(global $same_input (mut eqref) (ref.null eq))
         (func $unchanged (param $s eqref) (param $mapped eqref) (result i32)
           (ref.eq (local.get $s) (local.get $mapped)))",
    );
    checks.check(
        "a string with nothing to change is the same value",
        format!(
            "(block (result i32) (global.set $same_input {}) (call $unchanged (global.get $same_input) (call $core_to_upper (global.get $same_input))))",
            literal("ÀB 1")
        ),
    );
    checks.run();
}

#[test]
fn index_of_finds_bytes_at_character_boundaries() {
    let mut checks = Checks::default();
    let found = |s: &str, find: &str, from: String, want: &str, checks: &mut Checks| {
        checks.string(
            format!("index_of({s:?}, {find:?}, {from})"),
            &format!(
                "(call $inspect (call $core_index_of {} {} {from}))",
                literal(s),
                literal(find)
            ),
            want,
        );
    };
    found("hello", "l", int(0), "Ok(2)", &mut checks);
    found("hello", "l", int(3), "Ok(3)", &mut checks);
    found("hello", "l", int(4), "Error(Nil)", &mut checks);
    found("hello", "lo", int(0), "Ok(3)", &mut checks);
    found("hello", "hello", int(0), "Ok(0)", &mut checks);
    found("hello", "hello!", int(0), "Error(Nil)", &mut checks);
    found("hello", "", int(0), "Ok(0)", &mut checks);
    found("hello", "", int(5), "Ok(5)", &mut checks);
    found("hello", "", int(6), "Error(Nil)", &mut checks);
    found("hello", "o", int(5), "Error(Nil)", &mut checks);
    found("hello", "l", int(-1), "Error(Nil)", &mut checks);
    found(
        "hello",
        "l",
        "(call $int_from_i64 (i64.const 4294967297))".into(),
        "Error(Nil)",
        &mut checks,
    );
    found("", "", int(0), "Ok(0)", &mut checks);
    found("", "a", int(0), "Error(Nil)", &mut checks);
    found("héé", "é", int(0), "Ok(1)", &mut checks);
    found("héé", "é", int(2), "Error(Nil)", &mut checks);
    found("héé", "", int(2), "Error(Nil)", &mut checks);
    found("héé", "é", int(3), "Ok(3)", &mut checks);
    found("aaaaaaab", "aaab", int(0), "Ok(4)", &mut checks);
    found("abababca", "ababc", int(0), "Ok(2)", &mut checks);
    found("abcabcabd", "abcabd", int(0), "Ok(3)", &mut checks);
    found("aabaabaaab", "aabaaab", int(1), "Ok(3)", &mut checks);
    found("😀😁😀", "😀", int(1), "Error(Nil)", &mut checks);
    found("😀😁😀", "😀", int(4), "Ok(8)", &mut checks);
    checks.run();
}

#[test]
fn index_of_agrees_with_a_plain_search() {
    // Every pattern up to length 4 over "ab", in every text up to length 8.
    let words = |max: usize| {
        let mut all = vec![String::new()];
        let mut layer = vec![String::new()];
        for _ in 0..max {
            layer = layer
                .iter()
                .flat_map(|w| [format!("{w}a"), format!("{w}b")])
                .collect();
            all.extend(layer.iter().cloned());
        }
        all
    };
    let texts: Vec<String> = words(7).into_iter().step_by(5).collect();
    let mut checks = Checks::default();
    for text in &texts {
        for pattern in words(4).iter().filter(|p| !p.is_empty()) {
            for from in [0, 1, 3] {
                if from > text.len() {
                    continue;
                }
                let want = match text[from..].find(pattern.as_str()) {
                    Some(i) => format!("Ok({})", i + from),
                    None => "Error(Nil)".into(),
                };
                checks.string(
                    format!("index_of({text:?}, {pattern:?}, {from})"),
                    &format!(
                        "(call $inspect (call $core_index_of {} {} {}))",
                        literal(text),
                        literal(pattern),
                        int(from as i64)
                    ),
                    &want,
                );
            }
        }
    }
    checks.run();
}

/// Parts with offsets and lengths that are not whole bytes, joined in one
/// step, equal the parts joined two at a time, and the bits after the end
/// of the result are 0.
#[test]
fn bit_array_concat_joins_parts_of_any_length() {
    let mut checks = Checks::default();
    checks.define(
        "(func $tail_is_zero (param $v eqref) (result i32)
           (local $length i64) (local $data (ref $bytes)) (local $rest i32)
           (local.set $length (call $bits_len64 (local.get $v)))
           (local.set $data (call $bits_data (local.get $v)))
           (local.set $rest (i32.wrap_i64 (i64.and (local.get $length) (i64.const 7))))
           (if (i32.eqz (local.get $rest)) (then (return (i32.const 1))))
           (i32.eqz (i32.and
             (array.get_u $bytes (local.get $data) (i32.wrap_i64 (i64.shr_u (local.get $length) (i64.const 3))))
             (i32.shr_u (i32.const 0xff) (local.get $rest)))))
         (func $joined (param $parts eqref) (result eqref)
           (local $out eqref)
           (local.set $out (call $bits_empty))
           (block $done (loop $each
             (br_if $done (call $list_is_empty (local.get $parts)))
             (local.set $out (call $bits_concat (local.get $out) (call $list_head (local.get $parts))))
             (local.set $parts (call $list_tail (local.get $parts)))
             (br $each)))
           (local.get $out))
         (func $concat_ok (param $parts eqref) (result i32)
           (local $got eqref)
           (local.set $got (call $core_bit_array_concat (local.get $parts)))
           (i32.and (call $equal (local.get $got) (call $joined (local.get $parts)))
                    (call $tail_is_zero (local.get $got))))",
    );
    let source: Vec<u8> = vec![0xa5, 0xff, 0x3c, 0x81, 0x7e, 0xc3, 0x0f, 0xf0, 0x55];
    let total = source.len() as u64 * 8;
    // (offset, length) views of `source`: whole bytes, and bits on either side.
    let views: Vec<(u32, u64)> = vec![
        (0, 0),
        (0, 8),
        (0, 16),
        (8, 24),
        (0, 1),
        (3, 5),
        (1, 7),
        (5, 11),
        (7, 17),
        (2, 3),
        (4, 40),
        (8, 13),
        (0, total),
        (9, total - 9),
    ];
    let view = |&(offset, length): &(u32, u64)| bits(&source, offset, length);
    // Every ordered pair and some longer lists.
    for a in &views {
        for b in &views {
            checks.check(
                format!("concat {a:?} {b:?}"),
                format!("(call $concat_ok {})", list(&[view(a), view(b)])),
            );
        }
    }
    for window in views.windows(4) {
        let parts: Vec<String> = window.iter().map(view).collect();
        checks.check(
            format!("concat {window:?}"),
            format!("(call $concat_ok {})", list(&parts)),
        );
    }
    let all: Vec<String> = views.iter().map(view).collect();
    checks.check("concat all", format!("(call $concat_ok {})", list(&all)));
    checks.check(
        "concat []",
        format!(
            "(call $equal (call $core_bit_array_concat {}) (call $bits_empty))",
            list(&[])
        ),
    );
    // Text forms, to see the order of the bits.
    checks.string(
        "concat 1:u1, 2:u2, 255",
        &format!(
            "(call $inspect (call $core_bit_array_concat {}))",
            list(&[
                bits(&[0x80], 0, 1),
                bits(&[0x80], 0, 2),
                bits(&[0xff], 0, 8)
            ])
        ),
        "<<223, 7:u3>>",
    );
    checks.run();
}
