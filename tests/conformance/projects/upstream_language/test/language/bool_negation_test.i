// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

pub fn negation_true_test() -> Nil {
  assert !True == False
  Nil
}

pub fn negation_false_test() -> Nil {
  assert !False == True
  Nil
}

// This would crash if the right hand side evaluated
pub fn negation_true_and_panic_test() -> Nil {
  let bool = !True
  bool && panic
  Nil
}

// This would crash if the right hand side evaluated
pub fn negation_false_or_panic_test() -> Nil {
  let bool = !False
  bool || panic
  Nil
}

pub fn negation_correctly_parenthesised_test() -> Nil {
  let bool = !{ True || False }
  assert !bool
  Nil
}
