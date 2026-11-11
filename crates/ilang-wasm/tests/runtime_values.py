#!/usr/bin/env python3
"""Validate and execute the shared GC value primitives independently of lowering.

Usage: python3 tests/runtime_values.py --types TYPES.wat --runtime RUNTIME.wat
       [--wasm-tools PATH] [--wasmtime PATH]
The Rust test runtime_values.rs writes the runtime's TYPES and wat() to the two
files. The reference module adds stub host imports and test-only exports.
"""

import argparse
import pathlib
import re
import subprocess
import tempfile
import sys

sys.dont_write_bytecode = True
import runtime_records


ROOT = pathlib.Path(__file__).resolve().parents[1] / "src" / "runtime"


def raw(path, constant=None):
    prefix = rf"const {constant}: &str = " if constant else ""
    return re.search(prefix + r'r#"(.*?)"#', path.read_text(), re.S).group(1)


def string(value):
    encoded = value.encode("utf-8")
    values = " ".join(f"(i32.const {byte})" for byte in encoded)
    return f"(call $string_from_bytes (array.new_fixed $bytes {len(encoded)} {values}))"


PRELUDE = """
(global $panic_address (mut i32) (i32.const 0))
(global $panic_length (mut i32) (i32.const 0))
;; The runtime traps with `unreachable` after the panic import returns.
(func $host_panic (param $address i32) (param $length i32)
  (global.set $panic_address (local.get $address))
  (global.set $panic_length (local.get $length)))
(func $host_reference_id (param externref) (result i64) (i64.const 1))
(func $assert(param $v i32)(if(i32.eqz(local.get $v))(then unreachable)))
"""

