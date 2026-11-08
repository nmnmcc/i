import i/list
import i/ilang_string as text
import i/ilang_unicode as unicode

/// `StringTree` is a type used for efficiently building text content to be
/// written to a file or a socket. Internally it is represented as a tree so to
/// append or prepend to a string tree is a constant time operation that
/// allocates a new node in the tree without copying any of the content. When
/// writing to an output stream the tree is traversed and the content is sent
/// directly rather than copying it into a single buffer beforehand.
///
/// On Wasm, persistent GC nodes retain the original string buffers. Flattening
/// allocates the final UTF-8 buffer once. Deep trees are traversed using an
/// explicit work list so appending many chunks does not exhaust the call stack.
///
pub opaque type StringTree {
  Text(String)
  Strings(List(String))
  Trees(List(StringTree))
  Append(StringTree, StringTree)
}

/// Create an empty `StringTree`. Useful as the start of a pipe chaining many
/// trees together.
///
pub fn new() -> StringTree {
  from_strings([])
}

/// Prepends a `String` onto the start of some `StringTree`.
///
/// Runs in constant time.
///
pub fn prepend(to tree: StringTree, prefix prefix: String) -> StringTree {
  append_tree(from_string(prefix), tree)
}

/// Appends a `String` onto the end of some `StringTree`.
///
/// Runs in constant time.
///
pub fn append(to tree: StringTree, suffix second: String) -> StringTree {
  append_tree(tree, from_string(second))
}

/// Prepends some `StringTree` onto the start of another.
///
/// Runs in constant time.
///
pub fn prepend_tree(
  to tree: StringTree,
  prefix prefix: StringTree,
) -> StringTree {
  append_tree(prefix, tree)
}

/// Appends some `StringTree` onto the end of another.
///
/// Runs in constant time.
///
pub fn append_tree(
  to tree: StringTree,
  suffix suffix: StringTree,
) -> StringTree {
  Append(tree, suffix)
}

/// Converts a list of strings into a `StringTree`.
///
/// Runs in constant time.
///
pub fn from_strings(strings: List(String)) -> StringTree {
  Strings(strings)
}

/// Joins a list of trees into a single tree.
///
/// Runs in constant time.
///
pub fn concat(trees: List(StringTree)) -> StringTree {
  Trees(trees)
}

/// Converts a string into a `StringTree`.
///
/// Runs in constant time.
///
pub fn from_string(string: String) -> StringTree {
  Text(string)
}

/// Turns a `StringTree` into a `String`.
///
/// This function is implemented natively by the virtual machine and is highly
/// optimised.
///
pub fn to_string(tree: StringTree) -> String {
  text.concat(chunks(tree))
}

/// Returns the size of the `StringTree` in bytes.
///
pub fn byte_size(tree: StringTree) -> Int {
  list.fold(chunks(tree), 0, fn(size, chunk) { size + text.byte_size(chunk) })
}

/// Joins the given trees into a new tree separated with the given string.
///
pub fn join(trees: List(StringTree), with sep: String) -> StringTree {
  trees
  |> list.intersperse(from_string(sep))
  |> concat
}

/// Converts a `StringTree` to a new one where the contents have been
/// lowercased.
///
pub fn lowercase(tree: StringTree) -> StringTree {
  from_string(unicode.lowercase(to_string(tree)))
}

/// Converts a `StringTree` to a new one where the contents have been
/// uppercased.
///
pub fn uppercase(tree: StringTree) -> StringTree {
  from_string(unicode.uppercase(to_string(tree)))
}

/// Converts a `StringTree` to a new one with the contents reversed.
///
pub fn reverse(tree: StringTree) -> StringTree {
  tree
  |> to_string
  |> do_to_graphemes
  |> list.reverse
  |> from_strings
}

fn do_to_graphemes(string: String) -> List(String) {
  unicode.graphemes(string)
}

