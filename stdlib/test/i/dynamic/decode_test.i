import i/dict
import i/dynamic.{type Dynamic}
import i/dynamic/decode.{DecodeError}
import i/float
import i/int
import i/list
import i/option
import i/result
import i/string

pub type User {
  User(
    name: String,
    email: String,
    is_admin: Bool,
    is_confirmed: Bool,
    score: Int,
  )
}

pub fn decoder_test() -> Nil {
  let data =
    dynamic.properties([
      #(dynamic.string("name"), dynamic.string("Nubi")),
      #(dynamic.string("email"), dynamic.string("nubi@example.com")),
      #(dynamic.string("is_admin"), dynamic.bool(False)),
      #(dynamic.string("is_confirmed"), dynamic.bool(True)),
      #(dynamic.string("score"), dynamic.int(180)),
    ])

  let decoder = {
    use name <- decode.field("name", decode.string)
    use email <- decode.field("email", decode.string)
    use is_admin <- decode.field("is_admin", decode.bool)
    use is_confirmed <- decode.field("is_confirmed", decode.bool)
    use score <- decode.field("score", decode.int)
    decode.success(User(
      name: name,
      email: email,
      is_admin: is_admin,
      is_confirmed: is_confirmed,
      score: score,
    ))
  }

  let assert Ok(value) = decode.run(data, decoder)
  assert value == User("Nubi", "nubi@example.com", False, True, 180)
  Nil
}

