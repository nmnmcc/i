import gleam/dict
import gleam/option.{None, Some}

pub fn persistence_test() -> Nil {
  let empty = dict.new()
  let original = empty |> dict.insert("one", 1) |> dict.insert("two", 2)
  let updated = original |> dict.insert("one", 11) |> dict.insert("three", 3)
  let deleted = updated |> dict.delete("two")
  assert dict.is_empty(empty)
  assert dict.get(original, "one") == Ok(1)
  assert dict.get(original, "three") == Error(Nil)
  assert dict.get(updated, "one") == Ok(11)
  assert dict.get(updated, "two") == Ok(2)
  assert dict.get(deleted, "two") == Error(Nil)
  assert dict.size(original) == 2
  assert dict.size(updated) == 3
  assert dict.size(deleted) == 2
}

pub fn insertion_history_equality_test() -> Nil {
  let forward = insert_range(dict.new(), 0, 1, 256)
  let backward = insert_range(dict.new(), 255, -1, 256)
  assert forward == backward
  assert dict.size(forward) == 256
  check_range(forward, 0, 256)
}

pub fn deletion_history_equality_test() -> Nil {
  let complete = insert_range(dict.new(), 0, 1, 256)
  let without_even = delete_even(complete, 0, 128)
  let reconstructed = insert_range(dict.new(), 1, 2, 128)
  assert without_even == reconstructed
  assert dict.size(without_even) == 128
  assert dict.size(complete) == 256
  let singleton = delete_range(complete, 0, 1, 255)
  assert singleton == dict.from_list([#(255, 765)])
  assert dict.delete(singleton, 255) == dict.new()
  assert dict.delete(singleton, -1) == singleton
}

pub fn map_filter_combine_test() -> Nil {
  let original = dict.from_list([#(1, 10), #(2, 20), #(3, 30)])
  assert dict.map_values(original, fn(k, v) { k + v })
    == dict.from_list([#(1, 11), #(2, 22), #(3, 33)])
  assert dict.filter(original, fn(k, _) { k % 2 == 1 })
    == dict.from_list([#(1, 10), #(3, 30)])
  assert dict.combine(original, dict.from_list([#(2, 5), #(4, 7)]), fn(a, b) {
      a - b
    })
    == dict.from_list([#(1, 10), #(2, 15), #(3, 30), #(4, 7)])
  assert dict.merge(original, dict.from_list([#(2, 5)]))
    == dict.from_list([#(1, 10), #(2, 5), #(3, 30)])
  assert dict.fold(original, 0, fn(acc, key, value) { acc + key + value }) == 66
}

pub fn take_drop_upsert_test() -> Nil {
  let original = dict.from_list([#("a", 1), #("b", 2), #("c", 3)])
  assert dict.take(original, ["c", "a", "c", "absent"])
    == dict.from_list([#("c", 3), #("a", 1)])
  assert dict.drop(original, ["c", "c", "absent"])
    == dict.from_list([#("a", 1), #("b", 2)])
  let increment = fn(value) {
    case value {
      None -> 100
      Some(n) -> n + 1
    }
  }
  assert original |> dict.upsert("a", increment) |> dict.get("a") == Ok(2)
  assert original |> dict.upsert("absent", increment) |> dict.get("absent")
    == Ok(100)
}

type Key {
  User(id: Int, name: String)
  Group(members: List(Int))
}

pub fn structural_keys_test() -> Nil {
  let a = User(1, "α")
  let b = User(2, "β")
  let c = Group([1, 2, 3])
  let entries = dict.from_list([#(a, "first"), #(b, "second"), #(c, "third")])
  assert dict.get(entries, User(1, "α")) == Ok("first")
  assert dict.get(entries, Group([1, 2, 3])) == Ok("third")
  assert dict.get(entries, Group([1, 3, 2])) == Error(Nil)
  assert dict.from_list([#(c, "third"), #(a, "first"), #(b, "second")])
    == entries
}

// These distinct strings collide under the runtime's tagged FNV-1a hash.
// This exercises complete-hash collisions, not merely a shared trie prefix.
pub fn full_hash_collision_test() -> Nil {
  let first = "wleam-hamt-297688"
  let second = "wleam-hamt-1541250"
  assert first != second
  assert hash(first) == hash(second)
  let forward = dict.from_list([#(first, 1), #(second, 2)])
  let backward = dict.from_list([#(second, 2), #(first, 1)])
  assert forward == backward
  assert dict.size(forward) == 2
  assert dict.get(forward, first) == Ok(1)
  assert dict.get(forward, second) == Ok(2)
  let updated = dict.insert(forward, first, 3)
  assert dict.get(updated, first) == Ok(3)
  assert dict.get(updated, second) == Ok(2)
  assert dict.size(updated) == 2
  let expanded = dict.insert(updated, "unrelated", 4)
  assert dict.delete(expanded, "unrelated") == updated
  assert dict.delete(updated, first) == dict.from_list([#(second, 2)])
  assert updated |> dict.delete(second) |> dict.delete(first) == dict.new()
  assert dict.get(forward, first) == Ok(1)
}

@external(wasm, "wleam:runtime/v1", "value_hash")
fn hash(value: a) -> Int

fn insert_range(dict: dict.Dict(Int, Int), value: Int, step: Int, count: Int) {
  case count {
    0 -> dict
    _ ->
      insert_range(
        dict.insert(dict, value, value * 3),
        value + step,
        step,
        count - 1,
      )
  }
}

fn check_range(dict: dict.Dict(Int, Int), value: Int, count: Int) -> Nil {
  case count {
    0 -> Nil
    _ -> {
      assert dict.get(dict, value) == Ok(value * 3)
      check_range(dict, value + 1, count - 1)
    }
  }
}

fn delete_even(dict: dict.Dict(Int, Int), value: Int, count: Int) {
  case count {
    0 -> dict
    _ -> delete_even(dict.delete(dict, value), value + 2, count - 1)
  }
}

fn delete_range(dict: dict.Dict(Int, Int), value: Int, step: Int, count: Int) {
  case count {
    0 -> dict
    _ -> delete_range(dict.delete(dict, value), value + step, step, count - 1)
  }
}
