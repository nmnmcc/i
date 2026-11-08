// SPDX-License-Identifier: Apache-2.0
// Derived from gleam_stdlib v1.0.5; persistent Wasm implementation.

import i/int
import i/option.{type Option}

/// A dictionary of keys and values.
///
/// Any type can be used for the keys and values of a dict, but all the keys
/// must be of the same type and all the values must be of the same type.
///
/// Each key can only be present in a dict once.
///
/// Iteration order is an implementation detail and must not be relied upon.
/// Updates share unchanged subtrees with the original dictionary.
pub opaque type Dict(key, value) {
  Dict(size: Int, root: Node(key, value))
}

// A bitmap-indexed hash array mapped trie. A branch consumes five hash bits;
// every branch has at most 32 children, stored in ascending slot order.
// Collisions retain distinct keys with the same complete 32-bit hash.
type Node(key, value) {
  Empty
  Leaf(hash: Int, key: key, value: value)
  Collision(hash: Int, entries: List(#(key, value)))
  Branch(bitmap: Int, children: List(Node(key, value)))
}

// The existing bulk-operation helpers use this alias. It remains persistent:
// callers may retain either version, and no mutation is exposed or performed.
type TransientDict(key, value) =
  Dict(key, value)

fn to_transient(dict: Dict(key, value)) -> TransientDict(key, value) {
  dict
}

fn from_transient(transient: TransientDict(key, value)) -> Dict(key, value) {
  transient
}

@external(wasm, "ilang:runtime/v1", "value_hash")
fn hash(value: a) -> Int

// A total order consistent with structural equality gives collision buckets a
// canonical shape, so dictionary equality is independent of insertion history.
@external(wasm, "ilang:runtime/v1", "value_compare")
fn compare_keys(left: a, right: a) -> Int

/// Determines the number of key-value pairs in the dict.
/// This function runs in constant time and does not need to iterate the dict.
///
/// ## Examples
///
/// ```i
/// assert dict.new() |> dict.size == 0
/// ```
///
/// ```i
/// assert dict.new() |> dict.insert("key", "value") |> dict.size == 1
/// ```
///
pub fn size(dict: Dict(k, v)) -> Int {
  dict.size
}

/// Determines whether or not the dict is empty.
///
/// ## Examples
///
/// ```i
/// assert dict.new() |> dict.is_empty
/// ```
///
/// ```i
/// assert !{ dict.new() |> dict.insert("b", 1) |> dict.is_empty }
/// ```
///
pub fn is_empty(dict: Dict(k, v)) -> Bool {
  size(dict) == 0
}

/// Converts the dict to a list of 2-element tuples `#(key, value)`, one for
/// each key-value pair in the dict.
///
/// The tuples in the list have no specific order.
///
/// ## Examples
///
/// Calling `to_list` on an empty `dict` returns an empty list.
///
/// ```i
/// assert dict.new() |> dict.to_list == []
/// ```
///
/// The ordering of elements in the resulting list is an implementation detail
/// that should not be relied upon.
///
/// ```i
/// assert dict.new()
///   |> dict.insert("b", 1)
///   |> dict.insert("a", 0)
///   |> dict.insert("c", 2)
///   |> dict.to_list
///   == [#("a", 0), #("b", 1), #("c", 2)]
/// ```
///
pub fn to_list(dict: Dict(k, v)) -> List(#(k, v)) {
  fold(dict, from: [], with: fn(acc, key, value) { [#(key, value), ..acc] })
}

/// Converts a list of 2-element tuples `#(key, value)` to a dict.
///
/// If two tuples have the same key the last one in the list will be the one
/// that is present in the dict.
///
pub fn from_list(list: List(#(k, v))) -> Dict(k, v) {
  from_list_loop(to_transient(new()), list)
}

fn from_list_loop(
  transient: TransientDict(k, v),
  list: List(#(k, v)),
) -> Dict(k, v) {
  case list {
    [] -> from_transient(transient)
    [#(key, value), ..rest] ->
      from_list_loop(transient_insert(key, value, transient), rest)
  }
}

/// Determines whether or not a value is present in the dict for a given key.
///
/// ## Examples
///
/// ```i
/// assert dict.new() |> dict.insert("a", 0) |> dict.has_key("a")
/// ```
///
/// ```i
/// assert !{ dict.new() |> dict.insert("a", 0) |> dict.has_key("b") }
/// ```
///
pub fn has_key(dict: Dict(k, v), key: k) -> Bool {
  do_has_key(key, dict)
}

fn do_has_key(key: k, dict: Dict(k, v)) -> Bool {
  case get(dict, key) {
    Ok(_) -> True
    Error(_) -> False
  }
}

/// Creates a fresh dict that contains no values.
///
pub fn new() -> Dict(k, v) {
  Dict(0, Empty)
}

/// Fetches a value from a dict for a given key.
///
/// The dict may not have a value for the key, so the value is wrapped in a
/// `Result`.
///
/// ## Examples
///
/// ```i
/// assert dict.new() |> dict.insert("a", 0) |> dict.get("a") == Ok(0)
/// ```
///
/// ```i
/// assert dict.new() |> dict.insert("a", 0) |> dict.get("b") == Error(Nil)
/// ```
///
pub fn get(from: Dict(k, v), get: k) -> Result(v, Nil) {
  get_node(from.root, hash(get), get, 0)
}

/// Inserts a value into the dict with the given key.
///
/// If the dict already has a value for the given key then the value is
/// replaced with the new value.
///
/// ## Examples
///
/// ```i
/// assert dict.new() |> dict.insert("a", 0) == dict.from_list([#("a", 0)])
/// ```
///
/// ```i
/// assert dict.new() |> dict.insert("a", 0) |> dict.insert("a", 5)
///   == dict.from_list([#("a", 5)])
/// ```
///
pub fn insert(
  into dict: Dict(k, v),
  for key: k,
  insert value: v,
) -> Dict(k, v) {
  do_insert(key, value, dict)
}

fn do_insert(key: k, value: v, dict: Dict(k, v)) -> Dict(k, v) {
  let #(root, added) = insert_node(dict.root, hash(key), key, value, 0)
  Dict(dict.size + added, root)
}

fn transient_insert(
  key: k,
  value: v,
  transient: TransientDict(k, v),
) -> TransientDict(k, v) {
  do_insert(key, value, transient)
}

/// Updates all values in a given dict by calling a given function on each key
/// and value.
///
/// ## Examples
///
/// ```i
/// assert dict.from_list([#(3, 3), #(2, 4)])
///   |> dict.map_values(fn(key, value) { key * value })
///   == dict.from_list([#(3, 9), #(2, 8)])
/// ```
///
pub fn map_values(in dict: Dict(k, v), with fun: fn(k, v) -> a) -> Dict(k, a) {
  do_map_values(fun, dict)
}

fn do_map_values(f: fn(k, v) -> a, dict: Dict(k, v)) -> Dict(k, a) {
  Dict(dict.size, map_node(dict.root, f))
}

/// Gets a list of all keys in a given dict.
///
/// Dicts are not ordered so the keys are not returned in any specific order. Do
/// not write code that relies on the order keys are returned by this function
/// as it may change in later versions of i or Erlang.
///
/// ## Examples
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)]) |> dict.keys == ["a", "b"]
/// ```
///
pub fn keys(dict: Dict(k, v)) -> List(k) {
  fold(dict, [], fn(acc, key, _value) { [key, ..acc] })
}

/// Gets a list of all values in a given dict.
///
/// Dicts are not ordered so the values are not returned in any specific order. Do
/// not write code that relies on the order values are returned by this function
/// as it may change in later versions of i or Erlang.
///
/// ## Examples
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)]) |> dict.values == [0, 1]
/// ```
///
pub fn values(dict: Dict(k, v)) -> List(v) {
  fold(dict, [], fn(acc, _key, value) { [value, ..acc] })
}

/// Creates a new dict from a given dict, minus any entries that a given function
/// returns `False` for.
///
/// ## Examples
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)])
///   |> dict.filter(fn(key, value) { value != 0 })
///   == dict.from_list([#("b", 1)])
/// ```
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)])
///   |> dict.filter(fn(key, value) { True })
///   == dict.from_list([#("a", 0), #("b", 1)])
/// ```
///
pub fn filter(
  in dict: Dict(k, v),
  keeping predicate: fn(k, v) -> Bool,
) -> Dict(k, v) {
  do_filter(predicate, dict)
}

