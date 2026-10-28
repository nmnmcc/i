import gleam/io
import gleam/int
import gleam/dict
import gleam/list
import gleam/set

type Chain {
  End
  First(Chain, Int)
  Middle(Int, Chain, Int)
}

fn first_chain(depth: Int, accumulator: Chain) -> Chain {
  case depth {
    0 -> accumulator
    _ -> first_chain(depth - 1, First(accumulator, depth))
  }
}

fn middle_chain(depth: Int, accumulator: Chain) -> Chain {
  case depth {
    0 -> accumulator
    _ -> middle_chain(depth - 1, Middle(depth, accumulator, depth))
  }
}

pub fn main() {
  let original = dict.from_list([#([1, 2], "a"), #([3], "b")])
  let modified = dict.insert(original, [1, 2], "changed")
  assert dict.get(original, [1, 2]) == Ok("a")
  assert dict.get(modified, [1, 2]) == Ok("changed")
  assert dict.size(modified) == 2
  assert dict.get(dict.delete(modified, [3]), [3]) == Error(Nil)
  let many = int.range(from: 0, to: 501, with: dict.new(), run: fn(acc, n) { dict.insert(acc, n, n * n) })
  assert dict.size(many) == 501
  assert dict.get(many, 499) == Ok(249_001)
  let members = set.from_list([1, 2, 1, 3])
  assert set.size(members) == 3
  assert set.contains(members, 2)
  assert list.map([1, 2, 3], fn(n) { n * 2 }) == [2, 4, 6]
  let first_left = first_chain(10_000, End)
  let first_right = first_chain(10_000, End)
  assert first_left == first_right
  assert dict.get(dict.from_list([#(first_left, "deep")]), first_right) == Ok("deep")
  let middle_left = middle_chain(10_000, End)
  let middle_right = middle_chain(10_000, End)
  assert middle_left == middle_right
  let assert Middle(a, b, c) = middle_right
  let different = Middle(a, b, c + 1)
  assert middle_left != different
  assert dict.get(dict.from_list([#(middle_left, "deep")]), different) == Error(Nil)
  io.println("PASS:collections")
}
