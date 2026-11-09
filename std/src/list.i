//// Functions for `List(a)`, the built-in type of lists that may not be
//// changed.
////
//// Every function that walks a whole list is tail-recursive (§11.6), so it
//// works on lists of any length without using more stack. Functions that
//// build a new list build it in reverse and then turn it round, which takes
//// one more pass.
////
//// Functions that may find nothing give an `Option` (`first`, `find`,
//// `at`, …). Functions that take a function as an argument have the errors
//// and the environment of that function (`? e ! n`), so the argument may
//// itself raise errors or need services.

import (
  dict = (Dict) = std/dict
  option = (Option, Some, None) = std/option
  order = (Order, Less, Equal, Greater) = std/order
)

export (
  ContinueOrStop(..)
  length
  count
  reverse
  is_empty
  contains
  first
  rest
  last
  at
  index_of
  group
  filter
  filter_map
  map
  map2
  map_fold
  index_map
  try_map
  drop
  take
  new
  wrap
  append
  prepend
  concat
  flat_map
  fold
  fold_right
  index_fold
  try_fold
  fold_until
  find
  find_map
  all
  any
  zip
  strict_zip
  unzip
  intersperse
  unique
  sort
  range
  repeat
  split
  split_while
  key_find
  key_filter
  key_pop
  key_set
  each
  try_each
  partition
  permutations
  window
  window_by_2
  drop_while
  take_while
  chunk
  sized_chunk
  reduce
  scan
  combinations
  combination_pairs
  interleave
  transpose
  max
  min
)

/// What `fold_until` does next: go on with the new accumulator, or stop
/// with it.
ContinueOrStop(a) {
  Continue(a)
  Stop(a)
}

/// The number of elements. It walks the whole list.
///
/// ```i
/// list.length([1, 2, 3])  // 3
/// ```
length = (of: List(a)) -> Int {
  go = (rest, n) {
    case rest {
      [] -> n
      [_, ..xs] -> go(xs, n + 1)
    }
  }
  go(of, 0)
}

/// The number of elements for which the function gives `True`.
count = (list: List(a), where: (a) -> Bool ? e ! n) -> Int ? e ! n {
  go = (rest, n) {
    case rest {
      [] -> n
      [x, ..xs] -> case where(x) {
        True -> go(xs, n + 1)
        False -> go(xs, n)
      }
    }
  }
  go(list, 0)
}

/// The elements in the other order.
///
/// ```i
/// list.reverse([1, 2, 3])  // [3, 2, 1]
/// ```
reverse = (list: List(a)) -> List(a) { reverse_onto(list, []) }

// Puts the elements of `list`, last first, in front of `acc`.
reverse_onto = (list: List(a), acc: List(a)) -> List(a) {
  case list {
    [] -> acc
    [x, ..xs] -> reverse_onto(xs, [x, ..acc])
  }
}

/// `True` for the empty list. It takes one step.
is_empty = (list: List(a)) -> Bool { list == [] }

/// `True` when an element is `==` to `any`.
///
/// ```i
/// list.contains([1, 2], any: 2)  // True
/// ```
contains = (list: List(a), any: a) -> Bool {
  case list {
    [] -> False
    [x, ..xs] -> x == any || contains(xs, any)
  }
}

/// The first element, or `None` for the empty list.
first = (list: List(a)) -> Option(a) {
  case list {
    [] -> None
    [x, ..] -> Some(x)
  }
}

/// The list without its first element, or `None` for the empty list.
rest = (list: List(a)) -> Option(List(a)) {
  case list {
    [] -> None
    [_, ..xs] -> Some(xs)
  }
}

/// The last element, or `None` for the empty list.
last = (list: List(a)) -> Option(a) {
  case list {
    [] -> None
    [x] -> Some(x)
    [_, ..xs] -> last(xs)
  }
}

/// The element at an index, counting from 0, or `None` when the index is
/// not in the list.
///
/// ```i
/// list.at([10, 20, 30], 1)  // Some(20)
/// ```
at = (list: List(a), index: Int) -> Option(a) {
  case index < 0 {
    True -> None
    False -> case list {
      [] -> None
      [x, ..xs] -> case index {
        0 -> Some(x)
        _ -> at(xs, index - 1)
      }
    }
  }
}