fn do_filter(f: fn(k, v) -> Bool, dict: Dict(k, v)) -> Dict(k, v) {
  to_transient(new())
  |> fold(over: dict, with: fn(transient, key, value) {
    case f(key, value) {
      True -> transient_insert(key, value, transient)
      False -> transient
    }
  })
  |> from_transient
}

/// Creates a new dict from a given dict, only including any entries for which the
/// keys are in a given list.
///
/// ## Examples
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)])
///   |> dict.take(["b"])
///   == dict.from_list([#("b", 1)])
/// ```
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)])
///   |> dict.take(["a", "b", "c"])
///   == dict.from_list([#("a", 0), #("b", 1)])
/// ```
///
pub fn take(
  from dict: Dict(k, v),
  keeping desired_keys: List(k),
) -> Dict(k, v) {
  do_take(desired_keys, dict)
}

fn do_take(desired_keys: List(k), dict: Dict(k, v)) -> Dict(k, v) {
  do_take_loop(dict, desired_keys, to_transient(new()))
}

fn do_take_loop(
  dict: Dict(k, v),
  desired_keys: List(k),
  acc: TransientDict(k, v),
) -> Dict(k, v) {
  case desired_keys {
    [] -> from_transient(acc)
    [key, ..rest] ->
      case get(dict, key) {
        Ok(value) -> do_take_loop(dict, rest, transient_insert(key, value, acc))
        Error(_) -> do_take_loop(dict, rest, acc)
      }
  }
}

