//! Value inspection implemented inside the generated WebAssembly module.
//!
//! Objects are never exposed to a host formatter. In particular, arbitrary precision
//! integers and bit-array slices retain their exact value on every supported host.

mod float_format;

/// Runtime definitions, to be inserted after the common value types.
pub fn wat() -> String {
    let source = r#"
  ;; The text form is written into one growable byte buffer, `$ib` with the
  ;; length `$il`, so that inspecting is linear in the size of the text. Only
  ;; `$inspect` uses it, and it runs no i code.
  (global $ib (mut (ref null $bytes)) (ref.null $bytes))
  (global $il (mut i32) (i32.const 0))

  ;; Room for `n` more bytes.
  (func $ib_reserve (param $n i32)
    (local $old (ref $bytes)) (local $need i64) (local $cap i64) (local $new (ref $bytes))
    (local.set $old (ref.as_non_null (global.get $ib)))
    (local.set $need (i64.add (i64.extend_i32_u (global.get $il)) (i64.extend_i32_u (local.get $n))))
    (if (i64.le_u (local.get $need) (i64.extend_i32_u (array.len (local.get $old)))) (then (return)))
    (if (i64.gt_u (local.get $need) (i64.const 2147483647)) (then unreachable))
    (local.set $cap (i64.shl (i64.extend_i32_u (array.len (local.get $old))) (i64.const 1)))
    (if (i64.gt_u (local.get $cap) (i64.const 2147483647)) (then (local.set $cap (i64.const 2147483647))))
    (if (i64.lt_u (local.get $cap) (local.get $need)) (then (local.set $cap (local.get $need))))
    (local.set $new (array.new_default $bytes (i32.wrap_i64 (local.get $cap))))
    (array.copy $bytes $bytes (local.get $new) (i32.const 0) (local.get $old) (i32.const 0) (global.get $il))
    (global.set $ib (local.get $new)))

  (func $ib_byte (param $b i32)
    (call $ib_reserve (i32.const 1))
    (array.set $bytes (ref.as_non_null (global.get $ib)) (global.get $il) (local.get $b))
    (global.set $il (i32.add (global.get $il) (i32.const 1))))

  (func $ib_comma
    (call $ib_reserve (i32.const 2))
    (array.set $bytes (ref.as_non_null (global.get $ib)) (global.get $il) (i32.const 44))
    (array.set $bytes (ref.as_non_null (global.get $ib)) (i32.add (global.get $il) (i32.const 1)) (i32.const 32))
    (global.set $il (i32.add (global.get $il) (i32.const 2))))

  ;; The bytes of a String value.
  (func $ib_string (param $s eqref)
    (local $b (ref $bytes)) (local $n i32)
    (local.set $b (call $string_bytes (local.get $s)))
    (local.set $n (array.len (local.get $b)))
    (call $ib_reserve (local.get $n))
    (array.copy $bytes $bytes (ref.as_non_null (global.get $ib)) (global.get $il)
      (local.get $b) (i32.const 0) (local.get $n))
    (global.set $il (i32.add (global.get $il) (local.get $n))))

  ;; The decimal digits of a small Int, with `-` when it is negative.
  (func $ib_small_int (param $v i32)
    (local $u i64) (local $buf (ref $bytes)) (local $start i32) (local $i i32) (local $j i32) (local $t i32)
    (call $ib_reserve (i32.const 12))
    (local.set $buf (ref.as_non_null (global.get $ib)))
    (local.set $i (global.get $il))
    (local.set $u (i64.extend_i32_s (local.get $v)))
    (if (i64.lt_s (local.get $u) (i64.const 0))
      (then
        (array.set $bytes (local.get $buf) (local.get $i) (i32.const 45))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (local.set $u (i64.sub (i64.const 0) (local.get $u)))))
    (local.set $start (local.get $i))
    (loop $digits
      (array.set $bytes (local.get $buf) (local.get $i)
        (i32.add (i32.const 48) (i32.wrap_i64 (i64.rem_u (local.get $u) (i64.const 10)))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (local.set $u (i64.div_u (local.get $u) (i64.const 10)))
      (br_if $digits (i64.ne (local.get $u) (i64.const 0))))
    (global.set $il (local.get $i))
    (local.set $j (i32.sub (local.get $i) (i32.const 1)))
    (block $reversed (loop $reverse
      (br_if $reversed (i32.ge_s (local.get $start) (local.get $j)))
      (local.set $t (array.get_u $bytes (local.get $buf) (local.get $start)))
      (array.set $bytes (local.get $buf) (local.get $start) (array.get_u $bytes (local.get $buf) (local.get $j)))
      (array.set $bytes (local.get $buf) (local.get $j) (local.get $t))
      (local.set $start (i32.add (local.get $start) (i32.const 1)))
      (local.set $j (i32.sub (local.get $j) (i32.const 1)))
      (br $reverse))))

  ;; Quoting works on UTF-8 bytes. C0, DEL and the UTF-8 encoded C1 controls use
  ;; i escapes; all other valid UTF-8 is copied unchanged.
  (func $ib_quote (param $s eqref)
    (local $bytes (ref $bytes)) (local $n i32) (local $buf (ref $bytes))
    (local $out i32) (local $i i32) (local $b i32)
    (local $escape i32) (local $control i32)
    (local.set $bytes (call $string_bytes (local.get $s)))
    (local.set $n (array.len (local.get $bytes)))
    (if (i32.gt_u (local.get $n) (i32.const 268435455)) (then unreachable))
    (call $ib_reserve (i32.add (i32.mul (local.get $n) (i32.const 8)) (i32.const 2)))
    (local.set $buf (ref.as_non_null (global.get $ib)))
    (local.set $out (global.get $il))
    (array.set $bytes (local.get $buf) (local.get $out) (i32.const 34))
    (local.set $out (i32.add (local.get $out) (i32.const 1)))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (local.set $b (array.get_u $bytes (local.get $bytes) (local.get $i)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (local.set $control (i32.or (i32.lt_u (local.get $b) (i32.const 32)) (i32.eq (local.get $b) (i32.const 127))))
        (local.set $escape (i32.const 0))
        (if (i32.eq (local.get $b) (i32.const 10)) (then (local.set $escape (i32.const 110))))
        (if (i32.eq (local.get $b) (i32.const 13)) (then (local.set $escape (i32.const 114))))
        (if (i32.eq (local.get $b) (i32.const 9)) (then (local.set $escape (i32.const 116))))
        (if (i32.eq (local.get $b) (i32.const 12)) (then (local.set $escape (i32.const 102))))
        (if (i32.or (i32.eq (local.get $b) (i32.const 34)) (i32.eq (local.get $b) (i32.const 92)))
          (then (local.set $escape (local.get $b))))
        (if (local.get $escape)
          (then
            (array.set $bytes (local.get $buf) (local.get $out) (i32.const 92))
            (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 1)) (local.get $escape))
            (local.set $out (i32.add (local.get $out) (i32.const 2))))
          (else
            (if (i32.and (i32.eq (local.get $b) (i32.const 194)) (i32.lt_u (local.get $i) (local.get $n)))
              (then
                (if (i32.lt_u (array.get_u $bytes (local.get $bytes) (local.get $i)) (i32.const 160))
                  (then
                    (local.set $b (array.get_u $bytes (local.get $bytes) (local.get $i)))
                    (local.set $control (i32.const 1))
                    (local.set $i (i32.add (local.get $i) (i32.const 1)))))))
            (if (local.get $control)
              (then
                (array.set $bytes (local.get $buf) (local.get $out) (i32.const 92))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 1)) (i32.const 117))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 2)) (i32.const 123))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 3)) (i32.const 48))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 4)) (i32.const 48))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 5))
                  (call $inspect_hex (i32.shr_u (local.get $b) (i32.const 4))))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 6))
                  (call $inspect_hex (i32.and (local.get $b) (i32.const 15))))
                (array.set $bytes (local.get $buf) (i32.add (local.get $out) (i32.const 7)) (i32.const 125))
                (local.set $out (i32.add (local.get $out) (i32.const 8))))
              (else
                (array.set $bytes (local.get $buf) (local.get $out) (local.get $b))
                (local.set $out (i32.add (local.get $out) (i32.const 1)))))))
        (br $next)))
    (array.set $bytes (local.get $buf) (local.get $out) (i32.const 34))
    (global.set $il (i32.add (local.get $out) (i32.const 1))))

  (func $inspect_hex (param $n i32) (result i32)
    (i32.add (local.get $n) (if (result i32) (i32.lt_u (local.get $n) (i32.const 10))
      (then (i32.const 48)) (else (i32.const 55)))))

  (func $inspect_bits (param $value eqref) (result eqref)
    (local $obj (ref $obj)) (local $bytes (ref $bytes)) (local $length i64)
    (local $offset i64) (local $i i64) (local $pos i64) (local $j i32) (local $v i32) (local $out eqref)
    (local.set $obj (ref.cast (ref $obj) (local.get $value)))
    (local.set $bytes (ref.as_non_null (struct.get $obj 4 (local.get $obj))))
    (local.set $length (struct.get $obj 2 (local.get $obj)))
    (local.set $offset (i64.extend_i32_u (struct.get $obj 1 (local.get $obj))))
    (local.set $out (call $list_cons {{bits_open}} (call $list_nil)))
    (block $done
      (loop $next
        (br_if $done (i64.ge_u (local.get $i) (local.get $length)))
        (if (i64.ne (local.get $i) (i64.const 0)) (then (local.set $out (call $list_cons {{comma}} (local.get $out)))))
        (local.set $j (i32.const 0))
        (local.set $v (i32.const 0))
        (block $group_done
          (loop $group
            (br_if $group_done (i32.eq (local.get $j) (i32.const 8)))
            (br_if $group_done (i64.ge_u (local.get $i) (local.get $length)))
            (local.set $pos (i64.add (local.get $offset) (local.get $i)))
            (local.set $v (i32.or (i32.shl (local.get $v) (i32.const 1))
              (i32.and
                (i32.shr_u (array.get_u $bytes (local.get $bytes) (i32.wrap_i64 (i64.shr_u (local.get $pos) (i64.const 3))))
                  (i32.sub (i32.const 7) (i32.wrap_i64 (i64.and (local.get $pos) (i64.const 7))))) (i32.const 1))))
            (local.set $i (i64.add (local.get $i) (i64.const 1)))
            (local.set $j (i32.add (local.get $j) (i32.const 1)))
            (br $group)))
        (local.set $out (call $list_cons (call $int_to_string (call $int_from_i64 (i64.extend_i32_u (local.get $v)))) (local.get $out)))
        ;; The last k < 8 bits are written `value:uk` (spec §11.8).
        (if (i32.lt_u (local.get $j) (i32.const 8))
          (then
            (local.set $out (call $list_cons {{bits_size}} (local.get $out)))
            (local.set $out (call $list_cons (call $int_to_string (call $int_from_i64 (i64.extend_i32_u (local.get $j)))) (local.get $out)))))
        (br $next)))
    (call $string_concat (call $list_reverse (call $list_cons {{bits_close}} (local.get $out)))))

  ;; The shapes with labels (syntax-3.md §6.4), by `id - 2^32`: each is the
  ;; number of positions (an i31) and then the labels, in canonical order.
  (global $shape_table (mut (ref null $values)) (ref.null $values))
  (func $shapes_install (param $table (ref $values))
    (global.set $shape_table (local.get $table)))

  ;; The parameter data of a function of the shape `shape`, `(a, b, port = c)`,
  ;; put in front of the reversed list `out`. Key i is named by the letter
  ;; a + (i mod 26), then i div 26 when i >= 26; a label key is written
  ;; `label = name`.
  (func $inspect_params (param $shape i64) (param $out eqref) (result eqref)
    (local $n i32) (local $total i32) (local $i i32) (local $entry (ref null $values))
    (local.set $n (i32.wrap_i64 (local.get $shape)))
    (local.set $total (local.get $n))
    (if (i64.ge_u (local.get $shape) (i64.const 4294967296))
      (then
        (local.set $entry (ref.cast (ref $values) (array.get $values
          (ref.as_non_null (global.get $shape_table))
          (i32.wrap_i64 (i64.sub (local.get $shape) (i64.const 4294967296))))))
        (local.set $n (i31.get_u (ref.cast (ref i31)
          (array.get $values (ref.as_non_null (local.get $entry)) (i32.const 0)))))
        (local.set $total (i32.sub (i32.add (local.get $n)
          (array.len (ref.as_non_null (local.get $entry)))) (i32.const 1)))))
    (local.set $out (call $list_cons {{fn_paren}} (local.get $out)))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $total)))
        (if (local.get $i) (then (local.set $out (call $list_cons {{comma}} (local.get $out)))))
        (if (i32.ge_u (local.get $i) (local.get $n))
          (then
            (local.set $out (call $list_cons (array.get $values (ref.as_non_null (local.get $entry))
              (i32.add (i32.sub (local.get $i) (local.get $n)) (i32.const 1))) (local.get $out)))
            (local.set $out (call $list_cons {{label_eq}} (local.get $out)))))
        (local.set $out (call $list_cons
          (call $string_from_bytes (array.new_fixed $bytes 1
            (i32.add (i32.const 97) (i32.rem_u (local.get $i) (i32.const 26))))) (local.get $out)))
        (if (i32.ge_u (local.get $i) (i32.const 26))
          (then (local.set $out (call $list_cons
            (call $int_to_string (call $int_from_i64
              (i64.extend_i32_u (i32.div_u (local.get $i) (i32.const 26))))) (local.get $out)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (call $list_cons {{fn_close}} (local.get $out)))

  ;; A function: `//fn(a, b) { ... }`; a multi-shape value (field 2 is -1)
  ;; shows each group in order, `//fn(a) { ... } | (a, b) { ... }`.
  (func $inspect_function (param $obj (ref $obj)) (result eqref)
    (local $out eqref) (local $groups (ref null $values)) (local $i i32)
    (local.set $out (call $list_cons {{fn_open}} (call $list_nil)))
    (if (i64.ne (struct.get $obj 2 (local.get $obj)) (i64.const -1))
      (then (return (call $string_concat (call $list_reverse
        (call $inspect_params (struct.get $obj 2 (local.get $obj)) (local.get $out)))))))
    (local.set $groups (struct.get $obj 5 (local.get $obj)))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (array.len (ref.as_non_null (local.get $groups)))))
        (if (local.get $i) (then (local.set $out (call $list_cons {{fn_bar}} (local.get $out)))))
        (local.set $out (call $inspect_params
          (struct.get $obj 2 (ref.cast (ref $obj)
            (array.get $values (ref.as_non_null (local.get $groups)) (local.get $i))))
          (local.get $out)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (call $string_concat (call $list_reverse (local.get $out))))

  ;; Constructor names: the runtime's own below FIRST_CONSTRUCTOR (16), then the
  ;; names that the code generator installs, in order.
  (global $constructor_names (mut (ref null $values)) (ref.null $values))
  (func $constructors_install (param $names (ref $values))
    (global.set $constructor_names (local.get $names)))
  (func $constructor_name (param $ctor i32) (result eqref)
    (local $names (ref null $values)) (local $index i32)
    (if (i32.eqz (local.get $ctor)) (then (return {{ok}})))
    (if (i32.eq (local.get $ctor) (i32.const 1)) (then (return {{error}})))
    (if (i32.eq (local.get $ctor) (i32.const 2)) (then (return {{assertion_failure}})))
    (if (i32.eq (local.get $ctor) (i32.const 3)) (then (return {{assert}})))
    (local.set $names (global.get $constructor_names))
    (local.set $index (i32.sub (local.get $ctor) (i32.const 16)))
    (if (i32.or (ref.is_null (local.get $names)) (i32.lt_s (local.get $index) (i32.const 0)))
      (then (return {{unknown}})))
    (if (i32.ge_u (local.get $index) (array.len (local.get $names)))
      (then (return {{unknown}})))
    (array.get $values (local.get $names) (local.get $index)))

  ;; Aggregate traversal uses an explicit stack of tasks rather than Wasm
  ;; calls, so deep values do not use the Wasm stack. A task is a value and a
  ;; number, `kind | index << 2`, in two growable arrays.
  (global $it_values (mut (ref null $values)) (ref.null $values))
  (global $it_meta (mut (ref null $frames)) (ref.null $frames))
  (global $it_sp (mut i32) (i32.const 0))

  (func $inspect_push (param $kind i32) (param $value eqref) (param $index i32)
    (local $sp i32) (local $n i32) (local $values (ref $values)) (local $meta (ref $frames))
    (local.set $sp (global.get $it_sp))
    (local.set $n (array.len (ref.as_non_null (global.get $it_values))))
    (if (i32.ge_u (local.get $sp) (local.get $n))
      (then
        (if (i32.ge_u (local.get $n) (i32.const 268435456)) (then unreachable))
        (local.set $values (array.new_default $values (i32.shl (local.get $n) (i32.const 1))))
        (local.set $meta (array.new_default $frames (i32.shl (local.get $n) (i32.const 1))))
        (array.copy $values $values (local.get $values) (i32.const 0)
          (ref.as_non_null (global.get $it_values)) (i32.const 0) (local.get $n))
        (array.copy $frames $frames (local.get $meta) (i32.const 0)
          (ref.as_non_null (global.get $it_meta)) (i32.const 0) (local.get $n))
        (global.set $it_values (local.get $values))
        (global.set $it_meta (local.get $meta))))
    (array.set $values (ref.as_non_null (global.get $it_values)) (local.get $sp) (local.get $value))
    (array.set $frames (ref.as_non_null (global.get $it_meta)) (local.get $sp)
      (i32.or (local.get $kind) (i32.shl (local.get $index) (i32.const 2))))
    (global.set $it_sp (i32.add (local.get $sp) (i32.const 1))))

  ;; Values with no parts that are inspected; not Ints and Strings, which
  ;; `$ib_atom` writes itself.
  (func $inspect_atom (param $value eqref) (param $tag i32) (result eqref)
    (local $obj (ref $obj))
    (if (i32.eqz (local.get $tag)) (then (return {{nil}})))
    (if (i32.eq (local.get $tag) (i32.const 8)) (then (return_call $int_to_string (local.get $value))))
    (if (i32.eq (local.get $tag) (i32.const 1))
      (then (return (if (result eqref) (call $bool_get (local.get $value)) (then {{true}}) (else {{false}})))))
    (if (i32.eq (local.get $tag) (i32.const 2)) (then (return_call $float_to_string (local.get $value))))
    (if (i32.eq (local.get $tag) (i32.const 9)) (then (return_call $inspect_bits (local.get $value))))
    (if (i32.eq (local.get $tag) (i32.const 10)) (then (return {{opaque}})))
    (if (i32.eq (local.get $tag) (i32.const 11)) (then (return {{boundary}})))
    (if (i32.eq (local.get $tag) (i32.const 12)) (then (return {{continuation}})))
    (if (i32.eq (local.get $tag) (i32.const 15)) (then (return {{key}})))
    (if (i32.eq (local.get $tag) (i32.const 16)) (then (return {{keyed}})))
    (if (i32.eq (local.get $tag) (i32.const 17)) (then (return {{request}})))
    (if (i32.eq (local.get $tag) (i32.const 18)) (then (return {{ticket}})))
    (local.set $obj (ref.cast (ref $obj) (local.get $value)))
    (if (i32.eq (local.get $tag) (i32.const 7))
      (then (return_call $inspect_function (local.get $obj))))
    (if (i32.eq (local.get $tag) (i32.const 13))
      (then (return (call $string_append (call $string_append {{codepoint_open}}
        (call $string_from_codepoints (call $list_cons (local.get $value) (call $list_nil)))) {{paren_close}}))))
    {{unknown}})

  (func $ib_atom (param $value eqref) (param $tag i32)
    (if (ref.test (ref i31) (local.get $value))
      (then (return_call $ib_small_int (i31.get_s (ref.cast (ref i31) (local.get $value))))))
    (if (i32.eq (local.get $tag) (i32.const 3)) (then (return_call $ib_quote (local.get $value))))
    (return_call $ib_string (call $inspect_atom (local.get $value) (local.get $tag))))

  (func $inspect (param $value eqref) (result eqref)
    (local $obj (ref $obj)) (local $fields (ref null $values)) (local $out (ref $bytes))
    (local $kind i32) (local $index i32) (local $tag i32) (local $sp i32) (local $meta i32)
    (global.set $ib (array.new_default $bytes (i32.const 64)))
    (global.set $il (i32.const 0))
    (global.set $it_values (array.new_default $values (i32.const 64)))
    (global.set $it_meta (array.new_default $frames (i32.const 64)))
    (global.set $it_sp (i32.const 0))
    (call $inspect_push (i32.const 0) (local.get $value) (i32.const 0))
    (block $done (loop $next
      (local.set $sp (global.get $it_sp))
      (br_if $done (i32.eqz (local.get $sp)))
      (local.set $sp (i32.sub (local.get $sp) (i32.const 1)))
      (global.set $it_sp (local.get $sp))
      (local.set $value (array.get $values (ref.as_non_null (global.get $it_values)) (local.get $sp)))
      (array.set $values (ref.as_non_null (global.get $it_values)) (local.get $sp) (ref.null eq))
      (local.set $meta (array.get $frames (ref.as_non_null (global.get $it_meta)) (local.get $sp)))
      (local.set $kind (i32.and (local.get $meta) (i32.const 3)))
      (local.set $index (i32.shr_u (local.get $meta) (i32.const 2)))

      ;; 1: the byte `index`.
      (if (i32.eq (local.get $kind) (i32.const 1))
        (then (call $ib_byte (local.get $index)) (br $next)))

      ;; 2: the parts of a tuple or a custom value, from `index` on.
      (if (i32.eq (local.get $kind) (i32.const 2))
        (then
          (if (i32.eq (call $value_tag (local.get $value)) (i32.const 14))
            (then (call $inspect_record_field (local.get $value) (local.get $index)) (br $next)))
          (local.set $fields (struct.get $obj 5 (ref.cast (ref $obj) (local.get $value))))
          (if (ref.is_null (local.get $fields)) (then (br $next)))
          (br_if $next (i32.ge_u (local.get $index) (array.len (ref.as_non_null (local.get $fields)))))
          (if (local.get $index) (then (call $ib_comma)))
          (call $inspect_push (i32.const 2) (local.get $value) (i32.add (local.get $index) (i32.const 1)))
          (call $inspect_push (i32.const 0)
            (array.get $values (ref.as_non_null (local.get $fields)) (local.get $index)) (i32.const 0))
          (br $next)))

      ;; 3: the rest of a list; `index` is 1 after its first part.
      (if (i32.eq (local.get $kind) (i32.const 3))
        (then
          (if (call $list_is_empty (local.get $value))
            (then (call $ib_byte (i32.const 93)) (br $next)))
          (if (local.get $index) (then (call $ib_comma)))
          (call $inspect_push (i32.const 3) (call $list_tail (local.get $value)) (i32.const 1))
          (call $inspect_push (i32.const 0) (call $list_head (local.get $value)) (i32.const 0))
          (br $next)))

      ;; 0: a value.
      (local.set $tag (call $value_tag (local.get $value)))
      (if (i32.eq (local.get $tag) (i32.const 14))
        (then (call $inspect_record (local.get $value)) (br $next)))
      (if (i32.or (i32.eq (local.get $tag) (i32.const 4)) (i32.eq (local.get $tag) (i32.const 5)))
        (then
          (local.set $obj (ref.cast (ref $obj) (local.get $value)))
          (local.set $fields (struct.get $obj 5 (local.get $obj)))
          (if (i32.eq (local.get $tag) (i32.const 5))
            (then
              ;; A constructor without fields is only its name: `Nil`, `True`.
              (call $ib_string (call $constructor_name (struct.get $obj 1 (local.get $obj))))
              (if (ref.is_null (local.get $fields)) (then (br $next)))
              (br_if $next (i32.eqz (array.len (ref.as_non_null (local.get $fields)))))))
          (call $ib_byte (i32.const 40))
          (call $inspect_push (i32.const 1) (ref.null eq) (i32.const 41))
          (call $inspect_push (i32.const 2) (local.get $value) (i32.const 0))
          (br $next)))
      (if (i32.eq (local.get $tag) (i32.const 6))
        (then
          (call $ib_byte (i32.const 91))
          (call $inspect_push (i32.const 3) (local.get $value) (i32.const 0))
          (br $next)))
      (call $ib_atom (local.get $value) (local.get $tag))
      (br $next)))
    (local.set $out (array.new_default $bytes (global.get $il)))
    (array.copy $bytes $bytes (local.get $out) (i32.const 0)
      (ref.as_non_null (global.get $ib)) (i32.const 0) (global.get $il))
    (global.set $ib (ref.null $bytes))
    (global.set $it_values (ref.null $values))
    (global.set $it_meta (ref.null $frames))
    (call $string_from_bytes (local.get $out)))

  (func $value_inspect (param $value eqref) (result eqref)
    (return_call $inspect (local.get $value)))
"#
    .to_owned();

    let mut source = crate::runtime::with_literals(
        source,
        &[
            ("comma", ", "),
            ("paren_close", ")"),
            ("bits_open", "<<"),
            ("bits_close", ">>"),
            ("bits_size", ":u"),
            ("fn_open", "//fn"),
            ("fn_paren", "("),
            ("fn_bar", " | "),
            ("label_eq", " = "),
            ("fn_close", ") { ... }"),
            ("nil", "Nil"),
            ("true", "True"),
            ("false", "False"),
            ("ok", "Ok"),
            ("error", "Error"),
            ("assertion_failure", "AssertionFailure"),
            ("assert", "Assert"),
            ("opaque", "//wasm(opaque)"),
            ("boundary", "//boundary"),
            ("continuation", "//continuation"),
            ("key", "//key"),
            ("keyed", "//keyed"),
            ("request", "//request"),
            ("ticket", "//ticket"),
            ("codepoint_open", "//utfcodepoint("),
            ("unknown", "//wasm(unknown)"),
        ],
    );
    source.push_str(&float_format::wat());
    source
}

#[cfg(test)]
mod tests;