pub fn field_ok_test() -> Nil {
  let data =
    dynamic.properties([#(dynamic.string("name"), dynamic.string("Nubi"))])
  let decoder = {
    use name <- decode.field("name", decode.string)
    decode.success(name)
  }

  let assert Ok(value) = decode.run(data, decoder)
  assert value == "Nubi"
  Nil
}

pub fn subfield_ok_test() -> Nil {
  let data =
    dynamic.properties([
      #(
        dynamic.string("person"),
        dynamic.properties([#(dynamic.string("name"), dynamic.string("Nubi"))]),
      ),
    ])
  let decoder = {
    use name <- decode.subfield(["person", "name"], decode.string)
    decode.success(name)
  }

  let assert Ok(value) = decode.run(data, decoder)
  assert value == "Nubi"
  Nil
}

pub fn field_int_index_ok_test() -> Nil {
  let decoder = {
    use x <- decode.field(0, decode.string)
    use y <- decode.field(1, decode.string)
    decode.success(#(x, y))
  }

  let assert Ok(value) =
    decode.run(
      dynamic.array(["one", "two", "three"] |> list.map(dynamic.string)),
      decoder,
    )
  assert value == #("one", "two")
  Nil
}

pub fn field_int_index_list_ok_test() -> Nil {
  let decoder = {
    use x <- decode.field(0, decode.string)
    use y <- decode.field(1, decode.string)
    decode.success(#(x, y))
  }

  let assert Ok(value) =
    decode.run(
      dynamic.list(["one", "two", "three", "four"] |> list.map(dynamic.string)),
      decoder,
    )
  assert value == #("one", "two")
  Nil
}

pub fn field_int_index_big_list_ok_test() -> Nil {
  let decoder = {
    use x <- decode.field(6, decode.string)
    use y <- decode.field(7, decode.string)
    decode.success(#(x, y))
  }

  let assert Ok(value) =
    decode.run(
      dynamic.list(
        ["one", "two", "three", "four", "five", "six", "seven", "eight"]
        |> list.map(dynamic.string),
      ),
      decoder,
    )
  assert value == #("seven", "eight")
  Nil
}

pub fn subfield_not_found_error_test() -> Nil {
  let decoder = {
    use name <- decode.subfield(["name"], decode.string)
    decode.success(name)
  }

  let assert Error(value) = decode.run(dynamic.int(123), decoder)
  assert value == [DecodeError("Dict", "Int", [])]
  Nil
}

pub fn field_not_found_error_test() -> Nil {
  let decoder = {
    use name <- decode.subfield(["name"], decode.string)
    decode.success(name)
  }

  let assert Error(value) = decode.run(dynamic.int(123), decoder)
  assert value == [DecodeError("Dict", "Int", [])]
  Nil
}

pub fn field_wrong_inner_error_test() -> Nil {
  let decoder = {
    use name <- decode.field("name", decode.string)
    decode.success(name)
  }

  let assert Error(value) =
    decode.run(
      dynamic.properties([#(dynamic.string("name"), dynamic.int(123))]),
      decoder,
    )
  assert value == [DecodeError("String", "Int", ["name"])]
  Nil
}

pub fn subfield_int_index_ok_test() -> Nil {
  let decoder = {
    use x <- decode.subfield([0, 1], decode.string)
    use y <- decode.subfield([1, 0], decode.string)
    decode.success(#(x, y))
  }

  let assert Ok(value) =
    decode.run(
      dynamic.array([
        dynamic.array(["one", "two", "three"] |> list.map(dynamic.string)),
        dynamic.array(["a", "b"] |> list.map(dynamic.string)),
      ]),
      decoder,
    )
  assert value == #("two", "a")
  Nil
}

pub fn subfield_wrong_inner_error_test() -> Nil {
  let data = dynamic.properties([#(dynamic.string("name"), dynamic.int(123))])
  let assert Error(value) =
    decode.run(data, {
      use name <- decode.field("name", decode.string)
      decode.success(name)
    })
  assert value == [DecodeError("String", "Int", ["name"])]
  Nil
}

pub fn optional_field_wrong_inner_error_test() -> Nil {
  let data = dynamic.properties([#(dynamic.string("a"), dynamic.nil())])
  let assert Error(value) =
    decode.run(data, {
      use bar <- decode.optional_field("a", "", decode.string)
      decode.success(bar)
    })
  assert value == [DecodeError("String", "Nil", ["a"])]
  Nil
}

pub fn sub_optional_field_wrong_inner_error_test() -> Nil {
  let data =
    dynamic.properties([
      #(
        dynamic.string("a"),
        dynamic.properties([#(dynamic.string("b"), dynamic.nil())]),
      ),
    ])
  let assert Error(value) =
    decode.run(data, {
      use bar <- decode.optional_field("a", "", {
        use foo <- decode.optional_field("b", "", decode.string)
        decode.success(foo)
      })
      decode.success(bar)
    })
  assert value == [DecodeError("String", "Nil", ["a", "b"])]
  Nil
}

pub fn optional_field_wrong_inner_error_type_test() -> Nil {
  let data = dynamic.properties([#(dynamic.string("a"), dynamic.int(0))])
  let assert Error(value) =
    decode.run(data, {
      use bar <- decode.optional_field("a", "", decode.string)
      decode.success(bar)
    })
  assert value == [DecodeError("String", "Int", ["a"])]
  Nil
}

pub fn string_map_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.string("tEsT"),
      decode.string |> decode.map(string.lowercase),
    )
  assert value == "test"
  Nil
}

pub fn string_map_error_test() -> Nil {
  let assert Error(_) =
    decode.run(dynamic.int(0), decode.string |> decode.map(string.lowercase))
  Nil
}

pub fn string_ok_test() -> Nil {
  let assert Ok(value) = decode.run(dynamic.string("Hello!"), decode.string)
  assert value == "Hello!"
  Nil
}

pub fn string_error_test() -> Nil {
  let assert Error(value) = decode.run(dynamic.int(123), decode.string)
  assert value == [DecodeError("String", "Int", [])]
  Nil
}

pub fn dynamic_test() -> Nil {
  let data = dynamic.int(123)
  let assert Ok(value) = decode.run(data, decode.dynamic)
  assert value == data
  Nil
}

pub fn int_ok_test() -> Nil {
  let assert Ok(value) = decode.run(dynamic.int(123), decode.int)
  assert value == 123
  Nil
}

pub fn int_error_test() -> Nil {
  let assert Error(value) = decode.run(dynamic.string("123"), decode.int)
  assert value == [DecodeError("Int", "String", [])]
  Nil
}

pub fn float_ok_test() -> Nil {
  let assert Ok(value) = decode.run(dynamic.float(123.45), decode.float)
  assert value == 123.45
  Nil
}

pub fn float_error_test() -> Nil {
  let assert Error(value) = decode.run(dynamic.string("123.45"), decode.float)
  assert value == [DecodeError("Float", "String", [])]
  Nil
}

pub fn bool_true_test() -> Nil {
  let assert Ok(value) = decode.run(dynamic.bool(True), decode.bool)
  assert value == True
  Nil
}

pub fn bool_false_test() -> Nil {
  let assert Ok(value) = decode.run(dynamic.bool(False), decode.bool)
  assert value == False
  Nil
}

pub fn bool_error_test() -> Nil {
  let assert Error(value) = decode.run(dynamic.int(123), decode.bool)
  assert value == [DecodeError("Bool", "Int", [])]
  Nil
}

pub fn bit_array_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(dynamic.bit_array(<<1, 5, 3>>), decode.bit_array)
  assert value == <<1, 5, 3>>
  Nil
}

pub fn bit_array_error_test() -> Nil {
  let assert Error(value) = decode.run(dynamic.int(123), decode.bit_array)
  assert value == [DecodeError("BitArray", "Int", [])]
  Nil
}

pub fn list_tuple_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.array([dynamic.string("Hello"), dynamic.string("Joe")]),
      decode.list(decode.string),
    )
  assert value == ["Hello", "Joe"]
  Nil
}

pub fn list_string_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.list(["Hello", "Joe"] |> list.map(dynamic.string)),
      decode.list(decode.string),
    )
  assert value == ["Hello", "Joe"]
  Nil
}

pub fn list_bool_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.list([True, False] |> list.map(dynamic.bool)),
      decode.list(decode.bool),
    )
  assert value == [True, False]
  Nil
}

pub fn list_error_test() -> Nil {
  let assert Error(value) =
    decode.run(dynamic.int(123), decode.list(decode.int))
  assert value == [DecodeError("List", "Int", [])]
  Nil
}

pub fn list_inner_0_error_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.list([dynamic.int(1), dynamic.int(2)]),
      decode.list(decode.string),
    )
  assert value == [DecodeError("String", "Int", ["0"])]
  Nil
}

pub fn list_inner_1_error_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.list([dynamic.string("1"), dynamic.int(2)]),
      decode.list(decode.string),
    )
  assert value == [DecodeError("String", "Int", ["1"])]
  Nil
}

pub fn list_tuple_inner_1_error_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.array([dynamic.string("1"), dynamic.int(2)]),
      decode.list(decode.string),
    )
  assert value == [DecodeError("String", "Int", ["1"])]
  Nil
}

pub fn dict_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("first"), dynamic.int(1)),
        #(dynamic.string("second"), dynamic.int(2)),
      ]),
      decode.dict(decode.string, decode.int),
    )
  assert value == dict.from_list([#("first", 1), #("second", 2)])
  Nil
}

pub fn dict_value_error_float_key_test() -> Nil {
  // A single invalid value makes the float-key error path independent of the
  // dictionary's unspecified traversal order.
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.float(1.1), dynamic.int(1)),
        #(dynamic.float(1.2), dynamic.string("valid")),
      ]),
      decode.dict(decode.float, decode.string),
    )
  assert value == [DecodeError("String", "Int", ["1.1"])]

  // Retain the original two-invalid-values case. Either complete first-error
  // result is valid; an empty, unrelated or multi-error result is not.
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.float(1.1), dynamic.int(1)),
        #(dynamic.float(1.2), dynamic.int(2)),
      ]),
      decode.dict(decode.float, decode.string),
    )
  assert value == [DecodeError("String", "Int", ["1.1"])]
    || value == [DecodeError("String", "Int", ["1.2"])]
  Nil
}