TESTS = """
(func (export "correctness") (result i32)
  (local $a eqref) (local $b eqref) (local $c eqref)
  (call $assert (call $value_eq (ref.null eq) (ref.null eq)))
  (call $assert (i32.eqz (call $value_eq (call $bool_new(i32.const 1)) (ref.i31(i32.const 1)))))
  (call $assert (call $value_eq (call $bool_new(i32.const 1)) (call $bool_new(i32.const 9))))
  (call $assert (call $value_eq (call $float_new(f64.const 0)) (call $float_new(f64.const -0))))
  (call $assert (i32.eq (call $value_hash_u32(call $float_new(f64.const 0)))
                       (call $value_hash_u32(call $float_new(f64.const -0)))))
  (local.set $a (call $int_from_i64(i64.const 1099511627776)))
  (local.set $b (call $int_add(call $int_from_i64(i64.const 1099511627775))(ref.i31(i32.const 1))))
  (call $assert (call $value_eq(local.get $a)(local.get $b)))
  (call $assert (i32.eqz(call $value_compare_i32(local.get $a)(local.get $b))))
  (call $assert (i32.eq(call $value_hash_u32(local.get $a))(call $value_hash_u32(local.get $b))))
  (call $assert (i32.eq(call $value_compare_i32(call $int_neg(local.get $a))(local.get $b))(i32.const -1)))
  (local.set $a {{STRING_A}})
  (local.set $b {{STRING_A_AGAIN}})
  (call $assert (call $value_eq(local.get $a)(local.get $b)))
  (call $assert (i32.eq(call $value_compare_i32(local.get $a){{STRING_B}})(i32.const -1)))
  (call $assert (i32.eq(call $value_hash_u32(local.get $a))(call $value_hash_u32(local.get $b))))
  (local.set $a {{COLLISION_A}})
  (local.set $b {{COLLISION_B}})
  (call $assert (i32.eqz(call $value_eq(local.get $a)(local.get $b))))
  (call $assert (i32.eq(call $value_hash_u32(local.get $a))(i32.const 528531168)))
  (call $assert (i32.eq(call $value_hash_u32(local.get $a))(call $value_hash_u32(local.get $b))))
  (call $assert (i32.ne(call $value_compare_i32(local.get $a)(local.get $b))(i32.const 0)))
  (local.set $a (call $record(i32.const 32)(i64.const 40)(array.new_fixed $values 2(ref.i31(i32.const 4))(call $list_nil))))
  (local.set $b (call $record(i32.const 32)(i64.const 40)(array.new_fixed $values 2(ref.i31(i32.const 4))(call $list_nil))))
  (local.set $c (call $record(i32.const 32)(i64.const 41)(array.new_fixed $values 2(ref.i31(i32.const 4))(call $list_nil))))
  (call $assert (call $value_eq(local.get $a)(local.get $b)))
  (call $assert (i32.eqz(call $value_eq(local.get $a)(local.get $c))))
  (call $assert (i32.eq(call $value_compare_i32(local.get $a)(local.get $c))(i32.const -1)))
  (call $assert (i32.eq(call $value_hash_u32(local.get $a))(call $value_hash_u32(local.get $b))))
  (local.set $a (struct.new $obj(i32.const 9)(i32.const 3)(i64.const 9)(f64.const 0)
    (array.new_fixed $bytes 2(i32.const 21)(i32.const 64))(ref.null $values)(ref.null $limbs)(ref.null $call)))
  (local.set $b (struct.new $obj(i32.const 9)(i32.const 0)(i64.const 9)(f64.const 0)
    (array.new_fixed $bytes 2(i32.const 170)(i32.const 0))(ref.null $values)(ref.null $limbs)(ref.null $call)))
  (call $assert (call $value_eq(local.get $a)(local.get $b)))
  (call $assert (i32.eqz(call $value_compare_i32(local.get $a)(local.get $b))))
  (call $assert (i32.eq(call $value_hash_u32(local.get $a))(call $value_hash_u32(local.get $b))))
  (local.set $a (struct.new $hostref(ref.null extern) (i32.const 41) (i64.const 7)))
  (local.set $b (struct.new $hostref(ref.null extern) (i32.const 41) (i64.const 7)))
  (local.set $c (struct.new $hostref(ref.null extern) (i32.const 41) (i64.const 8)))
  (call $assert (call $value_eq(local.get $a)(local.get $b)))
  (call $assert (i32.eqz(call $value_eq(local.get $a)(local.get $c))))
  (call $assert (i32.eq(call $value_compare_i32(local.get $a)(local.get $c))(i32.const -1)))
  (call $assert (i32.eq(call $value_hash_u32(local.get $a))(call $value_hash_u32(local.get $b))))
  ;; Spec 4.1 and 11.4: identical foreign ids in distinct types differ.
  (local.set $c (struct.new $hostref(ref.null extern) (i32.const 42) (i64.const 7)))
  (call $assert (i32.eqz(call $value_eq(local.get $a)(local.get $c))))
  (call $assert (i32.eq(call $value_compare_i32(local.get $a)(local.get $c))(i32.const -1)))
  (call $assert (i32.eq(call $value_compare_i32(local.get $c)(local.get $a))(i32.const 1)))
  (local.set $a (call $abi_ref_in (ref.null extern) (i32.const 41)))
  (local.set $b (call $abi_ref_in (ref.null extern) (i32.const 41)))
  (local.set $c (call $abi_ref_in (ref.null extern) (i32.const 42)))
  (call $assert (call $value_eq(local.get $a)(local.get $b)))
  (call $assert (i32.eqz(call $value_eq(local.get $a)(local.get $c))))
  (call $assert (i32.eqz(call $value_compare_i32(local.get $a)(local.get $b))))
  (call $assert (i32.eq(call $value_hash_u32(local.get $a))(call $value_hash_u32(local.get $b))))
  (i32.const 1))
(func (export "deep_lists") (result i32)
  (local $a eqref)(local $b eqref)(local $i i32)
  (local.set $a(call $list_nil))(local.set $b(call $list_nil))
  (loop $build
    (local.set $a(call $list_cons(ref.i31(local.get $i))(local.get $a)))
    (local.set $b(call $list_cons(ref.i31(local.get $i))(local.get $b)))
    (local.set $i(i32.add(local.get $i)(i32.const 1)))
    (br_if $build(i32.lt_u(local.get $i)(i32.const 100000))))
  (call $assert(call $value_eq(local.get $a)(local.get $b)))
  (call $assert(i32.eqz(call $value_compare_i32(local.get $a)(local.get $b))))
  (call $assert(i32.eq(call $value_hash_u32(local.get $a))(call $value_hash_u32(local.get $b))))
  (call $assert(i64.eq(call $int_to_i64(call $list_length(local.get $a)))(i64.const 100000)))
  (call $assert(call $value_eq(local.get $a)(call $list_reverse(call $list_reverse(local.get $b)))))
  (i32.const 1))
(func (export "deep_records") (result i32)
  (local $a eqref)(local $b eqref)(local $i i32)
  (loop $build
    (local.set $a(call $record(i32.const 32)(i64.const 44)
      (array.new_fixed $values 2(ref.i31(local.get $i))(local.get $a))))
    (local.set $b(call $record(i32.const 32)(i64.const 44)
      (array.new_fixed $values 2(ref.i31(local.get $i))(local.get $b))))
    (local.set $i(i32.add(local.get $i)(i32.const 1)))
    (br_if $build(i32.lt_u(local.get $i)(i32.const 100000))))
  (call $assert(call $value_eq(local.get $a)(local.get $b)))
  (call $assert(i32.eqz(call $value_compare_i32(local.get $a)(local.get $b))))
  (call $assert(i32.eq(call $value_hash_u32(local.get $a))(call $value_hash_u32(local.get $b))))
  (i32.const 1))
;; A recursive child in the last field can accidentally hide host-stack
;; recursion through prefix fields. Exercise both non-final positions explicitly.
(func $test_record_chain_values(param $depth i32)(param $position i32)
  (param $base eqref)(param $sibling eqref)(result eqref)
  (local $i i32)(local $fields(ref $values))
  (block $done(loop $build
    (br_if $done(i32.ge_u(local.get $i)(local.get $depth)))
    (local.set $fields
      (if(result(ref $values))(i32.eqz(local.get $position))
        (then(array.new_fixed $values 2(local.get $base)(local.get $sibling)))
        (else(array.new_fixed $values 3(ref.i31(i32.const 0))(local.get $base)(local.get $sibling)))))
    (local.set $base(call $record(i32.const 33)(i64.const 57)(local.get $fields)))
    (local.set $i(i32.add(local.get $i)(i32.const 1)))
    (br $build)))
  (local.get $base))
(func $test_record_chain(param $depth i32)(param $position i32)
  (param $base eqref)(param $sibling i32)(result eqref)
  (call $test_record_chain_values(local.get $depth)(local.get $position)(local.get $base)(ref.i31(local.get $sibling))))
(func $test_equal_and_hash(param $a eqref)(param $b eqref)
  (call $assert(call $value_eq(local.get $a)(local.get $b)))
  (call $assert(i32.eqz(call $value_compare_i32(local.get $a)(local.get $b))))
  (call $assert(i32.eqz(call $value_compare_i32(local.get $b)(local.get $a))))
  (call $assert(i32.eq(call $value_hash_u32(local.get $a))(call $value_hash_u32(local.get $b)))))
(func $test_less(param $a eqref)(param $b eqref)
  (call $assert(i32.eqz(call $value_eq(local.get $a)(local.get $b))))
  (call $assert(i32.eq(call $value_compare_i32(local.get $a)(local.get $b))(i32.const -1)))
  (call $assert(i32.eq(call $value_compare_i32(local.get $b)(local.get $a))(i32.const 1))))
(func(export "deep_records_first_field")(result i32)
  (local $a eqref)(local $b eqref)
  (local.set $a(call $test_record_chain(i32.const 100000)(i32.const 0)(ref.null eq)(i32.const 7)))
  (local.set $b(call $test_record_chain(i32.const 100000)(i32.const 0)(ref.null eq)(i32.const 7)))
  (call $test_equal_and_hash(local.get $a)(local.get $b))
  (i32.const 1))
(func(export "deep_records_middle_field")(result i32)
  (local $a eqref)(local $b eqref)
  (local.set $a(call $test_record_chain(i32.const 100000)(i32.const 1)(ref.null eq)(i32.const 7)))
  (local.set $b(call $test_record_chain(i32.const 100000)(i32.const 1)(ref.null eq)(i32.const 7)))
  (call $test_equal_and_hash(local.get $a)(local.get $b))
  (i32.const 1))
(func(export "deep_records_unequal_base")(result i32)
  (local $a eqref)(local $b eqref)(local $position i32)
  (loop $positions
    ;; Every delayed sibling would order a>b, while the deepest base orders a<b.
    (local.set $a(call $test_record_chain(i32.const 100000)(local.get $position)(ref.i31(i32.const -1))(i32.const 100)))
    (local.set $b(call $test_record_chain(i32.const 100000)(local.get $position)(ref.i31(i32.const 0))(i32.const -100)))
    (call $test_less(local.get $a)(local.get $b))
    (local.set $a(ref.null eq))(local.set $b(ref.null eq))
    (local.set $position(i32.add(local.get $position)(i32.const 1)))
    (br_if $positions(i32.lt_u(local.get $position)(i32.const 2))))
  (i32.const 1))
(func(export "deep_records_delayed_siblings")(result i32)
  (local $a eqref)(local $b eqref)(local $position i32)
  (loop $positions
    ;; Equal deepest children must resume their own pending siblings before
    ;; any ancestor's siblings: inner 1<2 dominates outer 100>-100.
    (local.set $a(call $test_record_chain(i32.const 1)(local.get $position)(ref.null eq)(i32.const 1)))
    (local.set $b(call $test_record_chain(i32.const 1)(local.get $position)(ref.null eq)(i32.const 2)))
    (local.set $a(call $test_record_chain(i32.const 100000)(local.get $position)(local.get $a)(i32.const 100)))
    (local.set $b(call $test_record_chain(i32.const 100000)(local.get $position)(local.get $b)(i32.const -100)))
    (call $test_less(local.get $a)(local.get $b))
    (local.set $a(ref.null eq))(local.set $b(ref.null eq))
    (local.set $position(i32.add(local.get $position)(i32.const 1)))
    (br_if $positions(i32.lt_u(local.get $position)(i32.const 2))))
  (i32.const 1))
(func $test_nested_heads(param $depth i32)(param $base eqref)(result eqref)
  (local $i i32)
  (block $done(loop $build
    (br_if $done(i32.ge_u(local.get $i)(local.get $depth)))
    (local.set $base(call $list_cons(local.get $base)(call $list_nil)))
    (local.set $i(i32.add(local.get $i)(i32.const 1)))
    (br $build)))
  (local.get $base))
(func(export "deep_list_heads")(result i32)
  (local $a eqref)(local $b eqref)(local $c eqref)
  (local.set $a(call $test_nested_heads(i32.const 100000)(ref.i31(i32.const 1))))
  (local.set $b(call $test_nested_heads(i32.const 100000)(ref.i31(i32.const 1))))
  (call $test_equal_and_hash(local.get $a)(local.get $b))
  (local.set $b(ref.null eq))
  (local.set $c(call $test_nested_heads(i32.const 100000)(ref.i31(i32.const 2))))
  (call $test_less(local.get $a)(local.get $c))
  (i32.const 1))
(func(export "deep_records_boxed_siblings")(result i32)
  (local $a eqref)(local $b eqref)(local $position i32)(local $left eqref)(local $right eqref)
  (loop $positions
    ;; Separately allocated but equal primitive siblings exercise the equality
    ;; continuation after both the f64 and string comparison branches.
    (local.set $left(if(result eqref)(i32.eqz(local.get $position))
      (then(call $float_new(f64.const 7.5)))
      (else(call $string_from_bytes(array.new_fixed $bytes 1(i32.const 97))))))
    (local.set $right(if(result eqref)(i32.eqz(local.get $position))
      (then(call $float_new(f64.const 7.5)))
      (else(call $string_from_bytes(array.new_fixed $bytes 1(i32.const 97))))))
    (local.set $a(call $test_record_chain_values(i32.const 100000)(local.get $position)(ref.null eq)(local.get $left)))
    (local.set $b(call $test_record_chain_values(i32.const 100000)(local.get $position)(ref.null eq)(local.get $right)))
    (call $test_equal_and_hash(local.get $a)(local.get $b))
    (local.set $a(ref.null eq))(local.set $b(ref.null eq))
    ;; Deepest equal floats must not return from the whole comparison: the
    ;; following a<b string pair precedes all ancestor z>a string siblings.
    (local.set $a(call $tuple(array.new_fixed $values 2
      (call $float_new(f64.const 1.5))(call $string_from_bytes(array.new_fixed $bytes 1(i32.const 97))))))
    (local.set $b(call $tuple(array.new_fixed $values 2
      (call $float_new(f64.const 1.5))(call $string_from_bytes(array.new_fixed $bytes 1(i32.const 98))))))
    (local.set $left(call $string_from_bytes(array.new_fixed $bytes 1(i32.const 122))))
    (local.set $right(call $string_from_bytes(array.new_fixed $bytes 1(i32.const 97))))
    (local.set $a(call $test_record_chain_values(i32.const 100000)(local.get $position)(local.get $a)(local.get $left)))
    (local.set $b(call $test_record_chain_values(i32.const 100000)(local.get $position)(local.get $b)(local.get $right)))
    (call $test_less(local.get $a)(local.get $b))
    (local.set $a(ref.null eq))(local.set $b(ref.null eq))
    (local.set $position(i32.add(local.get $position)(i32.const 1)))
    (br_if $positions(i32.lt_u(local.get $position)(i32.const 2))))
  (i32.const 1))
(func(export "lexicographic_continuations")(result i32)
  (local $a eqref)(local $b eqref)
  ;; Common prefix equality precedes the array length continuation.
  (local.set $a(call $tuple(array.new_fixed $values 1(call $float_new(f64.const 1.5)))))
  (local.set $b(call $tuple(array.new_fixed $values 2(call $float_new(f64.const 1.5))(ref.i31(i32.const 0)))))
  (call $test_less(local.get $a)(local.get $b))
  ;; A shorter array may be greater when its first field is greater.
  (local.set $a(call $tuple(array.new_fixed $values 1(ref.i31(i32.const 2)))))
  (local.set $b(call $tuple(array.new_fixed $values 2(ref.i31(i32.const 1))(ref.i31(i32.const 999)))))
  (call $test_less(local.get $b)(local.get $a))
  ;; The first differing field precedes an oppositely ordered later field.
  (local.set $a(call $tuple(array.new_fixed $values 3(ref.i31(i32.const 0))(call $float_new(f64.const 5))(ref.i31(i32.const 999)))))
  (local.set $b(call $tuple(array.new_fixed $values 3(ref.i31(i32.const 0))(call $float_new(f64.const 6))(ref.i31(i32.const -999)))))
  (call $test_less(local.get $a)(local.get $b))
  ;; Empty arrays compare equal but precede nonempty arrays with a Nil field.
  (local.set $a(call $tuple(array.new_fixed $values 0)))
  (local.set $b(call $tuple(array.new_fixed $values 0)))
  (call $test_equal_and_hash(local.get $a)(local.get $b))
  (call $test_less(local.get $a)(call $tuple(array.new_fixed $values 1(ref.null eq))))
  (i32.const 1))
(func (export "reject_nan") (drop(call $float_new(f64.const nan))))
(func (export "reject_infinity") (drop(call $float_new(f64.const inf))))
"""