/// The index of the first element that is `==` to `item`, counting from
/// 0, or `None`.
index_of = (list: List(a), item: a) -> Option(Int) {
  go = (rest, i) {
    case rest {
      [] -> None
      [x, ..xs] -> case x == item {
        True -> Some(i)
        False -> go(xs, i + 1)
      }
    }
  }
  go(list, 0)
}

/// Puts the elements into groups by a key function, in a dict. Each group
/// keeps the elements in the order of the list.
///
/// ```i
/// list.group([1, 2, 3, 4], by: (x) { x % 2 })  // {1 -> [1, 3], 0 -> [2, 4]}
/// ```
group = (list: List(v), by: (v) -> k ? e ! n) -> Dict(k, List(v)) ? e ! n {
  dict.group(list, by)
}

/// Keeps the elements for which the function gives `True`, in order.
///
/// ```i
/// list.filter([1, 2, 3, 4], keeping: int.is_even)  // [2, 4]
/// ```
filter = (list: List(a), keeping: (a) -> Bool ? e ! n) -> List(a) ? e ! n {
  go = (rest, acc) {
    case rest {
      [] -> reverse(acc)
      [x, ..xs] -> case keeping(x) {
        True -> go(xs, [x, ..acc])
        False -> go(xs, acc)
      }
    }
  }
  go(list, [])
}

/// Calls a function on every element, and keeps the values of the `Some`
/// results.
///
/// ```i
/// list.filter_map(["1", "x", "3"], (s) { option.from_result(int.parse(s)) })  // [1, 3]
/// ```
filter_map = (list: List(a), with: (a) -> Option(b) ? e ! n) -> List(b) ? e ! n {
  go = (rest, acc) {
    case rest {
      [] -> reverse(acc)
      [x, ..xs] -> case with(x) {
        Some(y) -> go(xs, [y, ..acc])
        None -> go(xs, acc)
      }
    }
  }
  go(list, [])
}

/// Calls a function on every element, and gives the list of the results.
///
/// ```i
/// list.map([1, 2, 3], (x) { x * 2 })  // [2, 4, 6]
/// ```
map = (list: List(a), with: (a) -> b ? e ! n) -> List(b) ? e ! n {
  go = (rest, acc) {
    case rest {
      [] -> reverse(acc)
      [x, ..xs] -> go(xs, [with(x), ..acc])
    }
  }
  go(list, [])
}

/// Calls a function on the elements of two lists at the same places. When
/// one list is longer, its other elements are not used.
///
/// ```i
/// list.map2([1, 2, 3], [4, 5], (x, y) { x + y })  // [5, 7]
/// ```
map2 = (list1: List(a), list2: List(b), with: (a, b) -> c ? e ! n) -> List(c) ? e ! n {
  go = (xs, ys, acc) {
    case (xs, ys) {
      ([x, ..xs], [y, ..ys]) -> go(xs, ys, [with(x, y), ..acc])
      _ -> reverse(acc)
    }
  }
  go(list1, list2, [])
}

/// Maps the elements with an accumulator that goes from each element to the
/// next. Gives the last accumulator and the new list.
///
/// ```i
/// list.map_fold([1, 2, 3], from: 100, with: (acc, x) { (acc + x, x * 2) })
/// // (106, [2, 4, 6])
/// ```
map_fold = (over: List(a), from: acc, with: (acc, a) -> (acc, b) ? e ! n) -> (acc, List(b)) ? e ! n {
  go = (rest, acc, out) {
    case rest {
      [] -> (acc, reverse(out))
      [x, ..xs] -> {
        (next, y) = with(acc, x)
        go(xs, next, [y, ..out])
      }
    }
  }
  go(over, from, [])
}

/// Like `map`, and the function also gets the index of the element,
/// counting from 0.
index_map = (list: List(a), with: (a, Int) -> b ? e ! n) -> List(b) ? e ! n {
  go = (rest, i, acc) {
    case rest {
      [] -> reverse(acc)
      [x, ..xs] -> go(xs, i + 1, [with(x, i), ..acc])
    }
  }
  go(list, 0, [])
}