pub fn dict_value_error_string_key_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("a"), dynamic.string("first")),
        #(dynamic.string("b"), dynamic.int(2)),
      ]),
      decode.dict(decode.string, decode.string),
    )
  assert value == [DecodeError("String", "Int", ["b"])]
  Nil
}

pub fn dict_value_error_int_key_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.int(1), dynamic.string("first")),
        #(dynamic.int(2), dynamic.int(2)),
      ]),
      decode.dict(decode.int, decode.string),
    )
  assert value == [DecodeError("String", "Int", ["2"])]
  Nil
}

pub fn dict_value_error_bit_array_key_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.bit_array(<<0:5>>), dynamic.string("first")),
        #(dynamic.bit_array(<<1:5>>), dynamic.int(2)),
      ]),
      decode.dict(decode.bit_array, decode.string),
    )
  assert value == [DecodeError("String", "Int", ["<BitArray>"])]
  Nil
}

pub fn dict_key_error_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.float(1.1), dynamic.int(1)),
        #(dynamic.float(1.2), dynamic.int(2)),
      ]),
      decode.dict(decode.string, decode.int),
    )
  assert value == [DecodeError("String", "Float", ["keys"])]
  Nil
}

pub fn dict_error_test() -> Nil {
  let assert Error(value) =
    decode.run(dynamic.int(123), decode.dict(decode.string, decode.int))
  assert value == [DecodeError("Dict", "Int", [])]
  Nil
}

