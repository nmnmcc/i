//// Internal UTF-8 string operations shared by string and string_tree.
//// Keeping this module independent of those public modules avoids an import cycle.

import gleam/wleam_unicode as unicode

@external(wasm, "wleam:runtime/v1", "string_byte_size")
pub fn byte_size(string: String) -> Int

@external(wasm, "wleam:runtime/v1", "string_byte_slice")
pub fn byte_slice(string: String, index: Int, length: Int) -> String

@external(wasm, "wleam:runtime/v1", "string_concat")
pub fn concat(strings: List(String)) -> String

pub fn less_than(a: String, b: String) -> Bool {
  compare_codepoints(unicode.codepoints(a), unicode.codepoints(b))
}

fn compare_codepoints(a: List(Int), b: List(Int)) -> Bool {
  case a, b {
    [], [] -> False
    [], _ -> True
    _, [] -> False
    [x, ..xs], [y, ..ys] ->
      case x == y {
        True -> compare_codepoints(xs, ys)
        False -> x < y
      }
  }
}

fn prefix(rest: List(Int), needle: List(Int)) -> Bool {
  case rest, needle {
    _, [] -> True
    [], _ -> False
    [x, ..xs], [y, ..ys] -> x == y && prefix(xs, ys)
  }
}

pub fn starts_with(string: String, needle: String) -> Bool {
  prefix(unicode.codepoints(string), unicode.codepoints(needle))
}

pub fn ends_with(string: String, needle: String) -> Bool {
  prefix(
    reverse(unicode.codepoints(string), []),
    reverse(unicode.codepoints(needle), []),
  )
}

pub fn contains(string: String, needle: String) -> Bool {
  case find(unicode.codepoints(string), unicode.codepoints(needle), []) {
    Ok(_) -> True
    Error(_) -> False
  }
}

pub fn crop(string: String, needle: String) -> String {
  case find(unicode.codepoints(string), unicode.codepoints(needle), []) {
    Ok(#(_, rest)) -> unicode.from_codepoints(rest)
    Error(_) -> string
  }
}

// The prefix is accumulated in reverse order; the suffix starts at the match.
fn find(
  rest: List(Int),
  needle: List(Int),
  before: List(Int),
) -> Result(#(List(Int), List(Int)), Nil) {
  case prefix(rest, needle) {
    True -> Ok(#(before, rest))
    False ->
      case rest {
        [] -> Error(Nil)
        [first, ..tail] -> find(tail, needle, [first, ..before])
      }
  }
}

fn skip_prefix(rest: List(Int), prefix: List(Int)) -> List(Int) {
  case rest, prefix {
    _, [] -> rest
    [], _ -> []
    [_, ..tail], [_, ..remaining] -> skip_prefix(tail, remaining)
  }
}

pub fn split_once(
  string: String,
  needle: String,
) -> Result(#(String, String), Nil) {
  case needle {
    "" -> Error(Nil)
    _ -> {
      let needle = unicode.codepoints(needle)
      case find(unicode.codepoints(string), needle, []) {
        Error(_) -> Error(Nil)
        Ok(#(before, rest)) ->
          Ok(#(
            unicode.from_codepoints(reverse(before, [])),
            unicode.from_codepoints(skip_prefix(rest, needle)),
          ))
      }
    }
  }
}

// Empty separators leave the original string intact, matching the tree API.
// The public string.split API handles its documented grapheme split separately.
pub fn split(string: String, needle: String) -> List(String) {
  case needle {
    "" -> [string]
    _ ->
      split_loop(unicode.codepoints(string), unicode.codepoints(needle), [], [])
  }
}

fn split_loop(
  rest: List(Int),
  needle: List(Int),
  current: List(Int),
  parts: List(String),
) -> List(String) {
  case prefix(rest, needle) {
    True -> {
      let part = unicode.from_codepoints(reverse(current, []))
      split_loop(skip_prefix(rest, needle), needle, [], [part, ..parts])
    }
    False ->
      case rest {
        [] -> {
          let part = unicode.from_codepoints(reverse(current, []))
          reverse([part, ..parts], [])
        }
        [first, ..tail] -> split_loop(tail, needle, [first, ..current], parts)
      }
  }
}

pub fn replace(string: String, needle: String, replacement: String) -> String {
  case split(string, needle) {
    [] -> ""
    [first, ..rest] ->
      concat(reverse(intersperse(rest, replacement, [first]), []))
  }
}

fn intersperse(
  parts: List(String),
  separator: String,
  acc: List(String),
) -> List(String) {
  case parts {
    [] -> acc
    [part, ..rest] -> intersperse(rest, separator, [part, separator, ..acc])
  }
}

pub fn trim_start(string: String) -> String {
  unicode.from_codepoints(trim_codepoints(unicode.codepoints(string)))
}

pub fn trim_end(string: String) -> String {
  string
  |> unicode.codepoints
  |> reverse([])
  |> trim_codepoints
  |> reverse([])
  |> unicode.from_codepoints
}

fn trim_codepoints(codepoints: List(Int)) -> List(Int) {
  case codepoints {
    [first, ..rest] ->
      case is_trim_whitespace(first) {
        True -> trim_codepoints(rest)
        False -> codepoints
      }
    [] -> []
  }
}

// The nine whitespace scalars used by the Gleam 1.0.5 implementations.
// Directional marks and nonbreaking spaces are deliberately retained.
fn is_trim_whitespace(codepoint: Int) -> Bool {
  case codepoint {
    9 | 10 | 11 | 12 | 13 | 32 | 133 | 8232 | 8233 -> True
    _ -> False
  }
}

pub fn remove_prefix(string: String, needle: String) -> String {
  case starts_with(string, needle) {
    True ->
      byte_slice(
        string,
        byte_size(needle),
        byte_size(string) - byte_size(needle),
      )
    False -> string
  }
}

pub fn remove_suffix(string: String, needle: String) -> String {
  case ends_with(string, needle) {
    True -> byte_slice(string, 0, byte_size(string) - byte_size(needle))
    False -> string
  }
}

fn reverse(list: List(a), acc: List(a)) -> List(a) {
  case list {
    [] -> acc
    [first, ..rest] -> reverse(rest, [first, ..acc])
  }
}