/// Calls a function that gives a result on every element. Gives `Ok` with
/// all the values, or the first `Error`; after an `Error`, the function is
/// not called again.
///
/// ```i
/// list.try_map(["1", "2"], int.parse)  // Ok([1, 2])
/// ```
try_map = (over: List(a), with: (a) -> Result(b, f) ? e ! n) -> Result(List(b), f) ? e ! n {
  go = (rest, acc) {
    case rest {
      [] -> Ok(reverse(acc))
      [x, ..xs] -> case with(x) {
        Ok(y) -> go(xs, [y, ..acc])
        Error(error) -> Error(error)
      }
    }
  }
  go(over, [])
}

/// The list without its first `up_to` elements. When the list is shorter,
/// the empty list.
///
/// ```i
/// list.drop([1, 2, 3, 4], up_to: 2)  // [3, 4]
/// ```
drop = (from: List(a), up_to: Int) -> List(a) {
  case up_to <= 0 {
    True -> from
    False -> case from {
      [] -> []
      [_, ..xs] -> drop(xs, up_to - 1)
    }
  }
}

/// The first `up_to` elements. When the list is shorter, the whole list.
///
/// ```i
/// list.take([1, 2, 3, 4], up_to: 2)  // [1, 2]
/// ```
take = (from: List(a), up_to: Int) -> List(a) {
  go = (rest, n, acc) {
    case n <= 0 {
      True -> reverse(acc)
      False -> case rest {
        [] -> reverse(acc)
        [x, ..xs] -> go(xs, n - 1, [x, ..acc])
      }
    }
  }
  go(from, up_to, [])
}

/// The empty list.
new = () -> List(a) { [] }

/// A list of one element.
wrap = (item: a) -> List(a) { [item] }

/// The elements of `first`, then the elements of `second`.
///
/// ```i
/// list.append([1, 2], [3])  // [1, 2, 3]
/// ```
append = (first: List(a), second: List(a)) -> List(a) {
  reverse_onto(reverse(first), second)
}

/// Puts an element in front of a list.
prepend = (to: List(a), this: a) -> List(a) { [this, ..to] }

/// The elements of all the lists, one list after the other.
///
/// ```i
/// list.concat([[1], [2, 3], []])  // [1, 2, 3]
/// ```
concat = (lists: List(List(a))) -> List(a) {
  go = (rest, acc) {
    case rest {
      [] -> reverse(acc)
      [xs, ..more] -> go(more, reverse_onto(xs, acc))
    }
  }
  go(lists, [])
}

/// Maps every element to a list, and puts these lists together.
///
/// ```i
/// list.flat_map([1, 2], (x) { [x, x] })  // [1, 1, 2, 2]
/// ```
flat_map = (over: List(a), with: (a) -> List(b) ? e ! n) -> List(b) ? e ! n {
  go = (rest, acc) {
    case rest {
      [] -> reverse(acc)
      [x, ..xs] -> go(xs, reverse_onto(with(x), acc))
    }
  }
  go(over, [])
}

/// Combines the elements with an accumulator, from the first to the last.
///
/// ```i
/// list.fold([1, 2, 3], from: 0, with: (acc, x) { acc + x })  // 6
/// ```
fold = (over: List(a), from: acc, with: (acc, a) -> acc ? e ! n) -> acc ? e ! n {
  case over {
    [] -> from
    [x, ..xs] -> fold(xs, with(from, x), with)
  }
}

/// Combines the elements with an accumulator, from the last to the first.
///
/// ```i
/// list.fold_right([1, 2, 3], from: [], with: (acc, x) { [x, ..acc] })  // [1, 2, 3]
/// ```
fold_right = (over: List(a), from: acc, with: (acc, a) -> acc ? e ! n) -> acc ? e ! n {
  fold(reverse(over), from, with)
}

/// Like `fold`, and the function also gets the index of the element.
index_fold = (over: List(a), from: acc, with: (acc, a, Int) -> acc ? e ! n) -> acc ? e ! n {
  go = (rest, acc, i) {
    case rest {
      [] -> acc
      [x, ..xs] -> go(xs, with(acc, x, i), i + 1)
    }
  }
  go(over, from, 0)
}

/// Like `fold`, but the function gives a result. The fold stops at the
/// first `Error`.
///
/// ```i
/// list.try_fold([1, 2, 3], from: 0, with: (acc, x) { Ok(acc + x) })  // Ok(6)
/// ```
try_fold = (over: List(a), from: acc, with: (acc, a) -> Result(acc, f) ? e ! n) -> Result(acc, f) ? e ! n {
  case over {
    [] -> Ok(from)
    [x, ..xs] -> case with(from, x) {
      Ok(next) -> try_fold(xs, next, with)
      Error(error) -> Error(error)
    }
  }
}