pub fn at_dict_string_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(
          dynamic.string("first"),
          dynamic.properties([
            #(
              dynamic.string("second"),
              dynamic.properties([#(dynamic.string("third"), dynamic.int(1337))]),
            ),
          ]),
        ),
      ]),
      decode.at(["first", "second", "third"], decode.int),
    )
  assert value == 1337
  Nil
}

pub fn at_dict_int_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(
          dynamic.int(10),
          dynamic.properties([
            #(
              dynamic.int(20),
              dynamic.properties([#(dynamic.int(30), dynamic.int(1337))]),
            ),
          ]),
        ),
      ]),
      decode.at([10, 20, 30], decode.int),
    )
  assert value == 1337
  Nil
}

pub fn at_tuple_int_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.array([
        dynamic.string("x"),
        dynamic.array(["a", "b", "c"] |> list.map(dynamic.string)),
        dynamic.string("z"),
      ]),
      decode.at([1, 0], decode.string),
    )
  assert value == "a"
  Nil
}

pub fn at_wrong_inner_error_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(
          dynamic.string("first"),
          dynamic.properties([
            #(
              dynamic.string("second"),
              dynamic.properties([#(dynamic.string("third"), dynamic.int(1337))]),
            ),
          ]),
        ),
      ]),
      decode.at(["first", "second", "third"], decode.string),
    )
  assert value == [DecodeError("String", "Int", ["first", "second", "third"])]
  Nil
}

pub fn at_no_path_error_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(
          dynamic.string("first"),
          dynamic.properties([#(dynamic.string("third"), dynamic.int(1337))]),
        ),
      ]),
      decode.at(["first", "second", "third"], decode.int),
    )
  assert value == [DecodeError("Field", "Nothing", ["first", "second"])]
  Nil
}

pub fn optional_string_present_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(dynamic.string("Hello, Joe!"), decode.optional(decode.string))
  assert value == option.Some("Hello, Joe!")
  Nil
}

pub fn optional_bool_present_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(dynamic.bool(True), decode.optional(decode.bool))
  assert value == option.Some(True)
  Nil
}

pub fn optional_bool_absent_nil_ok_test() -> Nil {
  let assert Ok(value) = decode.run(dynamic.nil(), decode.optional(decode.bool))
  assert value == option.None
  Nil
}

pub fn optional_error_test() -> Nil {
  let assert Error(value) =
    decode.run(dynamic.int(123), decode.optional(decode.string))
  assert value == [DecodeError("String", "Int", [])]
  Nil
}

