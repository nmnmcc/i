// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

fn identity(x: a) -> a {
  x
}

fn pair(x: a, y: b) -> #(a, b) {
  #(x, y)
}

fn triplet(x x: a, y y: b, z z: c) -> #(a, b, c) {
  #(x, y, z)
}

pub fn pipe_last_test() -> Nil {
  let result =
    100
    |> identity
  assert 100 == result
  Nil
}

pub fn pipe_into_anon_test() -> Nil {
  let result =
    100
    |> fn(x) { x }
  assert 100 == result
  Nil
}

pub fn pipe_into_capture_test() -> Nil {
  let result =
    1
    |> pair(2, _)
  assert #(2, 1) == result
  Nil
}

pub fn pipe_first_test() -> Nil {
  let result =
    1
    |> pair(2)
  assert #(1, 2) == result
  Nil
}

pub fn pipe_middle_with_label_requires_false_capture_test() -> Nil {
  let result =
    2
    |> triplet(z: 3, x: 1)
  assert #(1, 2, 3) == result
  Nil
}

pub fn pipe_last_with_label_requires_false_capture_test() -> Nil {
  let result =
    3
    |> triplet(y: 2, x: 1)
  assert #(1, 2, 3) == result
  Nil
}
