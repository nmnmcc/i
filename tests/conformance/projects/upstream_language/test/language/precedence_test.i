// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

pub fn precedence_1_test() -> Nil {
  assert 7 == 1 + 2 * 3
  Nil
}

pub fn precedence_2_test() -> Nil {
  assert 5 == 3 * 1 + 2
  Nil
}

pub fn precedence_3_test() -> Nil {
  assert 9 == { 1 + 2 } * 3
  Nil
}

pub fn precedence_4_test() -> Nil {
  assert 9 == 3 * { 1 + 2 }
  Nil
}

pub fn precedence_5_test() -> Nil {
  assert 11 == 1 + 2 * 3 + 4
  Nil
}

pub fn precedence_6_test() -> Nil {
  assert 26 == 2 * 3 + 4 * 5
  Nil
}

pub fn precedence_7_test() -> Nil {
  assert 4 == 2 * { 3 + 1 } / 2
  Nil
}

pub fn precedence_8_test() -> Nil {
  assert -17 == 5 + 3 / 3 * 2 - 6 * 4
  Nil
}

pub fn precedence_9_test() -> Nil {
  assert -31 == -5 + -3 / -3 * -2 - -6 * -4
  Nil
}

pub fn precedence_10_test() -> Nil {
  let a = 5
  let b = 3
  let c = 3
  let d = 2
  let e = 6
  let f = 4
  assert -17 == a + b / c * d - e * f
  Nil
}

pub fn precedence_11_test() -> Nil {
  let a = 5
  let b = 3
  let c = 3
  let d = 2
  let e = 6
  let f = 4
  assert -31 == -a + -b / -c * -d - -e * -f
  Nil
}