pub fn map_test() -> Nil {
  let assert Ok(value) =
    decode.run(dynamic.int(123), decode.int |> decode.map(int.to_string))
  assert value == "123"
  Nil
}

pub fn map_errors_test() -> Nil {
  let decoder =
    decode.at(
      ["data"],
      decode.map_errors(decode.string, fn(errors) {
        let assert [DecodeError("String", "Int", [])] = errors
        [
          DecodeError("Wibble", "Wobble", ["ok"]),
          DecodeError("Wabble", "Wubble", ["ok"]),
        ]
      }),
    )

  let assert Error(value) =
    decode.run(
      dynamic.properties([#(dynamic.string("data"), dynamic.int(123))]),
      decoder,
    )
  assert value
    == [
      DecodeError("Wibble", "Wobble", ["data", "ok"]),
      DecodeError("Wabble", "Wubble", ["data", "ok"]),
    ]
  Nil
}

pub fn collapse_errors_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.properties([#(dynamic.string("data"), dynamic.int(123))]),
      decode.at(["data"], decode.string |> decode.collapse_errors("Wibble")),
    )
  assert value == [DecodeError("Wibble", "Int", ["data"])]
  Nil
}

pub fn then_test() -> Nil {
  let decoder =
    decode.at(["key"], decode.int)
    |> decode.then(fn(i) {
      decode.at(["value"], case i {
        1 -> decode.int |> decode.map(AnInt)
        _ -> decode.string |> decode.map(AString)
      })
    })

  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("key"), dynamic.int(1)),
        #(dynamic.string("value"), dynamic.int(100)),
      ]),
      decoder,
    )
  assert value == AnInt(100)

  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("key"), dynamic.int(2)),
        #(dynamic.string("value"), dynamic.string("Hi!")),
      ]),
      decoder,
    )
  assert value == AString("Hi!")
  Nil
}

type IntOrString {
  AnInt(Int)
  AString(String)
}

pub fn then_error_0_test() -> Nil {
  let decoder =
    decode.at(["key"], decode.int)
    |> decode.then(fn(i) {
      decode.at(["value"], case i {
        1 -> decode.int |> decode.map(AnInt)
        _ -> decode.string |> decode.map(AString)
      })
    })

  let assert Error(value) = decode.run(dynamic.int(123), decoder)
  assert value == [DecodeError("Dict", "Int", [])]
  Nil
}

pub fn then_error_1_test() -> Nil {
  let decoder =
    decode.at(["key"], decode.int)
    |> decode.then(fn(i) {
      decode.at(["value"], case i {
        1 -> decode.int |> decode.map(AnInt)
        _ -> decode.string |> decode.map(AString)
      })
    })

  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("key"), dynamic.int(1)),
        #(dynamic.string("value"), dynamic.string("Hi!")),
      ]),
      decoder,
    )
  assert value == [DecodeError("Int", "String", ["value"])]
  Nil
}

pub type MyEnum {
  A
  B
  C
}

pub fn then_enum_test() -> Nil {
  let decoder =
    decode.string
    |> decode.then(fn(s) {
      case s {
        "a" -> decode.success(A)
        "b" -> decode.success(B)
        "c" -> decode.success(C)
        _ -> decode.failure(A, "MyEnum")
      }
    })

  let assert Ok(value) = decode.run(dynamic.string("a"), decoder)
  assert value == A

  let assert Ok(value) = decode.run(dynamic.string("b"), decoder)
  assert value == B

  let assert Ok(value) = decode.run(dynamic.string("c"), decoder)
  assert value == C

  let assert Error(value) = decode.run(dynamic.string("d"), decoder)
  assert value == [DecodeError("MyEnum", "String", [])]
  Nil
}

pub fn one_of_ok_0_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.string("Hello!"),
      decode.one_of(decode.string, [decode.int |> decode.map(int.to_string)]),
    )
  assert value == "Hello!"
  Nil
}

