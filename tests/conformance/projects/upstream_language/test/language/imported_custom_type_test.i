// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

import importable.{NoFields}

pub fn no_fields_qualified_and_unqualified_test() -> Nil {
  assert importable.NoFields == NoFields
  Nil
}

pub fn no_fields_assert_assignment_test() -> Nil {
  let result = {
    let assert importable.NoFields = importable.NoFields
  }
  assert result == importable.NoFields
  Nil
}

pub fn no_fields_unqualified_assert_assignment_test() -> Nil {
  let result = {
    let assert NoFields = importable.NoFields
  }
  assert result == importable.NoFields
  Nil
}

pub fn no_fields_let_assignment_test() -> Nil {
  let result = {
    let importable.NoFields = importable.NoFields
  }
  assert result == importable.NoFields
  Nil
}

pub fn no_fields_unqualified_let_assignment_test() -> Nil {
  let result = {
    let NoFields = importable.NoFields
  }
  assert result == importable.NoFields
  Nil
}