/// Like `fold`, but the function says whether to go on (`Continue`) or to
/// stop (`Stop`).
///
/// ```i
/// list.fold_until([1, 2, 3, 4], from: 0, with: (acc, x) {
///   case x < 3 { True -> list.Continue(acc + x)  False -> list.Stop(acc) }
/// })  // 3
/// ```
fold_until = (over: List(a), from: acc, with: (acc, a) -> ContinueOrStop(acc) ? e ! n) -> acc ? e ! n {
  case over {
    [] -> from
    [x, ..xs] -> case with(from, x) {
      Continue(next) -> fold_until(xs, next, with)
      Stop(last) -> last
    }
  }
}

/// The first element for which the function gives `True`, or `None`.
///
/// ```i
/// list.find([1, 2, 3], one_that: (x) { x > 1 })  // Some(2)
/// ```
find = (in: List(a), one_that: (a) -> Bool ? e ! n) -> Option(a) ? e ! n {
  case in {
    [] -> None
    [x, ..xs] -> case one_that(x) {
      True -> Some(x)
      False -> find(xs, one_that)
    }
  }
}

/// The first `Some` value that the function gives, or `None`.
find_map = (in: List(a), with: (a) -> Option(b) ? e ! n) -> Option(b) ? e ! n {
  case in {
    [] -> None
    [x, ..xs] -> case with(x) {
      Some(y) -> Some(y)
      None -> find_map(xs, with)
    }
  }
}

/// `True` when the function gives `True` for every element. It stops at the
/// first `False`. `True` for the empty list.
all = (in: List(a), satisfying: (a) -> Bool ? e ! n) -> Bool ? e ! n {
  case in {
    [] -> True
    [x, ..xs] -> satisfying(x) && all(xs, satisfying)
  }
}

/// `True` when the function gives `True` for some element. It stops at the
/// first `True`. `False` for the empty list.
any = (in: List(a), satisfying: (a) -> Bool ? e ! n) -> Bool ? e ! n {
  case in {
    [] -> False
    [x, ..xs] -> satisfying(x) || any(xs, satisfying)
  }
}

/// Pairs of the elements of two lists at the same places. When one list is
/// longer, its other elements are not used.
///
/// ```i
/// list.zip([1, 2, 3], ["a", "b"])  // [(1, "a"), (2, "b")]
/// ```
zip = (list: List(a), with: List(b)) -> List((a, b)) {
  go = (xs, ys, acc) {
    case (xs, ys) {
      ([x, ..xs], [y, ..ys]) -> go(xs, ys, [(x, y), ..acc])
      _ -> reverse(acc)
    }
  }
  go(list, with, [])
}

/// Like `zip`, but `Error(Nil)` when the lists have different lengths.
strict_zip = (list: List(a), with: List(b)) -> Result(List((a, b)), Nil) {
  go = (xs, ys, acc) {
    case (xs, ys) {
      ([], []) -> Ok(reverse(acc))
      ([x, ..xs], [y, ..ys]) -> go(xs, ys, [(x, y), ..acc])
      _ -> Error(Nil)
    }
  }
  go(list, with, [])
}

/// Takes a list of pairs apart into two lists.
///
/// ```i
/// list.unzip([(1, "a"), (2, "b")])  // ([1, 2], ["a", "b"])
/// ```
unzip = (input: List((a, b))) -> (List(a), List(b)) {
  go = (rest, xs, ys) {
    case rest {
      [] -> (reverse(xs), reverse(ys))
      [(x, y), ..more] -> go(more, [x, ..xs], [y, ..ys])
    }
  }
  go(input, [], [])
}

/// Puts `with` between every two elements.
///
/// ```i
/// list.intersperse([1, 2, 3], with: 0)  // [1, 0, 2, 0, 3]
/// ```
intersperse = (list: List(a), with: a) -> List(a) {
  case list {
    [] -> []
    [x, ..xs] -> {
      go = (rest, acc) {
        case rest {
          [] -> reverse(acc)
          [y, ..ys] -> go(ys, [y, with, ..acc])
        }
      }
      go(xs, [x])
    }
  }
}

