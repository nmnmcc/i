import i/bytes_tree
import i/string_tree

pub fn tree_to_bit_array_test() -> Nil {
  let data =
    bytes_tree.from_bit_array(<<1>>)
    |> bytes_tree.append(<<2>>)
    |> bytes_tree.append(<<3>>)
    |> bytes_tree.prepend(<<0>>)

  assert bytes_tree.to_bit_array(data) == <<0, 1, 2, 3>>
  Nil
}

pub fn tree_byte_size_test() -> Nil {
  let data =
    bytes_tree.from_bit_array(<<1>>)
    |> bytes_tree.append(<<2>>)
    |> bytes_tree.append(<<3>>)
    |> bytes_tree.prepend(<<0>>)

  assert bytes_tree.byte_size(data) == 4
  Nil
}

pub fn tree_unaligned_to_bit_array_test() -> Nil {
  let data =
    bytes_tree.from_bit_array(<<-1:5>>)
    |> bytes_tree.append(<<-1:3>>)
    |> bytes_tree.append(<<-2:2>>)
    |> bytes_tree.prepend(<<-1:4>>)

  assert bytes_tree.to_bit_array(data)
    == <<-1:4, 0:4, -1:5, 0:3, -1:3, 0:5, -2:2, 0:6>>
  Nil
}

pub fn tree_unaligned_byte_size_test() -> Nil {
  let data =
    bytes_tree.from_bit_array(<<-1:5>>)
    |> bytes_tree.append(<<-1:3>>)
    |> bytes_tree.append(<<-2:2>>)
    |> bytes_tree.prepend(<<-1:4>>)

  assert bytes_tree.byte_size(data) == 4
  Nil
}

pub fn tree_with_strings_to_bit_array_test() -> Nil {
  let data =
    bytes_tree.from_bit_array(<<1>>)
    |> bytes_tree.append_string("2")
    |> bytes_tree.append_string("3")
    |> bytes_tree.prepend_string("0")

  assert bytes_tree.to_bit_array(data) == <<"0":utf8, 1, "2":utf8, "3":utf8>>
  Nil
}

pub fn tree_with_strings_byte_size_test() -> Nil {
  let data =
    bytes_tree.from_bit_array(<<1>>)
    |> bytes_tree.append_string("2")
    |> bytes_tree.append_string("3")
    |> bytes_tree.prepend_string("0")

  assert bytes_tree.byte_size(data) == 4
  Nil
}

pub fn tree_with_trees_to_bit_array_test() -> Nil {
  let data =
    bytes_tree.from_bit_array(<<1>>)
    |> bytes_tree.append_tree(bytes_tree.from_bit_array(<<2>>))
    |> bytes_tree.append_tree(bytes_tree.from_bit_array(<<3>>))
    |> bytes_tree.prepend_tree(bytes_tree.from_bit_array(<<0>>))

  assert bytes_tree.to_bit_array(data) == <<0, 1, 2, 3>>
  Nil
}

pub fn tree_with_trees_byte_size_test() -> Nil {
  let data =
    bytes_tree.from_bit_array(<<1>>)
    |> bytes_tree.append_tree(bytes_tree.from_bit_array(<<2>>))
    |> bytes_tree.append_tree(bytes_tree.from_bit_array(<<3>>))
    |> bytes_tree.prepend_tree(bytes_tree.from_bit_array(<<0>>))

  assert bytes_tree.byte_size(data) == 4
  Nil
}

pub fn concat_test() -> Nil {
  assert [
      bytes_tree.from_bit_array(<<1, 2>>),
      bytes_tree.from_bit_array(<<3, 4>>),
      bytes_tree.from_bit_array(<<5, 6>>),
    ]
    |> bytes_tree.concat
    |> bytes_tree.to_bit_array
    == <<1, 2, 3, 4, 5, 6>>
  Nil
}

pub fn concat_bit_arrays_test() -> Nil {
  assert bytes_tree.to_bit_array(
      bytes_tree.concat_bit_arrays([<<"h":utf8>>, <<"e":utf8>>, <<"y":utf8>>]),
    )
    == <<"hey":utf8>>
  Nil
}

pub fn concat_unaligned_bit_arrays_test() -> Nil {
  assert bytes_tree.to_bit_array(
      bytes_tree.concat_bit_arrays([<<-1:4>>, <<-1:5>>, <<-1:3>>, <<-2:2>>]),
    )
    == <<-1:4, 0:4, -1:5, 0:3, -1:3, 0:5, -2:2, 0:6>>
  Nil
}

pub fn from_bit_array_empty_test() -> Nil {
  // Regression test: no additional modification of the tree
  assert bytes_tree.to_bit_array(bytes_tree.from_bit_array(<<>>)) == <<>>
  Nil
}

pub fn from_string_empty_test() -> Nil {
  // Regression test: no additional modification of the tree
  assert bytes_tree.to_bit_array(bytes_tree.from_string("")) == <<>>
  Nil
}

pub fn new_test() -> Nil {
  assert bytes_tree.to_bit_array(bytes_tree.new()) == <<>>
  Nil
}

pub fn from_string_tree_test() -> Nil {
  assert bytes_tree.from_string_tree(string_tree.from_string("hello"))
    == bytes_tree.from_string("hello")
  Nil
}
