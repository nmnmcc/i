//// Functions for `String`, the type of lists of Unicode scalar values kept
//// as UTF-8.
////
//// Lengths, indexes, and slices count **codepoints** (Unicode scalar
//// values), not bytes and not graphemes: `"é"` written as `e` and a
//// combining accent is two codepoints. `byte_size` counts UTF-8 bytes.
////
//// `to_upper`, `to_lower`, and `capitalise` change only the ASCII letters
//// `a` to `z` and `A` to `Z`; other characters stay as they are. `trim` and
//// the other trim functions take out the characters of Unicode
//// Pattern_White_Space: U+0009 to U+000D, U+0020, U+0085, U+200E, U+200F,
//// U+2028, and U+2029.

import (
  i/core
  list = std/list
  option = (Option, Some, None) = std/option
  order = (Order, Less, Equal, Greater) = std/order
)

export (
  length
  byte_size
  is_empty
  concat
  append
  join
  repeat
  split
  split_once
  lines
  trim
  trim_start
  trim_end
  starts_with
  ends_with
  contains
  replace
  slice
  drop_start
  drop_end
  first
  last
  pop_codepoint
  crop
  remove_prefix
  remove_suffix
  to_upper
  to_lower
  capitalise
  reverse
  pad_left
  pad_right
  to_codepoints
  from_codepoints
  to_codepoint_strings
  utf_codepoint
  utf_codepoint_to_int
  compare
  inspect
  to_option
)

/// The number of codepoints.
///
/// ```i
/// string.length("héllo")  // 5
/// ```
length = (string: String) -> Int { list.length(core.codepoints(string)) }

/// The number of UTF-8 bytes.
///
/// ```i
/// string.byte_size("héllo")  // 6
/// ```
byte_size = (string: String) -> Int { core.byte_size(string) }

/// `True` for `""`.
is_empty = (string: String) -> Bool { string == "" }

/// The strings one after the other.
concat = (strings: List(String)) -> String { core.concat(strings) }

/// `to` and then `suffix`, as a function.
append = (to: String, suffix: String) -> String { to <> suffix }

/// The strings with `with` between every two of them.
///
/// ```i
/// string.join(["a", "b", "c"], with: ", ")  // "a, b, c"
/// ```
join = (strings: List(String), with: String) -> String {
  core.concat(list.intersperse(strings, with))
}

/// The string `times` times, one after the other. `""` when `times` is 0 or
/// less.
repeat = (string: String, times: Int) -> String {
  core.concat(list.repeat(string, times))
}

/// Cuts a string at every place where `on` is, and takes out `on`. With
/// `on` as `""`, it cuts between every two codepoints.
///
/// ```i
/// string.split("a,b,,c", on: ",")  // ["a", "b", "", "c"]
/// string.split("abc", on: "")      // ["a", "b", "c"]
/// ```
split = (string: String, on: String) -> List(String) {
  case on {
    "" -> to_codepoint_strings(string)
    _ -> {
      needle = core.codepoints(on)
      go = (rest, current, acc) {
        case rest {
          [] -> list.reverse([from_reversed(current), ..acc])
          [c, ..cs] -> case strip_prefix(rest, needle) {
            Some(after) -> go(after, [], [from_reversed(current), ..acc])
            None -> go(cs, [c, ..current], acc)
          }
        }
      }
      go(core.codepoints(string), [], [])
    }
  }
}

from_reversed = (codepoints: List(UtfCodepoint)) -> String {
  core.from_codepoints(list.reverse(codepoints))
}

// The rest of `list` after `prefix`, when `list` starts with `prefix`.
strip_prefix = (list: List(UtfCodepoint), prefix: List(UtfCodepoint)) -> Option(List(UtfCodepoint)) {
  case prefix {
    [] -> Some(list)
    [p, ..ps] -> case list {
      [c, ..cs] if c == p -> strip_prefix(cs, ps)
      _ -> None
    }
  }
}

/// Cuts a string in two at the first place where `on` is, and takes out
/// `on`. `None` when `on` is not in the string, or is `""`.
///
/// ```i
/// string.split_once("key=value=x", on: "=")  // Some(("key", "value=x"))
/// ```
split_once = (string: String, on: String) -> Option((String, String)) {
  case on {
    "" -> None
    _ -> {
      needle = core.codepoints(on)
      go = (rest, current) {
        case rest {
          [] -> None
          [c, ..cs] -> case strip_prefix(rest, needle) {
            Some(after) -> Some((from_reversed(current), core.from_codepoints(after)))
            None -> go(cs, [c, ..current])
          }
        }
      }
      go(core.codepoints(string), [])
    }
  }
}

/// Cuts a string into lines, at `"\n"` and at `"\r\n"`.
///
/// ```i
/// string.lines("a\nb\r\nc")  // ["a", "b", "c"]
/// ```
lines = (string: String) -> List(String) {
  list.map(split(string, "\n"), (line) {
    case ends_with(line, "\r") {
      True -> drop_end(line, 1)
      False -> line
    }
  })
}

