import i/int
import i/list
import i/set

pub fn size_empty_test() -> Nil {
  assert set.size(set.new()) == 0
  Nil
}

pub fn size_with_elements_test() -> Nil {
  assert set.new()
    |> set.insert(1)
    |> set.insert(2)
    |> set.size
    == 2
  Nil
}

pub fn size_with_duplicates_test() -> Nil {
  assert set.new()
    |> set.insert(1)
    |> set.insert(1)
    |> set.insert(2)
    |> set.size
    == 2
  Nil
}

pub fn is_empty_new_test() -> Nil {
  assert set.is_empty(set.new())
  Nil
}

pub fn is_empty_with_element_test() -> Nil {
  assert !{
    set.new()
    |> set.insert(1)
    |> set.is_empty()
  }
  Nil
}

pub fn is_empty_after_delete_test() -> Nil {
  assert set.new()
    |> set.insert(1)
    |> set.delete(1)
    |> set.is_empty()
  Nil
}

pub fn contains_present_test() -> Nil {
  assert set.new()
    |> set.insert(1)
    |> set.contains(this: 1)
  Nil
}

pub fn contains_absent_test() -> Nil {
  assert !set.contains(set.new(), this: 1)
  Nil
}

pub fn delete_test() -> Nil {
  assert !{
    set.new()
    |> set.insert(1)
    |> set.delete(1)
    |> set.contains(1)
  }
  Nil
}

pub fn to_list_test() -> Nil {
  assert set.new()
    |> set.insert(2)
    |> set.insert(3)
    |> set.insert(4)
    |> set.to_list
    |> list.sort(by: int.compare)
    == [2, 3, 4]
  Nil
}

pub fn from_list_test() -> Nil {
  assert [1, 1, 2, 4, 3, 2]
    |> set.from_list
    |> set.to_list
    |> list.sort(by: int.compare)
    == [1, 2, 3, 4]
  Nil
}

pub fn fold_test() -> Nil {
  assert [1, 3, 9]
    |> set.from_list
    |> set.fold(from: 0, with: fn(m, a) { m + a })
    == 13
  Nil
}

pub fn map_test() -> Nil {
  assert [1, 2, 3, 4]
    |> set.from_list
    |> set.map(with: int.to_string)
    == set.from_list(["1", "2", "3", "4"])
  Nil
}

pub fn filter_test() -> Nil {
  assert [1, 4, 6, 3, 675, 44, 67]
    |> set.from_list()
    |> set.filter(keeping: int.is_even)
    |> set.to_list
    |> list.sort(int.compare)
    == [4, 6, 44]
  Nil
}

pub fn take_test() -> Nil {
  assert [1, 2, 3]
    |> set.from_list
    |> set.take([1, 3, 5])
    == set.from_list([1, 3])
  Nil
}

pub fn drop_test() -> Nil {
  assert ["a", "b", "c"]
    |> set.from_list
    |> set.drop(["a", "b", "d"])
    == set.from_list(["c"])
  Nil
}

pub fn union_test() -> Nil {
  assert set.union(set.from_list([1, 2]), set.from_list([2, 3]))
    |> set.to_list
    |> list.sort(int.compare)
    == [1, 2, 3]
  Nil
}

pub fn intersection_test() -> Nil {
  assert set.to_list(set.intersection(
      set.from_list([1, 2]),
      set.from_list([2, 3]),
    ))
    == [2]
  Nil
}

pub fn difference_test() -> Nil {
  assert set.to_list(set.difference(
      set.from_list([1, 2]),
      set.from_list([2, 3, 4]),
    ))
    == [1]
  Nil
}

pub fn is_subset_false_superset_test() -> Nil {
  assert !set.is_subset(set.from_list([1, 2, 3, 4]), set.from_list([2, 4]))
  Nil
}

pub fn is_subset_true_test() -> Nil {
  assert set.is_subset(set.from_list([2, 4]), set.from_list([1, 2, 3, 4]))
  Nil
}

pub fn is_subset_disjoint_test() -> Nil {
  assert !set.is_subset(set.from_list([1, 2, 3]), set.from_list([4, 5, 6]))
  Nil
}

pub fn is_subset_partial_overlap_test() -> Nil {
  assert !set.is_subset(set.from_list([1, 2]), set.from_list([2, 3, 4]))
  Nil
}

pub fn is_disjoint_true_test() -> Nil {
  assert set.is_disjoint(set.from_list([1, 2, 3]), set.from_list([4, 5, 6]))
  Nil
}

pub fn is_disjoint_false_test() -> Nil {
  assert !set.is_disjoint(set.from_list([1, 2]), set.from_list([2, 3, 4]))
  Nil
}

pub fn symmetric_difference_test() -> Nil {
  assert set.symmetric_difference(
      set.from_list([1, 2, 3]),
      set.from_list([3, 4]),
    )
    == set.from_list([1, 2, 4])
  Nil
}

pub fn each_test() -> Nil {
  assert [1, 2, 3]
    |> set.from_list
    |> set.each(fn(member) {
      case member {
        1 | 2 | 3 -> Nil
        _ -> panic as "unexpected value"
      }
    })
    == Nil
  Nil
}