/// The list without the elements that are `==` to an earlier element. The
/// first of each is kept, in order. It takes O(n log n) steps, with a dict.
///
/// ```i
/// list.unique([1, 2, 1, 3, 2])  // [1, 2, 3]
/// ```
unique = (list: List(a)) -> List(a) {
  go = (rest, seen, acc) {
    case rest {
      [] -> reverse(acc)
      [x, ..xs] -> case dict.has(seen, x) {
        True -> go(xs, seen, acc)
        False -> go(xs, dict.insert(seen, x, Nil), [x, ..acc])
      }
    }
  }
  go(list, dict.new(), [])
}

/// Sorts the elements with a compare function. The sort is stable: equal
/// elements keep their order. It is a merge sort, which takes O(n log n)
/// steps, and which starts from the runs that are already in order.
///
/// ```i
/// list.sort([3, 1, 2], by: int.compare)  // [1, 2, 3]
/// ```
sort = (list: List(a), by: (a, a) -> Order ? e ! n) -> List(a) ? e ! n {
  case list {
    [] | [_] -> list
    _ -> merge_all(runs(list, by, []), by)
  }
}

// Cuts a list into runs that are in order. A run that goes down strictly is
// turned round; one that goes down with equal elements is not taken as one
// run, so that the sort stays stable. The runs are given in order.
runs = (list: List(a), by: (a, a) -> Order ? e ! n, acc: List(List(a))) -> List(List(a)) ? e ! n {
  case list {
    [] -> reverse(acc)
    [x] -> reverse([[x], ..acc])
    [x, y, ..rest] -> case by(x, y) {
      Greater -> descending(rest, y, [x], by, acc)
      _ -> ascending(rest, y, [x], by, acc)
    }
  }
}

// `run` is in reverse; `last` comes after it.
ascending = (list: List(a), last: a, run: List(a), by: (a, a) -> Order ? e ! n, acc: List(List(a))) -> List(List(a)) ? e ! n {
  case list {
    [] -> reverse([reverse([last, ..run]), ..acc])
    [x, ..rest] -> case by(last, x) {
      Greater -> runs(list, by, [reverse([last, ..run]), ..acc])
      _ -> ascending(rest, x, [last, ..run], by, acc)
    }
  }
}

// `run` goes up (it is the strictly falling run, reversed); `last` comes
// before it.
descending = (list: List(a), last: a, run: List(a), by: (a, a) -> Order ? e ! n, acc: List(List(a))) -> List(List(a)) ? e ! n {
  case list {
    [] -> reverse([[last, ..run], ..acc])
    [x, ..rest] -> case by(last, x) {
      Greater -> descending(rest, x, [last, ..run], by, acc)
      _ -> runs(list, by, [[last, ..run], ..acc])
    }
  }
}

merge_all = (runs: List(List(a)), by: (a, a) -> Order ? e ! n) -> List(a) ? e ! n {
  case runs {
    [] -> []
    [run] -> run
    _ -> merge_all(merge_pairs(runs, by, []), by)
  }
}

merge_pairs = (runs: List(List(a)), by: (a, a) -> Order ? e ! n, acc: List(List(a))) -> List(List(a)) ? e ! n {
  case runs {
    [] -> reverse(acc)
    [run] -> reverse([run, ..acc])
    [a, b, ..rest] -> merge_pairs(rest, by, [merge(a, b, by, []), ..acc])
  }
}

// Merges two sorted lists. On a tie, the element of `xs` comes first.
merge = (xs: List(a), ys: List(a), by: (a, a) -> Order ? e ! n, acc: List(a)) -> List(a) ? e ! n {
  case xs {
    [] -> reverse_onto(acc, ys)
    [x, ..xs2] -> case ys {
      [] -> reverse_onto(acc, xs)
      [y, ..ys2] -> case by(x, y) {
        Greater -> merge(xs, ys2, by, [y, ..acc])
        _ -> merge(xs2, ys, by, [x, ..acc])
      }
    }
  }
}

