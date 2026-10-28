// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

pub fn addition_1_test() -> Nil {
  assert 0 + 0 == 0
  Nil
}

pub fn addition_2_test() -> Nil {
  assert 1 + 1 == 2
  Nil
}

pub fn addition_3_test() -> Nil {
  assert 5 + 1 == 6
  Nil
}

pub fn addition_4_test() -> Nil {
  assert 1 + 3 == 4
  Nil
}

pub fn addition_5_test() -> Nil {
  assert 1 + -3 == -2
  Nil
}

pub fn subtraction_1_test() -> Nil {
  assert 0 - 0 == 0
  Nil
}

pub fn subtraction_2_test() -> Nil {
  assert 1 - 1 == 0
  Nil
}

pub fn subtraction_3_test() -> Nil {
  assert 5 - 1 == 4
  Nil
}

pub fn subtraction_4_test() -> Nil {
  assert 1 - 3 == -2
  Nil
}

pub fn subtraction_5_test() -> Nil {
  assert 1 - -3 == 4
  Nil
}

pub fn multiplication_1_test() -> Nil {
  assert 0 * 0 == 0
  Nil
}

pub fn multiplication_2_test() -> Nil {
  assert 1 * 1 == 1
  Nil
}

pub fn multiplication_3_test() -> Nil {
  assert 2 * 2 == 4
  Nil
}

pub fn multiplication_4_test() -> Nil {
  assert 2 * 4 == 8
  Nil
}

pub fn multiplication_5_test() -> Nil {
  assert -2 * 4 == -8
  Nil
}

pub fn multiplication_6_test() -> Nil {
  assert 2 * -4 == -8
  Nil
}

pub fn precedence_1_test() -> Nil {
  assert 2 * 2 + 3 == 7
  Nil
}

pub fn precedence_2_test() -> Nil {
  assert 2 + 2 * 3 == 8
  Nil
}

pub fn precedence_3_test() -> Nil {
  assert 2 * { 2 + 3 } == 10
  Nil
}

pub fn precedence_4_test() -> Nil {
  assert { 2 + 2 } * 3 == 12
  Nil
}

pub fn hex_int_test() -> Nil {
  let int = 15
  assert 0xF == int
  Nil
}

pub fn octal_int_test() -> Nil {
  let int = 15
  assert 0o17 == int
  Nil
}

pub fn binary_int_test() -> Nil {
  let int = 15
  assert 0b00001111 == int
  Nil
}

pub fn lex_1_test() -> Nil {
  assert 1 - 1 == 0
  Nil
}

pub fn lex_2_test() -> Nil {
  let a = 1
  assert a - 1 == 0
  Nil
}

pub fn lex_3_test() -> Nil {
  assert 1 - 1 == 0
  Nil
}

pub fn division_1_test() -> Nil {
  assert 1 / 1 == 1
  Nil
}

pub fn division_2_test() -> Nil {
  assert 1 / 0 == 0
  Nil
}

pub fn division_3_test() -> Nil {
  assert 3 / 2 == 1
  Nil
}

pub fn division_4_test() -> Nil {
  assert 3 / 0 == 0
  Nil
}

pub fn negative_int_test() -> Nil {
  let one = -1
  let other = 1
  assert one != other
  assert one * -1 == other
  Nil
}

pub fn negative_zero_same_as_positive_zero_test() -> Nil {
  let negative = -0
  let positive = 0
  assert negative == positive
  Nil
}
