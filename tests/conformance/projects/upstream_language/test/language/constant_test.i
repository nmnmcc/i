// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

const const_int = 5

const const_float = 1.0

const const_string = "Gleam"

const const_nil = Nil

const const_ok = Ok(1)

const const_list_empty = []

const const_list_1 = [1]

const const_list_2 = [1, 2]

pub fn int_test() -> Nil {
  assert const_int == 5
  Nil
}

pub fn float_test() -> Nil {
  assert const_float == 1.0
  Nil
}

pub fn string_test() -> Nil {
  assert const_string == "Gleam"
  Nil
}

pub fn nil_test() -> Nil {
  assert const_nil == Nil
  Nil
}

pub fn ok_test() -> Nil {
  assert const_ok == Ok(1)
  Nil
}

pub fn list_empty_test() -> Nil {
  assert const_list_empty == []
  Nil
}

pub fn list_1_test() -> Nil {
  assert const_list_1 == [1]
  Nil
}

pub fn list_2_test() -> Nil {
  assert const_list_2 == [1, 2]
  Nil
}

const message = "hello"

// https://github.com/gleam-lang/gleam/issues/6045
pub fn constant_string_in_bit_array_segment_test() -> Nil {
  assert <<"hello">> == <<message:utf8>>
  assert <<"hello":utf8>> == <<message:utf8>>
  Nil
}

const message_2 = message

const message_3 = message_2 <> "?"

// https://github.com/gleam-lang/gleam/issues/6086
pub fn constant_aliased_string_in_bit_arrays_test() -> Nil {
  assert <<"hello">> == <<message_2:utf8>>
  assert <<"hello":utf8>> == <<message_2:utf8>>
  assert "hello!" == message_2 <> "!"
  assert "hello?!" == message_3 <> "!"
  let subject = "hello!"
  assert case subject {
    wibble if wibble == message_2 <> "!" -> True
    _ -> False
  }
  Nil
}