/// Creates a new dict from a pair of given dicts by combining their entries.
///
/// If there are entries with the same keys in both dicts the entry from the
/// second dict takes precedence.
///
/// ## Examples
///
/// ```i
/// let a = dict.from_list([#("a", 0), #("b", 1)])
/// let b = dict.from_list([#("b", 2), #("c", 3)])
/// assert dict.merge(a, b) == dict.from_list([#("a", 0), #("b", 2), #("c", 3)])
/// ```
///
pub fn merge(
  into dict: Dict(k, v),
  from new_entries: Dict(k, v),
) -> Dict(k, v) {
  combine(dict, new_entries, fn(_, new_entry) { new_entry })
}

/// Creates a new dict from a given dict with all the same entries except for the
/// one with a given key, if it exists.
///
/// ## Examples
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)]) |> dict.delete("a")
///   == dict.from_list([#("b", 1)])
/// ```
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)]) |> dict.delete("c")
///   == dict.from_list([#("a", 0), #("b", 1)])
/// ```
///
pub fn delete(from dict: Dict(k, v), delete key: k) -> Dict(k, v) {
  to_transient(dict) |> transient_delete(key, _) |> from_transient
}

fn transient_delete(key: k, dict: TransientDict(k, v)) -> TransientDict(k, v) {
  let #(root, removed) = delete_node(dict.root, hash(key), key, 0)
  Dict(dict.size - removed, root)
}

/// Creates a new dict from a given dict with all the same entries except any with
/// keys found in a given list.
///
/// ## Examples
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)]) |> dict.drop(["a"])
///   == dict.from_list([#("b", 1)])
/// ```
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)]) |> dict.drop(["c"])
///   == dict.from_list([#("a", 0), #("b", 1)])
/// ```
///
/// ```i
/// assert dict.from_list([#("a", 0), #("b", 1)]) |> dict.drop(["a", "b", "c"])
///   == dict.from_list([])
/// ```
///
pub fn drop(
  from dict: Dict(k, v),
  drop disallowed_keys: List(k),
) -> Dict(k, v) {
  do_drop(disallowed_keys, dict)
}

fn do_drop(disallowed_keys: List(k), dict: Dict(k, v)) -> Dict(k, v) {
  drop_loop(to_transient(dict), disallowed_keys)
}

fn drop_loop(
  transient: TransientDict(k, v),
  disallowed_keys: List(k),
) -> Dict(k, v) {
  case disallowed_keys {
    [] -> from_transient(transient)
    [key, ..rest] -> drop_loop(transient_delete(key, transient), rest)
  }
}

