import (
  i/core
  set = std/set
  int = std/int
  list = std/list
  option = (Some, None) = std/option
)


@test
size_empty_test = () {
  assert(set.size(set.new()) == 0)
  Nil
}

@test
size_with_elements_test = () {
  assert(set.new()
    |> set.insert(1)
    |> set.insert(2)
    |> set.size
    == 2)
  Nil
}

@test
size_with_duplicates_test = () {
  assert(set.new()
    |> set.insert(1)
    |> set.insert(1)
    |> set.insert(2)
    |> set.size
    == 2)
  Nil
}

@test
is_empty_new_test = () {
  assert(set.is_empty(set.new()))
  Nil
}

@test
is_empty_with_element_test = () {
  assert(core.not(set.new()
    |> set.insert(1)
    |> set.is_empty()))
  Nil
}

@test
is_empty_after_delete_test = () {
  assert(set.new()
    |> set.insert(1)
    |> set.delete(1)
    |> set.is_empty())
  Nil
}

@test
contains_present_test = () {
  assert(set.new()
    |> set.insert(1)
    |> set.contains(this: 1))
  Nil
}

@test
contains_absent_test = () {
  assert(core.not(set.contains(set.new(), this: 1)))
  Nil
}

@test
delete_test = () {
  assert(core.not(set.new()
    |> set.insert(1)
    |> set.delete(1)
    |> set.contains(1)))
  Nil
}

@test
to_list_test = () {
  assert(set.new()
    |> set.insert(2)
    |> set.insert(3)
    |> set.insert(4)
    |> set.to_list
    |> list.sort(by: int.compare)
    == [2, 3, 4])
  Nil
}

@test
from_list_test = () {
  assert([1, 1, 2, 4, 3, 2]
    |> set.from_list
    |> set.to_list
    |> list.sort(by: int.compare)
    == [1, 2, 3, 4])
  Nil
}

@test
fold_test = () {
  assert([1, 3, 9]
    |> set.from_list
    |> set.fold(from: 0, with: (m, a) { m + a })
    == 13)
  Nil
}

@test
map_test = () {
  assert([1, 2, 3, 4]
    |> set.from_list
    |> set.map(with: int.to_string)
    == set.from_list(["1", "2", "3", "4"]))
  Nil
}

@test
filter_test = () {
  assert([1, 4, 6, 3, 675, 44, 67]
    |> set.from_list()
    |> set.filter(keeping: int.is_even)
    |> set.to_list
    |> list.sort(int.compare)
    == [4, 6, 44])
  Nil
}

@test
take_test = () {
  assert([1, 2, 3]
    |> set.from_list
    |> set.take([1, 3, 5])
    == set.from_list([1, 3]))
  Nil
}

@test
drop_test = () {
  assert(["a", "b", "c"]
    |> set.from_list
    |> set.drop(["a", "b", "d"])
    == set.from_list(["c"]))
  Nil
}

@test
union_test = () {
  assert(set.union(set.from_list([1, 2]), set.from_list([2, 3]))
    |> set.to_list
    |> list.sort(int.compare)
    == [1, 2, 3])
  Nil
}

@test
intersection_test = () {
  assert(set.to_list(set.intersection(
      set.from_list([1, 2]),
      set.from_list([2, 3]),
    ))
    == [2])
  Nil
}

@test
difference_test = () {
  assert(set.to_list(set.difference(
      set.from_list([1, 2]),
      set.from_list([2, 3, 4]),
    ))
    == [1])
  Nil
}

@test
is_subset_false_superset_test = () {
  assert(core.not(set.is_subset(set.from_list([1, 2, 3, 4]), set.from_list([2, 4]))))
  Nil
}

@test
is_subset_true_test = () {
  assert(set.is_subset(set.from_list([2, 4]), set.from_list([1, 2, 3, 4])))
  Nil
}

@test
is_subset_disjoint_test = () {
  assert(core.not(set.is_subset(set.from_list([1, 2, 3]), set.from_list([4, 5, 6]))))
  Nil
}

@test
is_subset_partial_overlap_test = () {
  assert(core.not(set.is_subset(set.from_list([1, 2]), set.from_list([2, 3, 4]))))
  Nil
}

@test
is_disjoint_true_test = () {
  assert(set.is_disjoint(set.from_list([1, 2, 3]), set.from_list([4, 5, 6])))
  Nil
}

@test
is_disjoint_false_test = () {
  assert(core.not(set.is_disjoint(set.from_list([1, 2]), set.from_list([2, 3, 4]))))
  Nil
}

@test
symmetric_difference_test = () {
  assert(set.symmetric_difference(
      set.from_list([1, 2, 3]),
      set.from_list([3, 4]),
    )
    == set.from_list([1, 2, 4]))
  Nil
}

@test
each_test = () {
  assert([1, 2, 3]
    |> set.from_list
    |> set.each((member) {
      case member {
        1 | 2 | 3 -> Nil
        _ -> panic("unexpected value")
      }
    })
    == Nil)
  Nil
}

@test
set_equality_test = () {
  assert(set.from_list([1, 2, 3]) == set.from_list([3, 2, 1, 2]))
  assert(set.delete(set.from_list([1, 2]), 2) == set.from_list([1]))
}

@test
set_ops_test = () {
  a = set.from_list([1, 2, 3, 4])
  b = set.from_list([3, 4, 5])
  assert(set.union(a, b) == set.from_list([1, 2, 3, 4, 5]))
  assert(set.intersection(a, b) == set.from_list([3, 4]))
  assert(set.difference(a, b) == set.from_list([1, 2]))
  assert(set.symmetric_difference(a, b) == set.from_list([1, 2, 5]))
  assert(set.is_subset(set.from_list([3]), b))
  assert(core.not(set.is_subset(a, b)))
  assert(set.is_disjoint(set.from_list([1]), b))
  assert(core.not(set.is_disjoint(a, b)))
}

@test
set_many_test = () {
  s = set.from_list(list.map(list.range(1, 50_000), (i) { i % 1000 }))
  assert(set.size(s) == 1000)
  assert(set.contains(s, 999))
  assert(core.not(set.contains(s, 1000)))
  assert(list.sort(set.to_list(s), int.compare) == list.range(0, 999))
}

@test
set_map_filter_test = () {
  s = set.from_list([1, 2, 3, 4])
  assert(set.map(s, (x) { x % 2 }) == set.from_list([0, 1]))
  assert(set.filter(s, int.is_even) == set.from_list([2, 4]))
  assert(set.fold(s, 0, int.add) == 10)
}
