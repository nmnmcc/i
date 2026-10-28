// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

pub fn let_assert_ok_discard_test() -> Nil {
  let result = {
    let x = ok_one()
    let assert Ok(_) = x
  }
  assert Ok(1) == result
  Nil
}

pub fn let_assert_ok_bind_test() -> Nil {
  let x = ok_one()
  let assert Ok(x) = x
  assert 1 == x
  Nil
}

fn ok_one() -> Result(Int, a) {
  Ok(1)
}
