// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

pub fn prepend_1_test() -> Nil {
  let left = [1, ..[]]
  let right = [1]
  assert left == right
  Nil
}

pub fn prepend_2_test() -> Nil {
  let left = [1, 2, ..[]]
  let right = [1, 2]
  assert left == right
  Nil
}

pub fn prepend_3_test() -> Nil {
  let left = [1, 2, ..[3]]
  let right = [1, 2, 3]
  assert left == right
  Nil
}

pub fn prepend_4_test() -> Nil {
  let left = [1, 2, ..[3, 4]]
  let right = [1, 2, 3, 4]
  assert left == right
  Nil
}
