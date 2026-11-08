import i/string
import i/string_tree

pub fn scalar_search_and_prefix_test() {
  let text = "a💜e\u{0301}z"
  assert string.contains(text, "💜e\u{0301}")
  assert !string.contains(text, "💜é")
  assert string.starts_with(text, "a💜")
  assert string.ends_with(text, "e\u{0301}z")
  assert string.crop(text, "💜") == "💜e\u{0301}z"
  assert string.crop(text, "missing") == text
  assert string.remove_prefix(text, "a💜") == "e\u{0301}z"
  assert string.remove_suffix(text, "e\u{0301}z") == "a💜"
}

pub fn utf8_splitting_test() {
  assert string.split("💜::α::", "::") == ["💜", "α", ""]
  assert string.split_once("💜::α::", "::") == Ok(#("💜", "α::"))
  assert string.split_once("💜", "::") == Error(Nil)
  assert string.replace("💜::α::", "::", "→") == "💜→α→"
}

pub fn empty_separator_contract_test() {
  assert string.split("e\u{0301}💜", "") == ["e\u{0301}", "💜"]
  assert string.split("", "") == []
  assert string.split_once("abc", "") == Error(Nil)
  assert string.replace("abc", "", "-") == "abc"
  let tree = string_tree.from_strings(["a", "bc"])
  assert string_tree.split(tree, "") == [tree]
}

pub fn grapheme_slice_boundaries_test() {
  let text = "a👩🏾‍❤️‍👨🏻e\u{0301}z"
  assert string.length(text) == 4
  assert string.slice(text, 1, 2) == "👩🏾‍❤️‍👨🏻e\u{0301}"
  assert string.slice(text, -2, 1) == "e\u{0301}"
  assert string.slice(text, -20, 1) == ""
  assert string.slice(text, 20, 1) == ""
  assert string.slice(text, 1, -1) == ""
  assert string.drop_start(text, 2) == "e\u{0301}z"
  assert string.drop_end(text, 2) == "a👩🏾‍❤️‍👨🏻"
  assert string.drop_start(text, 20) == ""
  assert string.drop_end(text, 20) == ""
}

pub fn scalar_conversion_test() {
  let source = "A\u{0000}💜\u{10FFFF}"
  assert source |> string.to_utf_codepoints |> string.from_utf_codepoints
    == source
  assert string.utf_codepoint(-1) == Error(Nil)
  assert string.utf_codepoint(0xD800) == Error(Nil)
  assert string.utf_codepoint(0xDFFF) == Error(Nil)
  assert string.utf_codepoint(0x110000) == Error(Nil)
  let assert Ok(last) = string.utf_codepoint(0x10FFFF)
  assert string.utf_codepoint_to_int(last) == 0x10FFFF
}

pub fn whitespace_contract_test() {
  let whitespace =
    "\u{0009}\u{000A}\u{000B}\u{000C}\u{000D} \u{0085}\u{2028}\u{2029}"
  assert string.trim(whitespace <> "💜" <> whitespace) == "💜"
  assert string.trim("\u{00A0}💜\u{2003}") == "\u{00A0}💜\u{2003}"
  assert string.trim("\u{200E}💜\u{200F}") == "\u{200E}💜\u{200F}"
}

pub fn tree_persistence_and_flatten_test() {
  let original = string_tree.from_strings(["a", "💜"])
  let expanded = original |> string_tree.append("b") |> string_tree.prepend("z")
  assert string_tree.to_string(original) == "a💜"
  assert string_tree.to_string(expanded) == "za💜b"
  assert string_tree.byte_size(expanded) == 7
  assert string_tree.is_equal(original, string_tree.from_string("a💜"))
  assert !string_tree.is_empty(original)
  assert string_tree.is_empty(string_tree.from_strings(["", ""]))
}

pub fn tree_split_preserves_chunks_test() {
  let source = string_tree.from_strings(["Gleam, Erl", "ang,Elixir"])
  assert string_tree.split(source, ", ")
    == [
      string_tree.from_string("Gleam"),
      string_tree.from_strings(["Erl", "ang,Elixir"]),
    ]
}

pub fn tree_split_across_node_boundary_test() {
  let source =
    string_tree.from_string("a💜")
    |> string_tree.append_tree(string_tree.from_strings(["::", "b::", "c"]))
  let assert [a, b, c] = string_tree.split(source, "::")
  assert string_tree.to_string(a) == "a💜"
  assert string_tree.to_string(b) == "b"
  assert string_tree.to_string(c) == "c"
  let crossed = string_tree.from_strings(["left:", ":right"])
  let assert [left, right] = string_tree.split(crossed, "::")
  assert string_tree.to_string(left) == "left"
  assert string_tree.to_string(right) == "right"
}

pub fn tree_unicode_across_chunks_test() {
  assert string_tree.from_strings(["Ο", "Σ"])
    |> string_tree.lowercase
    |> string_tree.to_string
    == "ος"
  assert string_tree.from_strings(["e", "\u{0301}", "💜"])
    |> string_tree.reverse
    |> string_tree.to_string
    == "💜e\u{0301}"
}

pub fn deep_tree_test() {
  let tree = append_many(10_000, string_tree.new())
  assert string_tree.byte_size(tree) == 10_000
  assert string_tree.to_string(tree) == string.repeat("x", 10_000)
}

fn append_many(
  count: Int,
  tree: string_tree.StringTree,
) -> string_tree.StringTree {
  case count {
    0 -> tree
    _ -> append_many(count - 1, string_tree.append(tree, "x"))
  }
}