/// Splits a `StringTree` on a given pattern into a list of trees.
///
pub fn split(tree: StringTree, on pattern: String) -> List(StringTree) {
  case pattern {
    "" -> [tree]
    _ ->
      split_pieces(
        text.split(to_string(tree), pattern),
        chunks(tree),
        text.byte_size(pattern),
        [],
      )
  }
}

/// Replaces all instances of a pattern with a given string substitute.
///
pub fn replace(
  in tree: StringTree,
  each pattern: String,
  with substitute: String,
) -> StringTree {
  from_string(text.replace(to_string(tree), pattern, substitute))
}

/// Compares two string trees to determine if they have the same textual
/// content.
///
/// Comparing two string trees using the `==` operator may return `False` even
/// if they have the same content as they may have been built in different ways,
/// so using this function is often preferred.
///
/// ## Examples
///
/// ```i
/// assert string_tree.from_strings(["a", "b"]) != string_tree.from_string("ab")
/// ```
///
/// ```i
/// assert string_tree.is_equal(
///   string_tree.from_strings(["a", "b"]),
///   string_tree.from_string("ab"),
/// )
/// ```
///
pub fn is_equal(a: StringTree, b: StringTree) -> Bool {
  to_string(a) == to_string(b)
}

/// Inspects a `StringTree` to determine if it is equivalent to an empty string.
///
/// ## Examples
///
/// ```i
/// assert !{ string_tree.from_string("ok") |> string_tree.is_empty }
/// ```
///
/// ```i
/// assert string_tree.from_string("") |> string_tree.is_empty
/// ```
///
/// ```i
/// assert string_tree.from_strings([]) |> string_tree.is_empty
/// ```
///
pub fn is_empty(tree: StringTree) -> Bool {
  byte_size(tree) == 0
}

fn chunks(tree: StringTree) -> List(String) {
  flatten([tree], [])
}

fn flatten(pending: List(StringTree), acc: List(String)) -> List(String) {
  case pending {
    [] -> reverse_into(acc, [])
    [Text(string), ..rest] -> flatten(rest, [string, ..acc])
    [Strings(strings), ..rest] -> flatten(rest, reverse_into(strings, acc))
    [Trees(trees), ..rest] ->
      flatten(reverse_into(reverse_into(trees, []), rest), acc)
    [Append(first, second), ..rest] -> flatten([first, second, ..rest], acc)
  }
}

fn reverse_into(list: List(a), tail: List(a)) -> List(a) {
  case list {
    [] -> tail
    [first, ..rest] -> reverse_into(rest, [first, ..tail])
  }
}

// Preserve the untouched chunks on either side of a separator, including a
// separator crossing tree nodes. Only a boundary chunk needs to be sliced.
fn split_pieces(
  pieces: List(String),
  source: List(String),
  separator_size: Int,
  acc: List(StringTree),
) -> List(StringTree) {
  case pieces {
    [] -> reverse_into(acc, [])
    [piece, ..rest] -> {
      let #(selected, remaining) = take_bytes(source, text.byte_size(piece), [])
      let tree = case selected {
        [] -> Text("")
        [one] -> Text(one)
        many -> Strings(many)
      }
      let remaining = case rest {
        [] -> remaining
        _ -> {
          let #(_, tail) = take_bytes(remaining, separator_size, [])
          tail
        }
      }
      split_pieces(rest, remaining, separator_size, [tree, ..acc])
    }
  }
}

fn take_bytes(
  source: List(String),
  count: Int,
  acc: List(String),
) -> #(List(String), List(String)) {
  case count <= 0 {
    True -> #(reverse_into(acc, []), source)
    False ->
      case source {
        [] -> #(reverse_into(acc, []), [])
        [chunk, ..rest] -> {
          let size = text.byte_size(chunk)
          case size <= count {
            True -> take_bytes(rest, count - size, [chunk, ..acc])
            False -> {
              let before = text.byte_slice(chunk, 0, count)
              let after = text.byte_slice(chunk, count, size - count)
              #(reverse_into([before, ..acc], []), [after, ..rest])
            }
          }
        }
      }
  }
}