ALLOCATOR_TESTS = """
(func $assert(param $v i32)(if(i32.eqz(local.get $v))(then unreachable)))
(func (export "allocator_reuse_and_growth") (result i32)
  (local $p i32)
  (local.set $p(call $alloc(i32.const 0)))
  (call $assert(i32.eqz(i32.and(local.get $p)(i32.const 7))))
  (call $free(local.get $p)(i32.const 0))
  (call $assert(i32.eq(local.get $p)(call $alloc(i32.const 8))))
  (global.set $heap(i32.const 65520))
  (local.set $p(call $alloc(i32.const 32)))
  (call $assert(i32.eq(local.get $p)(i32.const 65528)))
  (call $assert(i32.eq(memory.size)(i32.const 2)))
  (i32.store offset=28(local.get $p)(i32.const 123))
  (call $assert(i32.eq(i32.load offset=28(local.get $p))(i32.const 123)))
  (i32.const 1))
(func (export "allocator_full_memory32") (result i32)
  (local $p i32)
  ;; Wasmtime reserves address space sparsely; only these boundary bytes are
  ;; touched, so the 4 GiB address-boundary regression needs little physical RAM.
  (call $assert(i32.eq(memory.grow(i32.const 65535))(i32.const 1)))
  (global.set $heap(i32.const -32))
  (local.set $p(call $alloc(i32.const 8)))
  (call $assert(i32.eq(local.get $p)(i32.const -24)))
  (call $assert(i32.eq(memory.size)(i32.const 65536)))
  (i32.store(local.get $p)(i32.const 321))
  (call $assert(i32.eq(i32.load(local.get $p))(i32.const 321)))
  (call $free(local.get $p)(i32.const 8))
  (call $assert(i32.eq(local.get $p)(call $alloc(i32.const 8))))
  (i32.const 1))
(func (export "allocator_reject_address_wrap")
  (global.set $heap(i32.const -16))
  (drop(call $alloc(i32.const 8))))
(func (export "allocator_reject_oversize")
  (drop(call $alloc(i32.const -1))))
"""