/// Creates a new dict with one entry inserted or updated using a given function.
///
/// If there was not an entry in the dict for the given key then the function
/// gets `None` as its argument, otherwise it gets `Some(value)`.
///
/// ## Examples
///
/// ```i
/// let dict = dict.from_list([#("a", 0)])
/// let increment = fn(x) {
///   case x {
///     Some(i) -> i + 1
///     None -> 0
///   }
/// }
///
/// assert dict.upsert(dict, "a", increment) == dict.from_list([#("a", 1)])
/// ```
///
/// ```i
/// assert dict.upsert(dict, "b", increment)
///   == dict.from_list([#("a", 0), #("b", 0)])
/// ```
///
pub fn upsert(
  in dict: Dict(k, v),
  update key: k,
  with fun: fn(Option(v)) -> v,
) -> Dict(k, v) {
  case get(dict, key) {
    Ok(value) -> insert(dict, key, fun(option.Some(value)))
    Error(_) -> insert(dict, key, fun(option.None))
  }
}

/// Combines all entries into a single value by calling a given function on each
/// one.
///
/// Dicts are not ordered so the values are not returned in any specific order. Do
/// not write code that relies on the order entries are used by this function
/// as it may change in later versions of i or Erlang.
///
/// ## Examples
///
/// ```i
/// let dict = dict.from_list([#("a", 1), #("b", 3), #("c", 9)])
/// assert dict.fold(dict, 0, fn(accumulator, key, value) { accumulator + value })
///   == 13
/// ```
///
/// ```i
/// import i/string
///
/// let dict = dict.from_list([#("a", 1), #("b", 3), #("c", 9)])
/// assert dict.fold(dict, "", fn(accumulator, key, value) {
///     string.append(accumulator, key)
///   })
///   == "abc"
/// ```
///
pub fn fold(
  over dict: Dict(k, v),
  from initial: acc,
  with fun: fn(acc, k, v) -> acc,
) -> acc {
  let fun = fn(key, value, acc) { fun(acc, key, value) }
  do_fold(fun, initial, dict)
}

fn do_fold(fun: fn(k, v, acc) -> acc, initial: acc, dict: Dict(k, v)) -> acc {
  fold_node(dict.root, initial, fun)
}

/// Calls a function for each key and value in a dict, discarding the return
/// value.
///
/// Useful for producing a side effect for every item of a dict.
///
/// ```i
/// import i/io
///
/// let dict =
///   dict.from_list([#("a", "apple"), #("b", "banana"), #("c", "cherry")])
///
/// assert dict.each(dict, fn(k, v) { io.println(k <> " => " <> v) }) == Nil
/// // a => apple
/// // b => banana
/// // c => cherry
/// ```
///
/// The order of elements in the iteration is an implementation detail that
/// should not be relied upon.
///
pub fn each(dict: Dict(k, v), fun: fn(k, v) -> a) -> Nil {
  fold(dict, Nil, fn(nil, k, v) {
    fun(k, v)
    nil
  })
}

/// Creates a new dict from a pair of given dicts by combining their entries.
///
/// If there are entries with the same keys in both dicts the given function is
/// used to determine the new value to use in the resulting dict.
///
/// ## Examples
///
/// ```i
/// let a = dict.from_list([#("a", 0), #("b", 1)])
/// let b = dict.from_list([#("a", 2), #("c", 3)])
/// assert dict.combine(a, b, fn(one, other) { one + other })
///   == dict.from_list([#("a", 2), #("b", 1), #("c", 3)])
/// ```
///
pub fn combine(
  dict: Dict(k, v),
  other: Dict(k, v),
  with fun: fn(v, v) -> v,
) -> Dict(k, v) {
  do_combine(fn(_, l, r) { fun(l, r) }, dict, other)
}

fn do_combine(
  combine: fn(k, v, v) -> v,
  left: Dict(k, v),
  right: Dict(k, v),
) -> Dict(k, v) {
  let #(big, small, combine) = case size(left) >= size(right) {
    True -> #(left, right, combine)
    False -> #(right, left, fn(k, l, r) { combine(k, r, l) })
  }

  to_transient(big)
  |> fold(over: small, with: fn(transient, key, value) {
    let update = fn(existing) { combine(key, existing, value) }
    transient_update_with(key, update, value, transient)
  })
  |> from_transient
}