is_space = (c: UtfCodepoint) -> Bool {
  n = core.codepoint_to_int(c)
  { n >= 9 && n <= 13 } || n == 32 || n == 133 || n == 8206 || n == 8207 || n == 8232 || n == 8233
}

/// The string without white space at the start and at the end.
///
/// ```i
/// string.trim("  hi \n")  // "hi"
/// ```
trim = (string: String) -> String { trim_end(trim_start(string)) }

/// The string without white space at the start.
trim_start = (string: String) -> String {
  core.from_codepoints(list.drop_while(core.codepoints(string), is_space))
}

/// The string without white space at the end.
trim_end = (string: String) -> String {
  from_reversed(list.drop_while(list.reverse(core.codepoints(string)), is_space))
}

/// `True` when the string starts with `prefix`.
starts_with = (string: String, prefix: String) -> Bool {
  core.byte_slice(string, 0, core.byte_size(prefix)) == Ok(prefix)
}

/// `True` when the string ends with `suffix`.
ends_with = (string: String, suffix: String) -> Bool {
  n = core.byte_size(suffix)
  core.byte_slice(string, core.byte_size(string) - n, n) == Ok(suffix)
}

/// `True` when `contain` is somewhere in the string. Every string contains
/// `""`.
///
/// ```i
/// string.contains(does: "theory", contain: "ory")  // True
/// ```
contains = (does: String, contain: String) -> Bool {
  m = core.byte_size(contain)
  last = core.byte_size(does) - m
  go = (i) {
    case i > last {
      True -> False
      False -> case core.byte_slice(does, i, m) == Ok(contain) {
        True -> True
        False -> go(i + 1)
      }
    }
  }
  go(0)
}

/// Puts `with` in the place of every `each` in the string. When `each` is
/// `""`, the string is the same.
///
/// ```i
/// string.replace(in: "www.example.com", each: ".", with: "-")  // "www-example-com"
/// ```
replace = (in: String, each: String, with: String) -> String {
  case each {
    "" -> in
    _ -> join(split(in, each), with)
  }
}

/// `length` codepoints from the codepoint at `at_index`, counting from 0.
/// A negative index counts from the end: `-1` is the last codepoint. The
/// slice stops at the end of the string. A negative length gives `""`.
///
/// ```i
/// string.slice("gleam", at_index: 1, length: 2)   // "le"
/// string.slice("gleam", at_index: -3, length: 2)  // "ea"
/// ```
slice = (string: String, at_index: Int, length: Int) -> String {
  case length <= 0 {
    True -> ""
    False -> {
      codepoints = core.codepoints(string)
      start = case at_index < 0 {
        True -> list.length(codepoints) + at_index
        False -> at_index
      }
      case start < 0 {
        True -> ""
        False -> core.from_codepoints(list.take(list.drop(codepoints, start), length))
      }
    }
  }
}

/// The string without its first `up_to` codepoints.
drop_start = (from: String, up_to: Int) -> String {
  core.from_codepoints(list.drop(core.codepoints(from), up_to))
}

/// The string without its last `up_to` codepoints.
drop_end = (from: String, up_to: Int) -> String {
  from_reversed(list.drop(list.reverse(core.codepoints(from)), up_to))
}

/// The first codepoint, as a string, or `None` for `""`.
first = (string: String) -> Option(String) {
  case core.codepoints(string) {
    [] -> None
    [c, ..] -> Some(core.from_codepoints([c]))
  }
}

/// The last codepoint, as a string, or `None` for `""`.
last = (string: String) -> Option(String) {
  case list.last(core.codepoints(string)) {
    Some(c) -> Some(core.from_codepoints([c]))
    None -> None
  }
}

/// The first codepoint, as a string, and the rest, or `None` for `""`.
///
/// ```i
/// string.pop_codepoint("abc")  // Some(("a", "bc"))
/// ```
pop_codepoint = (string: String) -> Option((String, String)) {
  case core.codepoints(string) {
    [] -> None
    [c, ..cs] -> Some((core.from_codepoints([c]), core.from_codepoints(cs)))
  }
}

/// The string from the first place where `before` is. When `before` is not
/// there, the string is the same.
///
/// ```i
/// string.crop("The Lone Gleamer", before: "Lone")  // "Lone Gleamer"
/// ```
crop = (from: String, before: String) -> String {
  case split_once(from, before) {
    Some((_, after)) -> before <> after
    None -> from
  }
}

/// The string without `prefix` at its start. When it does not start with
/// `prefix`, the string is the same.
///
/// ```i
/// string.remove_prefix("@lpil", "@")  // "lpil"
/// ```
remove_prefix = (from: String, matching: String) -> String {
  n = core.byte_size(matching)
  case starts_with(from, matching) {
    True -> case core.byte_slice(from, n, core.byte_size(from) - n) {
      Ok(rest) -> rest
      Error(_) -> from
    }
    False -> from
  }
}

