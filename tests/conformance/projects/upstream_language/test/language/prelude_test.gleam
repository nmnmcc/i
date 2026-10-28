// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

import gleam

pub fn gleam_ok_test() -> Nil {
  let left = Ok(1)
  let right = gleam.Ok(1)
  assert left == right
  Nil
}

pub fn gleam_error_test() -> Nil {
  let left = Error(1)
  let right = gleam.Error(1)
  assert left == right
  Nil
}

pub fn gleam_nil_test() -> Nil {
  let left = Nil
  let right = gleam.Nil
  assert left == right
  Nil
}
