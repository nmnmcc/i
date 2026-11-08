// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

fn id(string: String) -> String {
  string
}

pub fn empty_strings_equal_test() -> Nil {
  assert id("") == id("")
  Nil
}

pub fn newlines_equal_test() -> Nil {
  assert id(
      "
",
    )
    == id("\n")
  Nil
}

pub fn let_assert_string_prefix_test() -> Nil {
  let assert "ab" <> rest = "abcdef"
  assert "cdef" == rest
  Nil
}

pub fn concat_block_single_test() -> Nil {
  let name = id("Lucy")
  let result = name <> { "/" } <> name
  assert result == "Lucy/Lucy"
  Nil
}

pub fn concat_block_multiple_test() -> Nil {
  let name = id("Lucy")
  let result =
    name
    <> {
      let x = "/"
      x
    }
    <> name
  assert result == "Lucy/Lucy"
  Nil
}