pub fn one_of_ok_1_test() -> Nil {
  let decoder =
    decode.one_of(decode.string, [
      decode.int
      |> decode.map(int.to_string),
    ])
  let assert Ok(value) = decode.run(dynamic.int(123), decoder)
  assert value == "123"
  Nil
}

pub fn one_of_ok_2_test() -> Nil {
  let decoder =
    decode.one_of(decode.string, [
      decode.int |> decode.map(int.to_string),
      decode.float |> decode.map(float.to_string),
    ])
  let assert Ok(value) = decode.run(dynamic.float(12.45), decoder)
  assert value == "12.45"
  Nil
}

pub fn one_of_error_test() -> Nil {
  let decoder =
    decode.one_of(decode.string, or: [
      decode.int
      |> decode.map(int.to_string),
    ])
  let assert Error(value) = decode.run(dynamic.float(1.2), decoder)
  assert value == [DecodeError("String", "Float", [])]
  Nil
}

pub fn failure_test() -> Nil {
  let assert Error(value) =
    decode.run(dynamic.int(123), decode.failure(1, "WibbleWobble"))
  assert value == [DecodeError("WibbleWobble", "Int", [])]
  Nil
}

pub fn variants_test() -> Nil {
  let decoder = {
    use tag <- decode.field("tag", decode.string)
    case tag {
      "int" -> {
        use int <- decode.field("the-int", decode.int)
        decode.success(AnInt(int))
      }
      "string" -> {
        use string <- decode.field("the-string", decode.string)
        decode.success(AString(string))
      }
      _ -> {
        decode.failure(AnInt(0), "IntOrString")
      }
    }
  }

  // Int variant
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("tag"), dynamic.string("int")),
        #(dynamic.string("the-int"), dynamic.int(123)),
      ]),
      decoder,
    )
  assert value == AnInt(123)

  // String variant
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("tag"), dynamic.string("string")),
        #(dynamic.string("the-string"), dynamic.string("hello")),
      ]),
      decoder,
    )
  assert value == AString("hello")

  // Invalid tag
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("tag"), dynamic.string("dunno")),
        #(dynamic.string("the-string"), dynamic.string("hello")),
      ]),
      decoder,
    )
  assert value == [DecodeError("IntOrString", "Dict", [])]

  // Missing tag
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("the-string"), dynamic.string("hello")),
      ]),
      decoder,
    )
  assert value
    == [
      DecodeError("Field", "Nothing", ["tag"]),
      DecodeError("IntOrString", "Dict", []),
    ]

  // String invalid field
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("tag"), dynamic.string("string")),
        #(dynamic.string("the-string"), dynamic.float(12.3)),
      ]),
      decoder,
    )
  assert value == [DecodeError("String", "Float", ["the-string"])]
  Nil
}

pub type PocketMonsterType {
  Fire
  Water
  Grass
  Electric
}

pub fn documentation_enum_example_test() -> Nil {
  let decoder = {
    use decoded_string <- decode.then(decode.string)
    case decoded_string {
      // Return succeeding decoders for valid strings
      "fire" -> decode.success(Fire)
      "water" -> decode.success(Water)
      "grass" -> decode.success(Grass)
      "electric" -> decode.success(Electric)
      // Return a failing decoder for any other strings
      _ -> decode.failure(Fire, "PocketMonsterType")
    }
  }

  let assert Ok(value) = decode.run(dynamic.string("water"), decoder)
  assert value == Water

  let assert Error(value) = decode.run(dynamic.string("wobble"), decoder)
  assert value == [DecodeError("PocketMonsterType", "String", [])]
  Nil
}

pub type PocketMonsterPerson {
  Trainer(name: String, badge_count: Int)
  GymLeader(name: String, speciality: String)
}