def execute(args, directory, name, module, successful, trapping):
    source = pathlib.Path(directory) / f"{name}.wat"
    binary = pathlib.Path(directory) / f"{name}.wasm"
    source.write_text(module)
    subprocess.run([args.wasm_tools, "parse", str(source), "-o", str(binary)], check=True)
    subprocess.run([args.wasm_tools, "validate", str(binary)], check=True)
    for export in successful:
        result = subprocess.run([args.wasmtime, "run", "--invoke", export, str(binary)],
                                check=True, capture_output=True, text=True)
        assert result.stdout.strip() == "1", (export, result.stdout, result.stderr)
        print(f"PASS {export}")
    for export in trapping:
        result = subprocess.run([args.wasmtime, "run", "--invoke", export, str(binary)],
                                capture_output=True, text=True)
        assert result.returncode != 0 and "unreachable" in result.stderr, (export, result)
        print(f"PASS {export}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--wasm-tools", default="wasm-tools")
    parser.add_argument("--wasmtime", default="wasmtime")
    parser.add_argument("--types", required=True)
    parser.add_argument("--runtime", required=True)
    args = parser.parse_args()
    tests = TESTS
    for key, value in {
        "STRING_A": "你好😀",
        "STRING_A_AGAIN": "你好😀",
        "STRING_B": "你好😁",
        "COLLISION_A": "ilang-hamt-63983",
        "COLLISION_B": "ilang-hamt-429330",
    }.items():
        tests = tests.replace("{{" + key + "}}", string(value))
    tests += runtime_records.tests(string)
    wat = "(module\n" + "\n".join([
        pathlib.Path(args.types).read_text(), PRELUDE,
        pathlib.Path(args.runtime).read_text(), tests,
    ]) + ")"
    with tempfile.TemporaryDirectory(prefix="ilang-values-") as directory:
        execute(args, directory, "values", wat,
                ["correctness", "deep_lists", "deep_records",
                 "deep_records_first_field", "deep_records_middle_field",
                 "deep_records_unequal_base", "deep_records_delayed_siblings",
                 "deep_list_heads", "deep_records_boxed_siblings",
                 "lexicographic_continuations", "structural_records",
                 "record_identity_labels", "deep_structural_records"],
                ["reject_nan", "reject_infinity", "record_missing_traps"])
        allocator = "(module\n" + raw(ROOT / "mod.rs", "MEMORY") + ALLOCATOR_TESTS + ")"
        execute(args, directory, "allocator", allocator,
                ["allocator_reuse_and_growth", "allocator_full_memory32"],
                ["allocator_reject_address_wrap", "allocator_reject_oversize"])


if __name__ == "__main__":
    main()
