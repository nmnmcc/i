// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

import i

pub fn gleam_ok_test() -> Nil {
  let left = Ok(1)
  let right = i.Ok(1)
  assert left == right
  Nil
}

pub fn gleam_error_test() -> Nil {
  let left = Error(1)
  let right = i.Error(1)
  assert left == right
  Nil
}

pub fn gleam_nil_test() -> Nil {
  let left = Nil
  let right = i.Nil
  assert left == right
  Nil
}
