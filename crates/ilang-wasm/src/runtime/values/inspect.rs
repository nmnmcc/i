//! Value inspection implemented inside the generated WebAssembly module.
//!
//! Objects are never exposed to a host formatter. In particular, arbitrary precision
//! integers and bit-array slices retain their exact value on every supported host.

mod float_format;

fn literal(text: &str) -> String {
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

/// Runtime definitions, to be inserted after the common value types.
pub fn wat() -> String {
    let mut source = r#"
  ;; Quoting works on UTF-8 bytes. C0, DEL and the UTF-8 encoded C1 controls use
  ;; Gleam escapes; all other valid UTF-8 is copied unchanged.
  (func $inspect_quote (param $s eqref) (result eqref)
    (local $bytes (ref $bytes)) (local $n i32) (local $size i32)
    (local $ptr i32) (local $out i32) (local $i i32) (local $b i32)
    (local $escape i32) (local $control i32) (local $answer eqref)
    (local.set $bytes (ref.as_non_null (struct.get $obj 4 (ref.cast (ref $obj) (local.get $s)))))
    (local.set $n (array.len (local.get $bytes)))
    (if (i32.gt_u (local.get $n) (i32.const 536870911)) (then unreachable))
    (local.set $size (i32.add (i32.mul (local.get $n) (i32.const 8)) (i32.const 2)))
    (local.set $ptr (call $alloc (local.get $size)))
    (i32.store8 (local.get $ptr) (i32.const 34))
    (local.set $out (i32.const 1))
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
            (i32.store8 (i32.add (local.get $ptr) (local.get $out)) (i32.const 92))
            (i32.store8 (i32.add (local.get $ptr) (i32.add (local.get $out) (i32.const 1))) (local.get $escape))
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
                (i32.store8 (i32.add (local.get $ptr) (local.get $out)) (i32.const 92))
                (i32.store8 (i32.add (local.get $ptr) (i32.add (local.get $out) (i32.const 1))) (i32.const 117))
                (i32.store8 (i32.add (local.get $ptr) (i32.add (local.get $out) (i32.const 2))) (i32.const 123))
                (i32.store8 (i32.add (local.get $ptr) (i32.add (local.get $out) (i32.const 3))) (i32.const 48))
                (i32.store8 (i32.add (local.get $ptr) (i32.add (local.get $out) (i32.const 4))) (i32.const 48))
                (i32.store8 (i32.add (local.get $ptr) (i32.add (local.get $out) (i32.const 5)))
                  (call $inspect_hex (i32.shr_u (local.get $b) (i32.const 4))))
                (i32.store8 (i32.add (local.get $ptr) (i32.add (local.get $out) (i32.const 6)))
                  (call $inspect_hex (i32.and (local.get $b) (i32.const 15))))
                (i32.store8 (i32.add (local.get $ptr) (i32.add (local.get $out) (i32.const 7))) (i32.const 125))
                (local.set $out (i32.add (local.get $out) (i32.const 8))))
              (else
                (i32.store8 (i32.add (local.get $ptr) (local.get $out)) (local.get $b))
                (local.set $out (i32.add (local.get $out) (i32.const 1)))))))
        (br $next)))
    (i32.store8 (i32.add (local.get $ptr) (local.get $out)) (i32.const 34))
    (local.set $answer (call $string_from_mem (local.get $ptr) (i32.add (local.get $out) (i32.const 1))))
    (call $free (local.get $ptr) (local.get $size))
    (local.get $answer))

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
        (if (i32.lt_u (local.get $j) (i32.const 8))
          (then
            (local.set $out (call $list_cons {{bits_size}} (local.get $out)))
            (local.set $out (call $list_cons (call $int_to_string (call $int_from_i64 (i64.extend_i32_u (local.get $j)))) (local.get $out)))
            (local.set $out (call $list_cons {{paren_close}} (local.get $out)))))
        (br $next)))
    (call $string_concat (call $list_reverse (call $list_cons {{bits_close}} (local.get $out)))))

  (func $inspect_function (param $arity i32) (result eqref)
    (local $n i32) (local $i i32) (local $out eqref)
    (local.set $n (local.get $arity))
    (local.set $out (call $list_cons {{fn_open}} (call $list_nil)))
    (block $done
      (loop $next
        (br_if $done (i32.ge_u (local.get $i) (local.get $n)))
        (if (local.get $i) (then (local.set $out (call $list_cons {{comma}} (local.get $out)))))
        (local.set $out (call $list_cons
          (call $string_from_bytes (array.new_fixed $bytes 1
            (i32.add (i32.const 97) (i32.rem_u (local.get $i) (i32.const 26))))) (local.get $out)))
        (if (i32.ge_u (local.get $i) (i32.const 26))
          (then (local.set $out (call $list_cons
            (call $int_to_string (call $int_from_i64
              (i64.extend_i32_u (i32.div_u (local.get $i) (i32.const 26))))) (local.get $out)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $next)))
    (call $string_concat (call $list_reverse (call $list_cons {{fn_close}} (local.get $out)))))

  ;; Aggregate traversal uses GC continuation frames rather than Wasm calls.
  ;; The fragment list is flattened once, so deeply nested values are linear
  ;; in their rendered size rather than copying every ancestor's output.
  (type $inspect_dict_state (struct (field (mut i32))))
  (type $inspect_task (struct
    (field i32) (field eqref) (field i32)
    (field (ref null $inspect_dict_state)) (field (ref null $inspect_task))))

  (func $inspect_push (param $kind i32) (param $value eqref) (param $index i32)
    (param $state (ref null $inspect_dict_state)) (param $next (ref null $inspect_task))
    (result (ref $inspect_task))
    (struct.new $inspect_task (local.get $kind) (local.get $value)
      (local.get $index) (local.get $state) (local.get $next)))

  ;; Atoms contain no recursively inspected values. Keeping this separate also
  ;; avoids allocating traversal frames for the common scalar-only path.
  (func $inspect_atom (param $value eqref) (param $tag i32) (result eqref)
    (local $obj (ref $obj))
    (if (i32.eqz (local.get $tag)) (then (return {{nil}})))
    (if (i32.eq (local.get $tag) (i32.const 8)) (then (return_call $int_to_string (local.get $value))))
    (if (i32.eq (local.get $tag) (i32.const 1))
      (then (return (if (result eqref) (call $bool_get (local.get $value)) (then {{true}}) (else {{false}})))))
    (if (i32.eq (local.get $tag) (i32.const 2)) (then (return_call $float_to_string (local.get $value))))
    (if (i32.eq (local.get $tag) (i32.const 3)) (then (return_call $inspect_quote (local.get $value))))
    (if (i32.eq (local.get $tag) (i32.const 9)) (then (return_call $inspect_bits (local.get $value))))
    (if (i32.eq (local.get $tag) (i32.const 10)) (then (return {{opaque}})))
    (local.set $obj (ref.cast (ref $obj) (local.get $value)))
    (if (i32.eq (local.get $tag) (i32.const 7))
      (then (return_call $inspect_function (i32.wrap_i64 (struct.get $obj 2 (local.get $obj))))))
    (if (i32.eq (local.get $tag) (i32.const 13))
      (then (return (call $string_append (call $string_append {{codepoint_open}}
        (call $string_from_codepoints (call $list_cons
          (call $int_from_i64 (i64.extend_i32_u (struct.get $obj 1 (local.get $obj)))) (call $list_nil)))) {{paren_close}}))))
    {{unknown}})

  (func $inspect (param $value eqref) (result eqref)
    (local $work (ref null $inspect_task)) (local $task (ref $inspect_task))
    (local $state (ref null $inspect_dict_state)) (local $obj (ref $obj))
    (local $fields (ref null $values)) (local $head eqref) (local $out eqref)
    (local $kind i32) (local $index i32) (local $n i32) (local $tag i32)
    (local.set $tag (call $value_tag (local.get $value)))
    (if (i32.eqz (i32.or (i32.or
      (i32.eq (local.get $tag) (i32.const 4)) (i32.eq (local.get $tag) (i32.const 5)))
      (i32.or (i32.eq (local.get $tag) (i32.const 6)) (i32.eq (local.get $tag) (i32.const 11)))))
      (then (return_call $inspect_atom (local.get $value) (local.get $tag))))
    (local.set $out (call $list_nil))
    (local.set $work (call $inspect_push (i32.const 0) (local.get $value) (i32.const 0)
      (ref.null $inspect_dict_state) (ref.null $inspect_task)))
    (block $done (loop $next
      (br_if $done (ref.is_null (local.get $work)))
      (local.set $task (ref.as_non_null (local.get $work)))
      (local.set $work (struct.get $inspect_task 4 (local.get $task)))
      (local.set $kind (struct.get $inspect_task 0 (local.get $task)))
      (local.set $value (struct.get $inspect_task 1 (local.get $task)))
      (local.set $index (struct.get $inspect_task 2 (local.get $task)))
      (local.set $state (struct.get $inspect_task 3 (local.get $task)))

      ;; 1: already-rendered literal fragment.
      (if (i32.eq (local.get $kind) (i32.const 1))
        (then
          (local.set $out (call $list_cons (local.get $value) (local.get $out)))
          (br $next)))

      ;; 2: tuple/record field sequence, retaining its next index.
      (if (i32.eq (local.get $kind) (i32.const 2))
        (then
          (local.set $fields (struct.get $obj 5 (ref.cast (ref $obj) (local.get $value))))
          (if (ref.is_null (local.get $fields)) (then (br $next)))
          (br_if $next (i32.ge_u (local.get $index) (array.len (ref.as_non_null (local.get $fields)))))
          (if (local.get $index) (then (local.set $out (call $list_cons {{comma}} (local.get $out)))))
          (local.set $work (call $inspect_push (i32.const 2) (local.get $value)
            (i32.add (local.get $index) (i32.const 1)) (ref.null $inspect_dict_state) (local.get $work)))
          (local.set $work (call $inspect_push (i32.const 0)
            (array.get $values (ref.as_non_null (local.get $fields)) (local.get $index))
            (i32.const 0) (ref.null $inspect_dict_state) (local.get $work)))
          (br $next)))

      ;; 3: list continuation. An empty tail emits the closing bracket.
      (if (i32.eq (local.get $kind) (i32.const 3))
        (then
          (local.set $fields (struct.get $obj 5 (ref.cast (ref $obj) (local.get $value))))
          (if (ref.is_null (local.get $fields))
            (then (local.set $out (call $list_cons {{list_close}} (local.get $out))) (br $next)))
          (if (local.get $index) (then (local.set $out (call $list_cons {{comma}} (local.get $out)))))
          (local.set $work (call $inspect_push (i32.const 3)
            (array.get $values (ref.as_non_null (local.get $fields)) (i32.const 1))
            (i32.const 1) (ref.null $inspect_dict_state) (local.get $work)))
          (local.set $work (call $inspect_push (i32.const 0)
            (array.get $values (ref.as_non_null (local.get $fields)) (i32.const 0))
            (i32.const 0) (ref.null $inspect_dict_state) (local.get $work)))
          (br $next)))

      ;; 4: canonical HAMT node. Every node in one dictionary shares its comma
      ;; state; a dictionary nested in a key/value receives a fresh state.
      (if (i32.eq (local.get $kind) (i32.const 4))
        (then
          (local.set $fields (struct.get $obj 5 (ref.cast (ref $obj) (local.get $value))))
          (if (ref.is_null (local.get $fields)) (then (br $next)))
          (local.set $n (array.len (ref.as_non_null (local.get $fields))))
          (br_if $next (i32.eqz (local.get $n)))
          (if (i32.eq (local.get $n) (i32.const 3))
            (then
              (if (i32.eqz (struct.get $inspect_dict_state 0 (ref.as_non_null (local.get $state))))
                (then (local.set $out (call $list_cons {{comma}} (local.get $out)))))
              (struct.set $inspect_dict_state 0 (ref.as_non_null (local.get $state)) (i32.const 0))
              (local.set $out (call $list_cons {{tuple_open}} (local.get $out)))
              (local.set $work (call $inspect_push (i32.const 1) {{paren_close}} (i32.const 0)
                (ref.null $inspect_dict_state) (local.get $work)))
              (local.set $work (call $inspect_push (i32.const 0)
                (array.get $values (ref.as_non_null (local.get $fields)) (i32.const 2))
                (i32.const 0) (ref.null $inspect_dict_state) (local.get $work)))
              (local.set $work (call $inspect_push (i32.const 1) {{comma}} (i32.const 0)
                (ref.null $inspect_dict_state) (local.get $work)))
              (local.set $work (call $inspect_push (i32.const 0)
                (array.get $values (ref.as_non_null (local.get $fields)) (i32.const 1))
                (i32.const 0) (ref.null $inspect_dict_state) (local.get $work)))
              (br $next)))
          (local.set $work (call $inspect_push (i32.const 5)
            (array.get $values (ref.as_non_null (local.get $fields)) (i32.const 1))
            (i32.const 0) (local.get $state) (local.get $work)))
          (br $next)))

      ;; 5: a branch's child nodes or a hash collision's list of key/value pairs.
      (if (i32.eq (local.get $kind) (i32.const 5))
        (then
          (local.set $fields (struct.get $obj 5 (ref.cast (ref $obj) (local.get $value))))
          (if (ref.is_null (local.get $fields)) (then (br $next)))
          (local.set $head (array.get $values (ref.as_non_null (local.get $fields)) (i32.const 0)))
          (local.set $work (call $inspect_push (i32.const 5)
            (array.get $values (ref.as_non_null (local.get $fields)) (i32.const 1))
            (i32.const 0) (local.get $state) (local.get $work)))
          (if (i32.eq (call $value_tag (local.get $head)) (i32.const 4))
            (then
              (if (i32.eqz (struct.get $inspect_dict_state 0 (ref.as_non_null (local.get $state))))
                (then (local.set $out (call $list_cons {{comma}} (local.get $out)))))
              (struct.set $inspect_dict_state 0 (ref.as_non_null (local.get $state)) (i32.const 0))
              (local.set $work (call $inspect_push (i32.const 0) (local.get $head) (i32.const 0)
                (ref.null $inspect_dict_state) (local.get $work))))
            (else (local.set $work (call $inspect_push (i32.const 4) (local.get $head) (i32.const 0)
              (local.get $state) (local.get $work)))))
          (br $next)))

      ;; 0: inspect a value, scheduling aggregate children without calling back
      ;; into inspection on the Wasm call stack.
      (local.set $tag (call $value_tag (local.get $value)))
      (if (i32.or (i32.eq (local.get $tag) (i32.const 4)) (i32.eq (local.get $tag) (i32.const 5)))
        (then
          (local.set $obj (ref.cast (ref $obj) (local.get $value)))
          (local.set $fields (struct.get $obj 5 (local.get $obj)))
          (if (i32.eq (local.get $tag) (i32.const 5))
            (then
              (if (call $type_is_dict (struct.get $obj 2 (local.get $obj)))
                (then
                  (local.set $out (call $list_cons {{dict_open}} (local.get $out)))
                  (local.set $work (call $inspect_push (i32.const 1) {{dict_close}} (i32.const 0)
                    (ref.null $inspect_dict_state) (local.get $work)))
                  (local.set $work (call $inspect_push (i32.const 4)
                    (array.get $values (ref.as_non_null (local.get $fields)) (i32.const 1))
                    (i32.const 0) (struct.new $inspect_dict_state (i32.const 1)) (local.get $work)))
                  (br $next)))
              (local.set $out (call $list_cons (call $constructor_name (struct.get $obj 1 (local.get $obj))) (local.get $out)))
              (if (ref.is_null (local.get $fields)) (then (br $next)))
              (br_if $next (i32.eqz (array.len (ref.as_non_null (local.get $fields)))))
              (local.set $out (call $list_cons {{paren_open}} (local.get $out))))
            (else (local.set $out (call $list_cons {{tuple_open}} (local.get $out)))))
          (local.set $work (call $inspect_push (i32.const 1) {{paren_close}} (i32.const 0)
            (ref.null $inspect_dict_state) (local.get $work)))
          (local.set $work (call $inspect_push (i32.const 2) (local.get $value) (i32.const 0)
            (ref.null $inspect_dict_state) (local.get $work)))
          (br $next)))
      (if (i32.eq (local.get $tag) (i32.const 6))
        (then
          (local.set $out (call $list_cons {{list_open}} (local.get $out)))
          (local.set $work (call $inspect_push (i32.const 3) (local.get $value) (i32.const 0)
            (ref.null $inspect_dict_state) (local.get $work)))
          (br $next)))
      (if (i32.eq (local.get $tag) (i32.const 11))
        (then
          (local.set $work (call $inspect_push (i32.const 0)
            (call $field (local.get $value) (i32.const 0)) (i32.const 0)
            (ref.null $inspect_dict_state) (local.get $work)))
          (br $next)))
      (local.set $out (call $list_cons (call $inspect_atom (local.get $value) (local.get $tag)) (local.get $out)))
      (br $next)))
    (call $string_concat (call $list_reverse (local.get $out))))

  (func $value_inspect (param $value eqref) (result eqref)
    (return_call $inspect (local.get $value)))

  (func $echo (param $value eqref) (result eqref)
    (local $s eqref) (local $ptr i32) (local $len i32)
    (local.set $s (call $string_append (call $inspect (local.get $value)) {{newline}}))
    (local.set $len (call $string_length_bytes (local.get $s)))
    (local.set $ptr (call $string_to_mem (local.get $s)))
    (call $host_write (i32.const 2) (local.get $ptr) (local.get $len))
    (call $free (local.get $ptr) (local.get $len))
    (local.get $value))
"#
    .to_owned();

    for (name, value) in [
        ("empty", ""),
        ("comma", ", "),
        ("list_open", "["),
        ("list_close", "]"),
        ("tuple_open", "#("),
        ("paren_open", "("),
        ("paren_close", ")"),
        ("bits_open", "<<"),
        ("bits_close", ">>"),
        ("bits_size", ":size("),
        ("fn_open", "//fn("),
        ("fn_close", ") { ... }"),
        ("nil", "Nil"),
        ("true", "True"),
        ("false", "False"),
        ("dict_open", "dict.from_list(["),
        ("dict_close", "])"),
        ("opaque", "//wasm(opaque)"),
        ("codepoint_open", "//utfcodepoint("),
        ("unknown", "//wasm(unknown)"),
        ("newline", "\n"),
    ] {
        source = source.replace(&format!("{{{{{name}}}}}"), &literal(value));
    }
    source.push_str(&float_format::wat());
    source
}

#[cfg(test)]
mod tests;