/// The numbers from `from` to `to`, with both ends. It counts down when
/// `to` is less than `from`.
///
/// ```i
/// list.range(1, 4)  // [1, 2, 3, 4]
/// list.range(2, 0)  // [2, 1, 0]
/// ```
range = (from: Int, to: Int) -> List(Int) {
  go = (i, acc) {
    case i == from {
      True -> [i, ..acc]
      False -> case i < from {
        True -> go(i + 1, [i, ..acc])
        False -> go(i - 1, [i, ..acc])
      }
    }
  }
  go(to, [])
}

/// A list of `times` copies of `item`.
///
/// ```i
/// list.repeat("a", times: 3)  // ["a", "a", "a"]
/// ```
repeat = (item: a, times: Int) -> List(a) {
  go = (n, acc) {
    case n <= 0 {
      True -> acc
      False -> go(n - 1, [item, ..acc])
    }
  }
  go(times, [])
}

/// The first `at` elements, and the rest.
///
/// ```i
/// list.split([1, 2, 3, 4], at: 1)  // ([1], [2, 3, 4])
/// ```
split = (list: List(a), at: Int) -> (List(a), List(a)) {
  go = (rest, n, acc) {
    case n <= 0 {
      True -> (reverse(acc), rest)
      False -> case rest {
        [] -> (reverse(acc), [])
        [x, ..xs] -> go(xs, n - 1, [x, ..acc])
      }
    }
  }
  go(list, at, [])
}

/// The first elements for which the function gives `True`, and the rest.
split_while = (list: List(a), satisfying: (a) -> Bool ? e ! n) -> (List(a), List(a)) ? e ! n {
  go = (rest, acc) {
    case rest {
      [] -> (reverse(acc), [])
      [x, ..xs] -> case satisfying(x) {
        True -> go(xs, [x, ..acc])
        False -> (reverse(acc), rest)
      }
    }
  }
  go(list, [])
}

/// The value of the first pair whose key is `==` to `find`, in a list of
/// pairs, or `None`.
///
/// ```i
/// list.key_find([("a", 1), ("b", 2)], "b")  // Some(2)
/// ```
key_find = (in: List((k, v)), find: k) -> Option(v) {
  case in {
    [] -> None
    [(k, v), ..rest] -> case k == find {
      True -> Some(v)
      False -> key_find(rest, find)
    }
  }
}

/// The values of all the pairs whose key is `==` to `find`, in order.
key_filter = (in: List((k, v)), find: k) -> List(v) {
  go = (rest, acc) {
    case rest {
      [] -> reverse(acc)
      [(k, v), ..xs] -> case k == find {
        True -> go(xs, [v, ..acc])
        False -> go(xs, acc)
      }
    }
  }
  go(in, [])
}

/// The value of the first pair whose key is `==` to `key`, and the list
/// without that pair, or `None`.
///
/// ```i
/// list.key_pop([("a", 1), ("b", 2)], "a")  // Some((1, [("b", 2)]))
/// ```
key_pop = (list: List((k, v)), key: k) -> Option((v, List((k, v)))) {
  go = (rest, acc) {
    case rest {
      [] -> None
      [(k, v), ..xs] -> case k == key {
        True -> Some((v, reverse_onto(acc, xs)))
        False -> go(xs, [(k, v), ..acc])
      }
    }
  }
  go(list, [])
}

/// Sets the value of the first pair whose key is `==` to `key`, or adds the
/// pair at the end when there is none.
key_set = (list: List((k, v)), key: k, value: v) -> List((k, v)) {
  go = (rest, acc) {
    case rest {
      [] -> reverse([(key, value), ..acc])
      [(k, v), ..xs] -> case k == key {
        True -> reverse_onto(acc, [(key, value), ..xs])
        False -> go(xs, [(k, v), ..acc])
      }
    }
  }
  go(list, [])
}

/// Calls a function on every element, in order, for its effects.
each = (list: List(a), with: (a) -> b ? e ! n) -> Nil ? e ! n {
  case list {
    [] -> Nil
    [x, ..xs] -> {
      with(x)
      each(xs, with)
    }
  }
}

/// Calls a function that gives a result on every element, in order, and
/// stops at the first `Error`.
try_each = (over: List(a), with: (a) -> Result(b, f) ? e ! n) -> Result(Nil, f) ? e ! n {
  case over {
    [] -> Ok(Nil)
    [x, ..xs] -> case with(x) {
      Ok(_) -> try_each(xs, with)
      Error(error) -> Error(error)
    }
  }
}

