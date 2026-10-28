// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

import mod_with_numbers_0123456789
import shadowed_module.{ShadowPerson}

pub fn mod_with_numbers_test() -> Nil {
  assert mod_with_numbers_0123456789.hello() == "world"
  Nil
}

pub fn shadowed_module_test() -> Nil {
  let shadowed_module = ShadowPerson(18)
  let shadowed_module = shadowed_module.celebrate_birthday(shadowed_module)
  assert shadowed_module.age == 19
  Nil
}
