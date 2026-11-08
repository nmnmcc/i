import i/pair

pub fn first_int_test() -> Nil {
  assert pair.first(#(1, 2)) == 1
  Nil
}

pub fn first_string_test() -> Nil {
  assert pair.first(#("abc", [])) == "abc"
  Nil
}

pub fn second_int_test() -> Nil {
  assert pair.second(#(1, 2)) == 2
  Nil
}

pub fn second_list_test() -> Nil {
  assert pair.second(#("abc", [])) == []
  Nil
}

pub fn swap_test() -> Nil {
  assert pair.swap(#(1, "2")) == #("2", 1)
  Nil
}

pub fn map_first_basic_test() -> Nil {
  let inc = fn(a) { a + 1 }
  assert pair.map_first(#(1, 2), inc) == #(2, 2)
  Nil
}

pub fn map_first_larger_test() -> Nil {
  let inc = fn(a) { a + 1 }
  assert pair.map_first(#(8, 2), inc) == #(9, 2)
  Nil
}

pub fn map_first_zero_test() -> Nil {
  let inc = fn(a) { a + 1 }
  assert pair.map_first(#(0, -2), inc) == #(1, -2)
  Nil
}

pub fn map_first_negative_test() -> Nil {
  let inc = fn(a) { a + 1 }
  assert pair.map_first(#(-10, 20), inc) == #(-9, 20)
  Nil
}

pub fn map_second_basic_test() -> Nil {
  let dec = fn(a) { a - 1 }
  assert pair.map_second(#(1, 2), dec) == #(1, 1)
  Nil
}

pub fn map_second_larger_test() -> Nil {
  let dec = fn(a) { a - 1 }
  assert pair.map_second(#(8, 2), dec) == #(8, 1)
  Nil
}

pub fn map_second_negative_test() -> Nil {
  let dec = fn(a) { a - 1 }
  assert pair.map_second(#(0, -2), dec) == #(0, -3)
  Nil
}

pub fn map_second_mixed_test() -> Nil {
  let dec = fn(a) { a - 1 }
  assert pair.map_second(#(-10, 20), dec) == #(-10, 19)
  Nil
}

pub fn new_test() -> Nil {
  assert pair.new(1, 2) == #(1, 2)
  Nil
}
