import gleam/dict
import gleam/dynamic
import gleam/dynamic/decode.{DecodeError}
import gleam/option.{None, Some}

pub fn tagged_primitives_test() -> Nil {
  assert dynamic.classify(dynamic.nil()) == "Nil"
  assert dynamic.classify(dynamic.bool(True)) == "Bool"
  assert dynamic.classify(dynamic.string("香港")) == "String"
  assert dynamic.classify(dynamic.int(1)) == "Int"
  assert dynamic.classify(dynamic.float(1.0)) == "Float"
  assert dynamic.classify(dynamic.bit_array(<<1:size(1)>>)) == "BitArray"
  assert dynamic.classify(dynamic.list([])) == "List"
  assert dynamic.classify(dynamic.array([])) == "Array"
  assert dynamic.classify(dynamic.properties([])) == "Dict"
  Nil
}

pub fn strict_numbers_test() -> Nil {
  let huge = 340_282_366_920_938_463_463_374_607_431_768_211_457
  assert decode.run(dynamic.int(huge), decode.int) == Ok(huge)
  assert decode.run(dynamic.float(1.0), decode.int)
    == Error([DecodeError("Int", "Float", [])])
  assert decode.run(dynamic.int(1), decode.float)
    == Error([DecodeError("Float", "Int", [])])
  Nil
}

pub fn list_and_array_decoders_test() -> Nil {
  let elements = [dynamic.int(1), dynamic.int(2), dynamic.int(3)]
  let decoder = decode.list(decode.int)
  assert decode.run(dynamic.list(elements), decoder) == Ok([1, 2, 3])
  assert decode.run(dynamic.array(elements), decoder) == Ok([1, 2, 3])
  assert decode.run(dynamic.array([]), decoder) == Ok([])
  assert decode.run(dynamic.string("not a list"), decoder)
    == Error([DecodeError("List", "String", [])])
  Nil
}

pub fn nested_list_error_path_test() -> Nil {
  let data =
    dynamic.list([
      dynamic.list([dynamic.int(1)]),
      dynamic.array([dynamic.int(2), dynamic.string("wrong")]),
    ])
  assert decode.run(data, decode.list(decode.list(decode.int)))
    == Error([DecodeError("Int", "String", ["1", "1"])])
  Nil
}

pub fn array_missing_index_test() -> Nil {
  let data = dynamic.array([dynamic.string("first")])
  assert decode.run(data, decode.at([0], decode.string)) == Ok("first")
  assert decode.run(data, decode.optionally_at([1], "missing", decode.string))
    == Ok("missing")
  assert decode.run(data, decode.optionally_at([-1], "missing", decode.string))
    == Ok("missing")
  Nil
}

pub fn linked_list_index_boundary_test() -> Nil {
  let data =
    dynamic.list([
      dynamic.int(0),
      dynamic.int(1),
      dynamic.int(2),
      dynamic.int(3),
      dynamic.int(4),
      dynamic.int(5),
      dynamic.int(6),
      dynamic.int(7),
      dynamic.int(8),
    ])
  assert decode.run(data, decode.at([7], decode.int)) == Ok(7)
  assert decode.run(data, decode.at([8], decode.int))
    == Error([DecodeError("Indexable", "List", [])])
  Nil
}

pub fn property_keys_are_typed_test() -> Nil {
  let data =
    dynamic.properties([
      #(dynamic.int(1), dynamic.string("integer")),
      #(dynamic.string("1"), dynamic.string("string")),
    ])
  assert decode.run(data, decode.at([1], decode.string)) == Ok("integer")
  assert decode.run(data, decode.at(["1"], decode.string)) == Ok("string")
  Nil
}

pub fn optional_missing_is_distinct_from_wrong_type_test() -> Nil {
  let data =
    dynamic.properties([
      #(dynamic.string("present"), dynamic.string("wrong")),
      #(dynamic.string("null"), dynamic.nil()),
    ])
  assert decode.run(data, decode.optionally_at(["missing"], 5, decode.int))
    == Ok(5)
  assert decode.run(data, decode.optionally_at(["present"], 5, decode.int))
    == Error([DecodeError("Int", "String", ["present"])])
  assert decode.run(data, decode.at(["null"], decode.optional(decode.int)))
    == Ok(None)
  assert decode.run(dynamic.int(5), decode.optional(decode.int)) == Ok(Some(5))
  Nil
}

pub fn decoder_accumulates_record_fields_test() -> Nil {
  let decoder = {
    use name <- decode.field("name", decode.string)
    use age <- decode.field("age", decode.int)
    decode.success(#(name, age))
  }
  let data =
    dynamic.properties([
      #(dynamic.string("name"), dynamic.int(12)),
      #(dynamic.string("age"), dynamic.string("old")),
    ])
  assert decode.run(data, decoder)
    == Error([
      DecodeError("String", "Int", ["name"]),
      DecodeError("Int", "String", ["age"]),
    ])
  Nil
}

pub fn dictionary_decoder_test() -> Nil {
  let data =
    dynamic.properties([
      #(dynamic.string("a"), dynamic.int(1)),
      #(dynamic.string("b"), dynamic.int(2)),
    ])
  assert decode.run(data, decode.dict(decode.string, decode.int))
    == Ok(dict.from_list([#("a", 1), #("b", 2)]))
  Nil
}

pub fn empty_array_and_list_are_not_equal_test() -> Nil {
  assert dynamic.array([]) != dynamic.list([])
  Nil
}
