// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2021 The Gleam contributors
// Wleam adaptation: portable implementations for the two used test helpers.

import gleam/string

pub fn utf_codepoint(value: Int) -> UtfCodepoint {
  let assert Ok(codepoint) = string.utf_codepoint(value)
  codepoint
}

pub fn read_uint16_from_bit_array(value: BitArray) -> Int {
  let assert <<integer:16-big-unsigned, _:bits>> = value
  integer
}