/// The elements for which the function gives `True`, and the others, each
/// in order.
///
/// ```i
/// list.partition([1, 2, 3, 4], int.is_odd)  // ([1, 3], [2, 4])
/// ```
partition = (list: List(a), with: (a) -> Bool ? e ! n) -> (List(a), List(a)) ? e ! n {
  go = (rest, yes, no) {
    case rest {
      [] -> (reverse(yes), reverse(no))
      [x, ..xs] -> case with(x) {
        True -> go(xs, [x, ..yes], no)
        False -> go(xs, yes, [x, ..no])
      }
    }
  }
  go(list, [], [])
}

/// Every order of the elements. Elements are told apart by their place, so
/// equal elements give equal permutations more than once. The number of
/// permutations is n!, so this is only for short lists.
///
/// ```i
/// list.permutations([1, 2, 3])
/// // [[1, 2, 3], [1, 3, 2], [2, 1, 3], [2, 3, 1], [3, 1, 2], [3, 2, 1]]
/// ```
permutations = (list: List(a)) -> List(List(a)) {
  case list {
    [] -> [[]]
    _ -> {
      // Each element in turn, with the others in order.
      picks = (before, after, acc) {
        case after {
          [] -> reverse(acc)
          [x, ..xs] -> picks([x, ..before], xs, [(x, reverse_onto(before, xs)), ..acc])
        }
      }
      flat_map(picks([], list, []), (pick) {
        (x, others) = pick
        map(permutations(others), (p) { [x, ..p] })
      })
    }
  }
}

/// Every run of `by` elements that come one after the other, in order.
///
/// ```i
/// list.window([1, 2, 3, 4], by: 2)  // [[1, 2], [2, 3], [3, 4]]
/// ```
window = (list: List(a), by: Int) -> List(List(a)) {
  case by <= 0 {
    True -> []
    False -> {
      go = (rest, acc) {
        w = take(rest, by)
        case length(w) == by {
          True -> case rest {
            [_, ..xs] -> go(xs, [w, ..acc])
            [] -> reverse(acc)
          }
          False -> reverse(acc)
        }
      }
      go(list, [])
    }
  }
}

/// Every pair of elements that come one after the other.
///
/// ```i
/// list.window_by_2([1, 2, 3])  // [(1, 2), (2, 3)]
/// ```
window_by_2 = (list: List(a)) -> List((a, a)) {
  case list {
    [] -> []
    [_, ..xs] -> zip(list, xs)
  }
}

/// The list without its first elements for which the function gives
/// `True`.
drop_while = (in: List(a), satisfying: (a) -> Bool ? e ! n) -> List(a) ? e ! n {
  case in {
    [] -> []
    [x, ..xs] -> case satisfying(x) {
      True -> drop_while(xs, satisfying)
      False -> in
    }
  }
}

/// The first elements for which the function gives `True`.
take_while = (in: List(a), satisfying: (a) -> Bool ? e ! n) -> List(a) ? e ! n {
  go = (rest, acc) {
    case rest {
      [] -> reverse(acc)
      [x, ..xs] -> case satisfying(x) {
        True -> go(xs, [x, ..acc])
        False -> reverse(acc)
      }
    }
  }
  go(in, [])
}

/// Cuts the list into runs of elements, one after the other, for which the
/// function gives the same key.
///
/// ```i
/// list.chunk([1, 2, 2, 3, 4, 4, 6], by: (x) { x % 2 })
/// // [[1], [2, 2], [3], [4, 4, 6]]
/// ```
chunk = (in: List(a), by: (a) -> k ? e ! n) -> List(List(a)) ? e ! n {
  case in {
    [] -> []
    [x, ..xs] -> {
      go = (rest, key, current, acc) {
        case rest {
          [] -> reverse([reverse(current), ..acc])
          [y, ..ys] -> {
            k = by(y)
            case k == key {
              True -> go(ys, key, [y, ..current], acc)
              False -> go(ys, k, [y], [reverse(current), ..acc])
            }
          }
        }
      }
      go(xs, by(x), [x], [])
    }
  }
}

