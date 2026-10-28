// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

fn make_int_zero() {
  0
}

pub fn numbers_test() -> Nil {
  let int = 4
  let int_2 = case int {
    1 | 2 | 3 | 4 -> 0
    _ -> 1
  }
  assert 0 == int_2
  Nil
}

pub fn lists_test() -> Nil {
  let ints = [1, 2]
  let int = case ints {
    [0] | [1, 2] -> 0
    _ -> 1
  }
  assert 0 == int
  Nil
}

pub fn assignment_test() -> Nil {
  let ints = [1, 2]
  let int = case ints {
    [x] | [_, x] -> x
    _ -> 0
  }
  assert 2 == int
  Nil
}

pub fn multiple_assignment_test() -> Nil {
  let ints = [1, 2, 3]
  let value = case ints {
    [x, y] | [x, y, 3] -> #(x, y)
    _ -> #(0, 0)
  }
  assert #(1, 2) == value
  Nil
}

pub fn guard_test() -> Nil {
  let int_two = make_int_zero() + 2
  let ints = [1, 2]
  let int = case ints {
    [x] | [_, x] if x == int_two -> x
    _ -> 0
  }
  assert 2 == int
  Nil
}

pub fn guard_left_hand_side_test() -> Nil {
  let int_one = make_int_zero() + 1
  let ints = [1]
  let int = case ints {
    [x] | [_, x] if x == int_one -> x
    _ -> 0
  }
  assert 1 == int
  Nil
}