/// The string without `suffix` at its end. When it does not end with
/// `suffix`, the string is the same.
remove_suffix = (from: String, matching: String) -> String {
  n = core.byte_size(from) - core.byte_size(matching)
  case ends_with(from, matching) {
    True -> case core.byte_slice(from, 0, n) {
      Ok(rest) -> rest
      Error(_) -> from
    }
    False -> from
  }
}

map_ascii = (string: String, from: Int, to: Int, shift: Int) -> String {
  core.from_codepoints(list.map(core.codepoints(string), (c) {
    n = core.codepoint_to_int(c)
    case n >= from && n <= to {
      True -> case core.int_to_codepoint(n + shift) {
        Ok(d) -> d
        Error(_) -> c
      }
      False -> c
    }
  }))
}

/// The string with the ASCII letters `a` to `z` in upper case. Other
/// characters, also other letters, stay as they are.
///
/// ```i
/// string.to_upper("héllo")  // "HéLLO"
/// ```
to_upper = (string: String) -> String { map_ascii(string, 97, 122, -32) }

/// The string with the ASCII letters `A` to `Z` in lower case. Other
/// characters, also other letters, stay as they are.
to_lower = (string: String) -> String { map_ascii(string, 65, 90, 32) }

/// The first codepoint in upper case and the rest in lower case, for ASCII
/// letters only.
///
/// ```i
/// string.capitalise("mamouna")  // "Mamouna"
/// ```
capitalise = (string: String) -> String {
  case pop_codepoint(string) {
    Some((head, tail)) -> to_upper(head) <> to_lower(tail)
    None -> ""
  }
}

/// The codepoints in the other order.
///
/// ```i
/// string.reverse("abc")  // "cba"
/// ```
reverse = (string: String) -> String { from_reversed(core.codepoints(string)) }

// `length` codepoints of `with`, repeated as needed.
padding = (length: Int, with: String) -> String {
  pad = core.codepoints(with)
  case pad {
    [] -> ""
    _ -> {
      go = (n, rest, acc) {
        case n <= 0 {
          True -> from_reversed(acc)
          False -> case rest {
            [] -> go(n, pad, acc)
            [c, ..cs] -> go(n - 1, cs, [c, ..acc])
          }
        }
      }
      go(length, pad, [])
    }
  }
}

/// Puts `with`, repeated, before the string until it has `to` codepoints.
/// The last copy of `with` may be cut. A string that is long enough stays.
///
/// ```i
/// string.pad_left("121", to: 5, with: ".")  // "..121"
/// string.pad_left("121", to: 6, with: "XY")  // "XYX121"
/// ```
pad_left = (string: String, to: Int, with: String) -> String {
  padding(to - length(string), with) <> string
}

/// Puts `with`, repeated, after the string until it has `to` codepoints.
/// The last copy of `with` may be cut. A string that is long enough stays.
pad_right = (string: String, to: Int, with: String) -> String {
  string <> padding(to - length(string), with)
}

/// The codepoints of the string, in order.
to_codepoints = (string: String) -> List(UtfCodepoint) { core.codepoints(string) }

/// The string of these codepoints.
from_codepoints = (codepoints: List(UtfCodepoint)) -> String { core.from_codepoints(codepoints) }

/// Every codepoint as a string of its own.
///
/// ```i
/// string.to_codepoint_strings("hé")  // ["h", "é"]
/// ```
to_codepoint_strings = (string: String) -> List(String) {
  list.map(core.codepoints(string), (c) { core.from_codepoints([c]) })
}

/// The codepoint with this number, or `Error(Nil)` when it is not a Unicode
/// scalar value (0 to 0x10FFFF, but not 0xD800 to 0xDFFF).
utf_codepoint = (value: Int) -> Result(UtfCodepoint, Nil) { core.int_to_codepoint(value) }

/// The number of a codepoint.
utf_codepoint_to_int = (codepoint: UtfCodepoint) -> Int { core.codepoint_to_int(codepoint) }

/// Compares two strings by their codepoints, one by one. This is also the
/// order of their UTF-8 bytes.
///
/// ```i
/// string.compare("Anthony", "Anthony")  // Equal
/// string.compare("A", "B")              // Less
/// ```
compare = (a: String, b: String) -> Order {
  case a == b {
    True -> Equal
    False -> compare_bytes(<<a:utf8>>, <<b:utf8>>)
  }
}

compare_bytes = (a: BitArray, b: BitArray) -> Order {
  case (a, b) {
    (<<x, xs:bytes>>, <<y, ys:bytes>>) -> case x == y {
      True -> compare_bytes(xs, ys)
      False -> case x < y {
        True -> Less
        False -> Greater
      }
    }
    (<<>>, <<>>) -> Equal
    (<<>>, _) -> Less
    _ -> Greater
  }
}

/// The text form of any value, as i source (§11.8).
///
/// ```i
/// string.inspect([1, 2])  // "[1, 2]"
/// string.inspect("a")     // "\"a\""
/// ```
inspect = (term: a) -> String { core.inspect(term) }

/// `None` for `""`, and `Some(string)` for any other string.
to_option = (string: String) -> Option(String) {
  case string {
    "" -> None
    _ -> Some(string)
  }
}
