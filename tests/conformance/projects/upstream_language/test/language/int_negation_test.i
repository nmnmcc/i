// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

pub fn negation_1_test() -> Nil {
  let a = 3
  let b = -a
  assert -3 == b
  Nil
}

pub fn negation_2_test() -> Nil {
  let a = 3
  let b = -{ -a }
  assert 3 == b
  Nil
}

pub fn negation_3_test() -> Nil {
  let a = 3
  let b = -{ -{ -a } }
  assert -3 == b
  Nil
}

pub fn negation_4_test() -> Nil {
  let a = 3
  let b = -a
  let c = a - -b
  assert 0 == c
  Nil
}

pub fn negation_5_test() -> Nil {
  let a = 3
  let b = -a
  let c = a - -{ -{ -{ -{ -b } } } }
  assert 0 == c
  Nil
}

pub fn negation_6_test() -> Nil {
  let a = 3
  let b = -a
  let c = -a - -b
  assert -6 == c
  Nil
}

pub fn negation_7_test() -> Nil {
  let abs = fn(value) {
    case value {
      value if value > 0 -> value
      _ -> -value
    }
  }
  assert -6 == -abs(-6)
  Nil
}
