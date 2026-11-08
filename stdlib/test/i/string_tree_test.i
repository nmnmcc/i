import i/string_tree

pub fn string_tree_to_string_test() -> Nil {
  let data =
    string_tree.from_string("ello")
    |> string_tree.append(",")
    |> string_tree.append(" world!")
    |> string_tree.prepend("H")

  assert string_tree.to_string(data) == "Hello, world!"
  Nil
}

pub fn string_tree_byte_size_test() -> Nil {
  let data =
    string_tree.from_string("ello")
    |> string_tree.append(",")
    |> string_tree.append(" world!")
    |> string_tree.prepend("H")

  assert string_tree.byte_size(data) == 13
  Nil
}

pub fn string_tree_with_trees_to_string_test() -> Nil {
  let data =
    string_tree.from_string("ello")
    |> string_tree.append_tree(string_tree.from_string(","))
    |> string_tree.append_tree(
      string_tree.concat([
        string_tree.from_string(" wo"),
        string_tree.from_string("rld!"),
      ]),
    )
    |> string_tree.prepend_tree(string_tree.from_string("H"))

  assert string_tree.to_string(data) == "Hello, world!"
  Nil
}

pub fn string_tree_with_trees_byte_size_test() -> Nil {
  let data =
    string_tree.from_string("ello")
    |> string_tree.append_tree(string_tree.from_string(","))
    |> string_tree.append_tree(
      string_tree.concat([
        string_tree.from_string(" wo"),
        string_tree.from_string("rld!"),
      ]),
    )
    |> string_tree.prepend_tree(string_tree.from_string("H"))

  assert string_tree.byte_size(data) == 13
  Nil
}

pub fn reverse_roundtrip_test() -> Nil {
  assert "Ĺo͂řȩm̅"
    |> string_tree.from_string
    |> string_tree.reverse
    |> string_tree.reverse
    |> string_tree.to_string
    == "Ĺo͂řȩm̅"
  Nil
}

pub fn reverse_test() -> Nil {
  assert "Ĺo͂řȩm̅"
    |> string_tree.from_string
    |> string_tree.reverse
    |> string_tree.to_string
    == "m̅ȩřo͂Ĺ"
  Nil
}

pub fn reverse_emoji_roundtrip_test() -> Nil {
  assert "👶🏿"
    |> string_tree.from_string
    |> string_tree.reverse
    |> string_tree.reverse
    |> string_tree.to_string
    == "👶🏿"
  Nil
}

pub fn reverse_emoji_test() -> Nil {
  assert "👶🏿"
    |> string_tree.from_string
    |> string_tree.reverse
    |> string_tree.to_string
    == "👶🏿"
  Nil
}

pub fn lowercase_test() -> Nil {
  assert ["Gleam", "Gleam"]
    |> string_tree.from_strings
    |> string_tree.lowercase
    |> string_tree.to_string
    == "gleamgleam"
  Nil
}

pub fn uppercase_test() -> Nil {
  assert ["Gleam", "Gleam"]
    |> string_tree.from_strings
    |> string_tree.uppercase
    |> string_tree.to_string
    == "GLEAMGLEAM"
  Nil
}

pub fn split_simple_test() -> Nil {
  assert "Gleam,Erlang,Elixir"
    |> string_tree.from_string
    |> string_tree.split(",")
    == [
      string_tree.from_string("Gleam"),
      string_tree.from_string("Erlang"),
      string_tree.from_string("Elixir"),
    ]
  Nil
}

pub fn split_multi_string_test() -> Nil {
  assert ["Gleam, Erl", "ang,Elixir"]
    |> string_tree.from_strings
    |> string_tree.split(", ")
    == [
      string_tree.from_string("Gleam"),
      string_tree.from_strings(["Erl", "ang,Elixir"]),
    ]
  Nil
}

pub fn is_equal_different_structure_test() -> Nil {
  assert string_tree.is_equal(
    string_tree.from_string("12"),
    string_tree.from_strings(["1", "2"]),
  )
  Nil
}

pub fn is_equal_same_structure_test() -> Nil {
  assert string_tree.is_equal(
    string_tree.from_string("12"),
    string_tree.from_string("12"),
  )
  Nil
}

pub fn is_equal_different_content_test() -> Nil {
  assert !string_tree.is_equal(
    string_tree.from_string("12"),
    string_tree.from_string("2"),
  )
  Nil
}

pub fn is_empty_empty_string_test() -> Nil {
  assert string_tree.is_empty(string_tree.from_string(""))
  Nil
}

pub fn is_empty_non_empty_test() -> Nil {
  assert !string_tree.is_empty(string_tree.from_string("12"))
  Nil
}

pub fn is_empty_empty_list_test() -> Nil {
  assert string_tree.is_empty(string_tree.from_strings([]))
  Nil
}

pub fn is_empty_list_of_empty_strings_test() -> Nil {
  assert string_tree.is_empty(string_tree.from_strings(["", ""]))
  Nil
}

pub fn new_test() -> Nil {
  assert string_tree.to_string(string_tree.new()) == ""
  Nil
}

pub fn join_test() -> Nil {
  assert [
      string_tree.from_string("Gleam"),
      string_tree.from_string("Elixir"),
      string_tree.from_string("Erlang"),
    ]
    |> string_tree.join(", ")
    |> string_tree.to_string
    == "Gleam, Elixir, Erlang"
  Nil
}