pub fn documentation_variants_example_test() -> Nil {
  let trainer_decoder = {
    use name <- decode.field("name", decode.string)
    use badge_count <- decode.field("badge-count", decode.int)
    decode.success(Trainer(name, badge_count))
  }

  let gym_leader_decoder = {
    use name <- decode.field("name", decode.string)
    use speciality <- decode.field("speciality", decode.string)
    decode.success(GymLeader(name, speciality))
  }

  let decoder = {
    use tag <- decode.field("type", decode.string)
    case tag {
      "gym-leader" -> gym_leader_decoder
      _ -> trainer_decoder
    }
  }

  // Trainer
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("type"), dynamic.string("trainer")),
        #(dynamic.string("name"), dynamic.string("Ash")),
        #(dynamic.string("badge-count"), dynamic.int(8)),
      ]),
      decoder,
    )
  assert value == Trainer("Ash", 8)

  // Gym leader
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("type"), dynamic.string("gym-leader")),
        #(dynamic.string("name"), dynamic.string("Brock")),
        #(dynamic.string("speciality"), dynamic.string("Rock")),
      ]),
      decoder,
    )
  assert value == GymLeader("Brock", "Rock")

  // Error
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("type"), dynamic.string("gym-leader")),
        #(dynamic.string("name"), dynamic.string("Brock")),
      ]),
      decoder,
    )
  assert value
    == [DecodeError(expected: "Field", found: "Nothing", path: ["speciality"])]
  Nil
}

fn decode_string(data: Dynamic) -> Result(String, String) {
  decode.run(data, decode.string)
  |> result.replace_error("")
}

fn decode_float(data: Dynamic) -> Result(Float, Float) {
  decode.run(data, decode.float)
  |> result.replace_error(0.0)
}

pub fn new_primitive_decoder_string_ok_test() -> Nil {
  let decoder = decode.new_primitive_decoder("String", decode_string)
  let assert Ok(value) = decode.run(dynamic.string("Hello!"), decoder)
  assert value == "Hello!"
  Nil
}

pub fn new_primitive_decoder_string_error_test() -> Nil {
  let decoder = decode.new_primitive_decoder("String", decode_string)
  let assert Error(value) = decode.run(dynamic.int(123), decoder)
  assert value == [DecodeError("String", "Int", [])]
  Nil
}

pub fn new_primitive_decoder_float_ok_test() -> Nil {
  let decoder = decode.new_primitive_decoder("Float", decode_float)
  let assert Ok(value) = decode.run(dynamic.float(12.4), decoder)
  assert value == 12.4
  Nil
}

pub fn new_primitive_decoder_float_error_test() -> Nil {
  let decoder = decode.new_primitive_decoder("Float", decode_float)
  let assert Error(value) = decode.run(dynamic.string("blah"), decoder)
  assert value == [DecodeError("Float", "String", [])]
  Nil
}

pub type LinkedList {
  ListEmpty
  ListNonEmpty(element: Int, tail: LinkedList)
}

pub fn list_decoder() -> decode.Decoder(LinkedList) {
  use tag <- decode.field("type", decode.string)
  case tag {
    "list-non-empty" -> {
      use element <- decode.field("element", decode.int)
      use tail <- decode.field("tail", list_decoder())
      decode.success(ListNonEmpty(element: element, tail: tail))
    }
    _ -> decode.success(ListEmpty)
  }
}

pub fn recursive_data_structure_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(dynamic.string("type"), dynamic.string("list-non-empty")),
        #(dynamic.string("element"), dynamic.int(1)),
        #(
          dynamic.string("tail"),
          dynamic.properties([
            #(dynamic.string("type"), dynamic.string("list-non-empty")),
            #(dynamic.string("element"), dynamic.int(2)),
            #(
              dynamic.string("tail"),
              dynamic.properties([
                #(dynamic.string("type"), dynamic.string("list-empty")),
              ]),
            ),
          ]),
        ),
      ]),
      list_decoder(),
    )
  assert value == ListNonEmpty(1, ListNonEmpty(2, ListEmpty))
  Nil
}

