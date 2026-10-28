// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

pub fn list_eq_1_test() -> Nil {
  let left = []
  let right = []
  assert left == right
  Nil
}

pub fn list_eq_2_test() -> Nil {
  let left = []
  let right = [0]
  assert left != right
  Nil
}

pub fn list_eq_3_test() -> Nil {
  let left = [0]
  let right = []
  assert left != right
  Nil
}

pub fn list_eq_4_test() -> Nil {
  let left = [0]
  let right = [0]
  assert left == right
  Nil
}

pub fn list_neq_1_test() -> Nil {
  let left = []
  let right = []
  assert left == right
  Nil
}

pub fn list_neq_2_test() -> Nil {
  let left = []
  let right = [0]
  assert left != right
  Nil
}

pub fn list_neq_3_test() -> Nil {
  let left = [0]
  let right = []
  assert left != right
  Nil
}

pub fn list_neq_4_test() -> Nil {
  let left = [0]
  let right = [0]
  assert left == right
  Nil
}

pub fn int_eq_1_test() -> Nil {
  let left = 0
  let right = 0
  assert left == right
  Nil
}

pub fn int_neq_1_test() -> Nil {
  let left = 0
  let right = 0
  assert left == right
  Nil
}

pub fn int_eq_2_test() -> Nil {
  let left = 1
  let right = 0
  assert left != right
  Nil
}

pub fn int_neq_2_test() -> Nil {
  let left = 1
  let right = 0
  assert left != right
  Nil
}

pub fn int_eq_3_test() -> Nil {
  let left = 1
  let right = 1
  assert left == right
  Nil
}

pub fn int_neq_3_test() -> Nil {
  let left = 1
  let right = 1
  assert left == right
  Nil
}

pub fn bit_array_eq_1_test() -> Nil {
  let left = <<>>
  let right = <<>>
  assert left == right
  Nil
}

pub fn bit_array_neq_1_test() -> Nil {
  let left = <<>>
  let right = <<>>
  assert left == right
  Nil
}

pub fn bit_array_eq_2_test() -> Nil {
  let left = <<1, 2>>
  let right = <<1, 2>>
  assert left == right
  Nil
}

pub fn bit_array_neq_2_test() -> Nil {
  let left = <<1, 2>>
  let right = <<1, 2>>
  assert left == right
  Nil
}

pub fn bit_array_eq_3_test() -> Nil {
  let left = <<1, 2>>
  let right = <<2>>
  assert left != right
  Nil
}

pub fn bit_array_neq_3_test() -> Nil {
  let left = <<1, 2>>
  let right = <<2>>
  assert left != right
  Nil
}

pub fn record_ok_eq_1_test() -> Nil {
  let left = Ok(1)
  let right = Ok(1)
  assert left == right
  Nil
}

pub fn record_ok_neq_1_test() -> Nil {
  let left = Ok(1)
  let right = Ok(1)
  assert left == right
  Nil
}

pub fn record_ok_eq_2_test() -> Nil {
  let left = Ok(2)
  let right = Ok(1)
  assert left != right
  Nil
}

pub fn record_ok_neq_2_test() -> Nil {
  let left = Ok(2)
  let right = Ok(1)
  assert left != right
  Nil
}

pub fn record_error_eq_1_test() -> Nil {
  let left = Error(1)
  let right = Error(1)
  assert left == right
  Nil
}

pub fn record_error_neq_1_test() -> Nil {
  let left = Error(1)
  let right = Error(1)
  assert left == right
  Nil
}

pub fn record_error_eq_2_test() -> Nil {
  let left = Error(2)
  let right = Error(1)
  assert left != right
  Nil
}

pub fn record_error_neq_2_test() -> Nil {
  let left = Error(2)
  let right = Error(1)
  assert left != right
  Nil
}
