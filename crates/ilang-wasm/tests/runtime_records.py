"""Record-runtime oracle from records.md sections 9.1–9.5.

Fixtures use the internal runtime form. The provisional source spelling
appears only in inspect_text, so a surface change has one expectation site.
"""
import itertools
import json
import re


def tests(string):
    def integer(value):
        return f"(call $int_from_i64 (i64.const {value}))"

    def array(fields):
        pairs = " ".join(
            f"{string(key)} {integer(value)}"
            for key, value in sorted(fields.items(), key=lambda item: item[0].encode())
        )
        return f"(array.new_fixed $values {2 * len(fields)} {pairs})"

    def record(fields):
        return f"(call $record_new {array(fields)})"

    def equal(a, b):
        return f"(call $assert (call $value_eq {a} {b}))"

    def inspect_text(fields):
        if not fields:
            return "()"
        def label(key):
            return key if re.fullmatch(r"_?[a-z][a-z0-9_]*", key) else json.dumps(key, ensure_ascii=False)
        return "(" + ", ".join(
            f"{label(key)}: {value}"
            for key, value in sorted(fields.items(), key=lambda item: item[0].encode())
        ) + ")"

    out = ['(func (export "structural_records") (result i32) (local $a eqref) (local $b eqref) (local $c eqref)']
    labels = ["", "_a", "a", "a b", "fooBar", "é", "😀"]
    for bits in range(1 << len(labels)):
        fields = {label: i + 1 for i, label in enumerate(labels) if bits & (1 << i)}
        out.append(f"(local.set $a {record(fields)})")
        built = record({})
        for label, value in reversed(list(fields.items())):
            built = f"(call $record_put {built} {string(label)} {integer(value)})"
        out.append(f"(local.set $b {built})")
        out.append(equal('(local.get $a)', '(local.get $b)'))
        out.append('(call $assert (i32.eq (call $value_hash_u32 (local.get $a)) (call $value_hash_u32 (local.get $b))))')
        out.append(equal('(call $record_size (local.get $a))', integer(len(fields))))
        out.append(equal('(call $inspect (local.get $a))', string(inspect_text(fields))))
        keys = '(call $list_nil)'
        texts = '(call $list_nil)'
        for key, value in reversed(sorted(fields.items(), key=lambda item: item[0].encode())):
            keys = f'(call $list_cons {string(key)} {keys})'
            entry = f'(call $tuple (array.new_fixed $values 2 {string(key)} {string(str(value))}))'
            texts = f'(call $list_cons {entry} {texts})'
        out.append(equal('(call $record_keys (local.get $a))', keys))
        out.append(equal('(call $record_texts (local.get $a))', texts))
        for key in labels:
            found = key in fields
            out.append(f'(call $assert (i32.eq (call $bool_get (call $record_has (local.get $a) {string(key)})) (i32.const {int(found)})))')
            if found:
                out.append(equal(f'(call $record_get (local.get $a) {string(key)})', integer(fields[key])))
            removed = {k: value for k, value in fields.items() if k != key}
            out.append(equal(f'(call $record_remove (local.get $a) {string(key)})', record(removed)))
            changed = dict(fields, **{key: 99})
            out.append(equal(f'(call $record_put (local.get $a) {string(key)} {integer(99)})', record(changed)))
        # §9.2 includes labels and size, and distinguishes tuple/record tags.
        out.append(f'(call $assert (i32.eqz (call $value_eq (local.get $a) (call $tuple {array(fields)}))))')
    for left, right in itertools.product(range(16), repeat=2):
        a = {key: 10 + i for i, key in enumerate('abcd') if left & (1 << i)}
        b = {key: 20 + i for i, key in enumerate('abcd') if right & (1 << i)}
        expected = {**a, **b}
        out.append(equal(f'(call $record_with {record(a)} {array(b)})', record(expected)))
    out.append('(i32.const 1))')
    out.append(f'(func (export "record_missing_traps") (drop (call $record_get {record({})} {string("missing")})))')
    out.append(f'''(func (export "record_identity_labels") (result i32)
      (local $key eqref) (local $a eqref)
      (local.set $key {string("key")})
      (local.set $a (call $record_put {record({})} (local.get $key) {integer(7)}))
      (call $assert (ref.eq (array.get $values (call $record_fields (local.get $a)) (i32.const 0)) (local.get $key)))
      {equal('(call $record_get (local.get $a) (local.get $key))', integer(7))}
      {equal(f'(call $record_get (local.get $a) {string("key")})', integer(7))}
      (i32.const 1))''')
    out.append(f'''(func (export "deep_structural_records") (result i32)
      (local $a eqref) (local $b eqref) (local $key eqref) (local $i i32)
      (local.set $key {string("next")})
      (loop $build
        (local.set $a (call $record_new (array.new_fixed $values 2 (local.get $key) (local.get $a))))
        (local.set $b (call $record_new (array.new_fixed $values 2 (local.get $key) (local.get $b))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br_if $build (i32.lt_u (local.get $i) (i32.const 10000))))
      {equal('(local.get $a)', '(local.get $b)')}
      (call $assert (i32.eq (call $value_hash_u32 (local.get $a)) (call $value_hash_u32 (local.get $b))))
      (call $assert (i32.eq (call $string_length_bytes (call $inspect (local.get $a))) (i32.const 80003)))
      (i32.const 1))''')
    return '\n'.join(out)
