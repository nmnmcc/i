// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

pub fn remainder_1_test() -> Nil {
  assert 1 % 1 == 0
  Nil
}

pub fn remainder_2_test() -> Nil {
  assert 1 % 0 == 0
  Nil
}

pub fn remainder_3_test() -> Nil {
  assert 3 % 2 == 1
  Nil
}

pub fn remainder_4_test() -> Nil {
  assert 3 % 0 == 0
  Nil
}

pub fn remainder_5_test() -> Nil {
  assert 3 % -2 == 1
  Nil
}

pub fn remainder_6_test() -> Nil {
  assert 3 % -0 == 0
  Nil
}

pub fn remainder_7_test() -> Nil {
  assert -13 % 3 == -1
  Nil
}

pub fn remainder_8_test() -> Nil {
  assert 13 % -3 == 1
  Nil
}

pub fn remainder_9_test() -> Nil {
  assert -13 % -3 == -1
  Nil
}