fn transient_update_with(
  key: k,
  fun: fn(v) -> v,
  init: v,
  transient: TransientDict(k, v),
) -> TransientDict(k, v) {
  let value = case get(transient, key) {
    Ok(existing) -> fun(existing)
    Error(_) -> init
  }
  do_insert(key, value, transient)
}

@internal
pub fn group(key: fn(v) -> k, list: List(v)) -> Dict(k, List(v)) {
  group_loop(to_transient(new()), key, list)
}

fn group_loop(
  transient: TransientDict(k, List(v)),
  to_key: fn(v) -> k,
  list: List(v),
) -> Dict(k, List(v)) {
  case list {
    [] -> from_transient(transient)
    [value, ..rest] -> {
      let key = to_key(value)
      let update = fn(existing) { [value, ..existing] }

      transient
      |> transient_update_with(key, update, [value], _)
      |> group_loop(to_key, rest)
    }
  }
}

fn hash_slot(hash: Int, shift: Int) -> Int {
  int.bitwise_and(int.bitwise_shift_right(hash, shift), 31)
}

fn slot_bit(hash: Int, shift: Int) -> Int {
  int.bitwise_shift_left(1, hash_slot(hash, shift))
}

fn population_count(bits: Int, count: Int) -> Int {
  case bits {
    0 -> count
    _ -> population_count(int.bitwise_and(bits, bits - 1), count + 1)
  }
}

fn child_index(bitmap: Int, bit: Int) -> Int {
  population_count(int.bitwise_and(bitmap, bit - 1), 0)
}

fn get_node(node: Node(k, v), hash: Int, key: k, shift: Int) -> Result(v, Nil) {
  case node {
    Empty -> Error(Nil)
    Leaf(other_hash, other_key, value) -> {
      case hash == other_hash && key == other_key {
        True -> Ok(value)
        False -> Error(Nil)
      }
    }
    Collision(other_hash, entries) -> {
      case hash == other_hash {
        True -> find_entry(entries, key)
        False -> Error(Nil)
      }
    }
    Branch(bitmap, children) -> {
      let bit = slot_bit(hash, shift)
      case int.bitwise_and(bitmap, bit) == 0 {
        True -> Error(Nil)
        False ->
          get_node(
            child_at(children, child_index(bitmap, bit)),
            hash,
            key,
            shift + 5,
          )
      }
    }
  }
}

fn find_entry(entries: List(#(k, v)), key: k) -> Result(v, Nil) {
  case entries {
    [] -> Error(Nil)
    [#(other_key, value), ..rest] -> {
      case key == other_key {
        True -> Ok(value)
        False -> find_entry(rest, key)
      }
    }
  }
}

fn insert_node(
  node: Node(k, v),
  hash: Int,
  key: k,
  value: v,
  shift: Int,
) -> #(Node(k, v), Int) {
  case node {
    Empty -> #(Leaf(hash, key, value), 1)
    Leaf(other_hash, other_key, other_value) -> {
      case hash == other_hash {
        True -> {
          case key == other_key {
            True -> #(Leaf(hash, key, value), 0)
            False -> {
              let entries = case compare_keys(key, other_key) < 0 {
                True -> [#(key, value), #(other_key, other_value)]
                False -> [#(other_key, other_value), #(key, value)]
              }
              #(Collision(hash, entries), 1)
            }
          }
        }
        False -> #(
          join_nodes(node, other_hash, Leaf(hash, key, value), hash, shift),
          1,
        )
      }
    }
    Collision(other_hash, entries) -> {
      case hash == other_hash {
        True -> {
          let #(entries, added) = insert_entry(entries, key, value)
          #(Collision(hash, entries), added)
        }
        False -> #(
          join_nodes(node, other_hash, Leaf(hash, key, value), hash, shift),
          1,
        )
      }
    }
    Branch(bitmap, children) -> {
      let bit = slot_bit(hash, shift)
      let index = child_index(bitmap, bit)
      case int.bitwise_and(bitmap, bit) == 0 {
        True -> #(
          Branch(
            int.bitwise_or(bitmap, bit),
            insert_child(children, index, Leaf(hash, key, value)),
          ),
          1,
        )
        False -> {
          let #(child, added) =
            insert_node(child_at(children, index), hash, key, value, shift + 5)
          #(Branch(bitmap, replace_child(children, index, child)), added)
        }
      }
    }
  }
}