pub fn optionally_at_dict_string_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(
          dynamic.string("first"),
          dynamic.properties([
            #(
              dynamic.string("second"),
              dynamic.properties([#(dynamic.string("third"), dynamic.int(1337))]),
            ),
          ]),
        ),
      ]),
      decode.optionally_at(["first", "second", "third"], 100, decode.int),
    )
  assert value == 1337
  Nil
}

pub fn optionally_at_dict_int_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(
          dynamic.int(10),
          dynamic.properties([
            #(
              dynamic.int(20),
              dynamic.properties([#(dynamic.int(30), dynamic.int(1337))]),
            ),
          ]),
        ),
      ]),
      decode.optionally_at([10, 20, 30], 123, decode.int),
    )
  assert value == 1337
  Nil
}

pub fn optionally_at_tuple_int_ok_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.array([
        dynamic.string("x"),
        dynamic.array(["a", "b", "c"] |> list.map(dynamic.string)),
        dynamic.string("z"),
      ]),
      decode.optionally_at([1, 0], "something", decode.string),
    )
  assert value == "a"
  Nil
}

pub fn optionally_at_wrong_inner_error_test() -> Nil {
  let assert Error(value) =
    decode.run(
      dynamic.properties([
        #(
          dynamic.string("first"),
          dynamic.properties([
            #(
              dynamic.string("second"),
              dynamic.properties([#(dynamic.string("third"), dynamic.int(1337))]),
            ),
          ]),
        ),
      ]),
      decode.optionally_at(
        ["first", "second", "third"],
        "default",
        decode.string,
      ),
    )
  assert value == [DecodeError("String", "Int", ["first", "second", "third"])]
  Nil
}

pub fn optionally_at_no_path_error_test() -> Nil {
  let assert Ok(value) =
    decode.run(
      dynamic.properties([
        #(
          dynamic.string("first"),
          dynamic.properties([#(dynamic.string("third"), dynamic.int(1337))]),
        ),
      ]),
      decode.optionally_at(["first", "second", "third"], 100, decode.int),
    )
  assert value == 100
  Nil
}

// Names are retained from the upstream host interoperability tests. On Wasm,
// property values cross the decoder boundary through the typed Dynamic API.
fn make_object(items: List(#(String, Int))) -> Dynamic {
  items
  |> list.map(fn(entry) {
    #(dynamic.string(entry.0), dynamic.int(entry.1))
  })
  |> dynamic.properties
}

fn make_map(items: List(#(String, Int))) -> Dynamic {
  // Reversing insertion order also checks unordered dictionary decoding.
  items |> list.reverse |> make_object
}

pub fn js_object_test() -> Nil {
  let assert Ok(value) =
    [#("a", 10), #("b", 20), #("c", 30)]
    |> make_object
    |> decode.run(decode.dict(decode.string, decode.int))
  assert value == dict.from_list([#("a", 10), #("b", 20), #("c", 30)])
  Nil
}

pub fn js_map_test() -> Nil {
  let assert Ok(value) =
    [#("a", 10), #("b", 20), #("c", 30)]
    |> make_map
    |> decode.run(decode.dict(decode.string, decode.int))
  assert value == dict.from_list([#("a", 10), #("b", 20), #("c", 30)])
  Nil
}

type Nested {
  Nested(List(Nested))
  Value(String)
}

fn recursive_decoder() -> decode.Decoder(Nested) {
  use <- decode.recursive()
  decode.one_of(decode.string |> decode.map(Value), [
    decode.list(recursive_decoder()) |> decode.map(Nested),
  ])
}

pub fn recursive_test() -> Nil {
  let expected =
    Nested([
      Nested([Value("one"), Value("two")]),
      Nested([Value("three")]),
      Nested([]),
    ])

  let assert Ok(value) =
    decode.run(
      dynamic.list(
        [
          ["one", "two"] |> list.map(dynamic.string),
          ["three"] |> list.map(dynamic.string),
          [],
        ]
        |> list.map(dynamic.list),
      ),
      recursive_decoder(),
    )
  assert value == expected
  Nil
}