/// Cuts the list into lists of `into` elements. The last list may be
/// shorter. When `into` is 0 or less, the empty list.
///
/// ```i
/// list.sized_chunk([1, 2, 3, 4, 5], into: 2)  // [[1, 2], [3, 4], [5]]
/// ```
sized_chunk = (in: List(a), into: Int) -> List(List(a)) {
  case into <= 0 {
    True -> []
    False -> {
      go = (rest, n, current, acc) {
        case rest {
          [] -> case current {
            [] -> reverse(acc)
            _ -> reverse([reverse(current), ..acc])
          }
          [x, ..xs] -> case n == 1 {
            True -> go(xs, into, [], [reverse([x, ..current]), ..acc])
            False -> go(xs, n - 1, [x, ..current], acc)
          }
        }
      }
      go(in, into, [], [])
    }
  }
}

/// Combines the elements with a function, from the first to the last, with
/// the first element as the start. `None` for the empty list.
///
/// ```i
/// list.reduce([1, 2, 3], with: (a, b) { a + b })  // Some(6)
/// ```
reduce = (over: List(a), with: (a, a) -> a ? e ! n) -> Option(a) ? e ! n {
  case over {
    [] -> None
    [x, ..xs] -> Some(fold(xs, x, with))
  }
}

/// Like `fold`, but gives every accumulator after each element.
///
/// ```i
/// list.scan([1, 2, 3], from: 0, with: (acc, x) { acc + x })  // [1, 3, 6]
/// ```
scan = (over: List(a), from: acc, with: (acc, a) -> acc ? e ! n) -> List(acc) ? e ! n {
  go = (rest, acc, out) {
    case rest {
      [] -> reverse(out)
      [x, ..xs] -> {
        next = with(acc, x)
        go(xs, next, [next, ..out])
      }
    }
  }
  go(over, from, [])
}

/// Every way to choose `by` elements, keeping their order.
///
/// ```i
/// list.combinations([1, 2, 3], by: 2)  // [[1, 2], [1, 3], [2, 3]]
/// ```
combinations = (items: List(a), by: Int) -> List(List(a)) {
  case by <= 0 {
    True -> [[]]
    False -> case items {
      [] -> []
      [x, ..xs] -> append(map(combinations(xs, by - 1), (c) { [x, ..c] }), combinations(xs, by))
    }
  }
}

/// Every pair of two elements, keeping their order.
///
/// ```i
/// list.combination_pairs([1, 2, 3])  // [(1, 2), (1, 3), (2, 3)]
/// ```
combination_pairs = (items: List(a)) -> List((a, a)) {
  go = (rest, acc) {
    case rest {
      [] -> reverse(acc)
      [x, ..xs] -> go(xs, reverse_onto(map(xs, (y) { (x, y) }), acc))
    }
  }
  go(items, [])
}

/// The first elements of all the lists, then the second elements, and so on.
///
/// ```i
/// list.interleave([[1, 2], [101, 102], [201]])  // [1, 101, 201, 2, 102]
/// ```
interleave = (list: List(List(a))) -> List(a) { concat(transpose(list)) }

/// Turns rows into columns. Shorter lists leave gaps, which are taken out.
///
/// ```i
/// list.transpose([[1, 2, 3], [101, 102, 103]])  // [[1, 101], [2, 102], [3, 103]]
/// ```
transpose = (list_of_lists: List(List(a))) -> List(List(a)) {
  go = (rows, acc) {
    nonempty = filter(rows, (row) { row != [] })
    case nonempty {
      [] -> reverse(acc)
      _ -> {
        heads = filter_map(nonempty, first)
        tails = map(nonempty, (row) { drop(row, 1) })
        go(tails, [heads, ..acc])
      }
    }
  }
  go(list_of_lists, [])
}

/// The greatest element by a compare function, or `None` for the empty
/// list. Of equal greatest elements, the first is given.
max = (over: List(a), with: (a, a) -> Order ? e ! n) -> Option(a) ? e ! n {
  case over {
    [] -> None
    [x, ..xs] -> Some(fold(xs, x, (best, y) {
      case with(y, best) {
        Greater -> y
        _ -> best
      }
    }))
  }
}

/// The smallest element by a compare function, or `None` for the empty
/// list. Of equal smallest elements, the first is given.
min = (over: List(a), with: (a, a) -> Order ? e ! n) -> Option(a) ? e ! n {
  case over {
    [] -> None
    [x, ..xs] -> Some(fold(xs, x, (best, y) {
      case with(y, best) {
        Less -> y
        _ -> best
      }
    }))
  }
}