// Joining distinct complete hashes must reach different five-bit slots before
// the 32-bit hash runs out. A one-child branch is retained for each shared prefix.
fn join_nodes(
  left: Node(k, v),
  left_hash: Int,
  right: Node(k, v),
  right_hash: Int,
  shift: Int,
) -> Node(k, v) {
  let left_slot = hash_slot(left_hash, shift)
  let right_slot = hash_slot(right_hash, shift)
  let left_bit = int.bitwise_shift_left(1, left_slot)
  let right_bit = int.bitwise_shift_left(1, right_slot)
  case left_slot == right_slot {
    True ->
      Branch(left_bit, [
        join_nodes(left, left_hash, right, right_hash, shift + 5),
      ])
    False -> {
      let children = case left_slot < right_slot {
        True -> [left, right]
        False -> [right, left]
      }
      Branch(int.bitwise_or(left_bit, right_bit), children)
    }
  }
}

fn insert_entry(
  entries: List(#(k, v)),
  key: k,
  value: v,
) -> #(List(#(k, v)), Int) {
  insert_entry_loop(entries, key, value, [])
}

fn insert_entry_loop(
  entries: List(#(k, v)),
  key: k,
  value: v,
  preceding: List(#(k, v)),
) -> #(List(#(k, v)), Int) {
  case entries {
    [] -> #(prepend_reversed(preceding, [#(key, value)]), 1)
    [#(other_key, _) as entry, ..rest] -> {
      case key == other_key {
        True -> #(prepend_reversed(preceding, [#(key, value), ..rest]), 0)
        False -> {
          case compare_keys(key, other_key) < 0 {
            True -> #(
              prepend_reversed(preceding, [#(key, value), ..entries]),
              1,
            )
            False -> insert_entry_loop(rest, key, value, [entry, ..preceding])
          }
        }
      }
    }
  }
}

fn prepend_reversed(prefix: List(a), suffix: List(a)) -> List(a) {
  case prefix {
    [] -> suffix
    [value, ..rest] -> prepend_reversed(rest, [value, ..suffix])
  }
}

fn delete_node(
  node: Node(k, v),
  hash: Int,
  key: k,
  shift: Int,
) -> #(Node(k, v), Int) {
  case node {
    Empty -> #(node, 0)
    Leaf(other_hash, other_key, _) -> {
      case hash == other_hash && key == other_key {
        True -> #(Empty, 1)
        False -> #(node, 0)
      }
    }
    Collision(other_hash, entries) -> {
      case hash == other_hash {
        False -> #(node, 0)
        True -> {
          let #(entries, removed) = delete_entry(entries, key)
          let node = case entries {
            [] -> Empty
            [#(key, value)] -> Leaf(hash, key, value)
            _ -> Collision(hash, entries)
          }
          #(node, removed)
        }
      }
    }
    Branch(bitmap, children) -> {
      let bit = slot_bit(hash, shift)
      case int.bitwise_and(bitmap, bit) == 0 {
        True -> #(node, 0)
        False -> {
          let index = child_index(bitmap, bit)
          let #(child, removed) =
            delete_node(child_at(children, index), hash, key, shift + 5)
          case removed {
            0 -> #(node, 0)
            _ -> {
              let #(bitmap, children) = case child {
                Empty -> #(bitmap - bit, delete_child(children, index))
                _ -> #(bitmap, replace_child(children, index, child))
              }
              #(compact_branch(bitmap, children), removed)
            }
          }
        }
      }
    }
  }
}

// A leaf or collision carries its complete hash and may be lifted through any
// number of prefix branches. A branch itself is depth-dependent and cannot be
// lifted, even when it is the only child of its parent.
fn compact_branch(bitmap: Int, children: List(Node(k, v))) -> Node(k, v) {
  case children {
    [] -> Empty
    [Leaf(..) as child] -> child
    [Collision(..) as child] -> child
    _ -> Branch(bitmap, children)
  }
}

fn delete_entry(entries: List(#(k, v)), key: k) -> #(List(#(k, v)), Int) {
  delete_entry_loop(entries, key, [])
}

fn delete_entry_loop(
  entries: List(#(k, v)),
  key: k,
  preceding: List(#(k, v)),
) -> #(List(#(k, v)), Int) {
  case entries {
    [] -> #(prepend_reversed(preceding, []), 0)
    [#(other_key, _) as entry, ..rest] -> {
      case key == other_key {
        True -> #(prepend_reversed(preceding, rest), 1)
        False -> delete_entry_loop(rest, key, [entry, ..preceding])
      }
    }
  }
}

// Child indices come from the population count of a branch's bitmap. The
// assertions defend internal invariants; arbitrary user keys never choose an
// out-of-bounds index.
fn child_at(children: List(Node(k, v)), index: Int) -> Node(k, v) {
  let assert [child, ..rest] = children
  case index {
    0 -> child
    _ -> child_at(rest, index - 1)
  }
}

fn insert_child(
  children: List(Node(k, v)),
  index: Int,
  child: Node(k, v),
) -> List(Node(k, v)) {
  case index {
    0 -> [child, ..children]
    _ -> {
      let assert [first, ..rest] = children
      [first, ..insert_child(rest, index - 1, child)]
    }
  }
}

fn replace_child(
  children: List(Node(k, v)),
  index: Int,
  child: Node(k, v),
) -> List(Node(k, v)) {
  let assert [first, ..rest] = children
  case index {
    0 -> [child, ..rest]
    _ -> [first, ..replace_child(rest, index - 1, child)]
  }
}

fn delete_child(children: List(Node(k, v)), index: Int) -> List(Node(k, v)) {
  let assert [first, ..rest] = children
  case index {
    0 -> rest
    _ -> [first, ..delete_child(rest, index - 1)]
  }
}

fn map_node(node: Node(k, v), fun: fn(k, v) -> a) -> Node(k, a) {
  case node {
    Empty -> Empty
    Leaf(hash, key, value) -> Leaf(hash, key, fun(key, value))
    Collision(hash, entries) -> Collision(hash, map_entries(entries, fun))
    Branch(bitmap, children) -> Branch(bitmap, map_children(children, fun))
  }
}

fn map_entries(entries: List(#(k, v)), fun: fn(k, v) -> a) -> List(#(k, a)) {
  map_entries_loop(entries, fun, [])
}

fn map_entries_loop(
  entries: List(#(k, v)),
  fun: fn(k, v) -> a,
  reversed: List(#(k, a)),
) -> List(#(k, a)) {
  case entries {
    [] -> prepend_reversed(reversed, [])
    [#(key, value), ..rest] ->
      map_entries_loop(rest, fun, [#(key, fun(key, value)), ..reversed])
  }
}

fn map_children(
  children: List(Node(k, v)),
  fun: fn(k, v) -> a,
) -> List(Node(k, a)) {
  case children {
    [] -> []
    [child, ..rest] -> [map_node(child, fun), ..map_children(rest, fun)]
  }
}

fn fold_node(node: Node(k, v), initial: acc, fun: fn(k, v, acc) -> acc) -> acc {
  case node {
    Empty -> initial
    Leaf(_, key, value) -> fun(key, value, initial)
    Collision(_, entries) -> fold_entries(entries, initial, fun)
    Branch(_, children) -> fold_children(children, initial, fun)
  }
}

fn fold_entries(
  entries: List(#(k, v)),
  initial: acc,
  fun: fn(k, v, acc) -> acc,
) -> acc {
  case entries {
    [] -> initial
    [#(key, value), ..rest] -> fold_entries(rest, fun(key, value, initial), fun)
  }
}

fn fold_children(
  children: List(Node(k, v)),
  initial: acc,
  fun: fn(k, v, acc) -> acc,
) -> acc {
  case children {
    [] -> initial
    [child, ..rest] -> fold_children(rest, fold_node(child, initial, fun), fun)
  }
}
