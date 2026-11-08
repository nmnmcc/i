// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

import ffi

pub fn utf16_pattern_match_test() -> Nil {
  let result = {
    let assert <<"Hello, world":utf16>> = <<"Hello, world":utf16>>
  }
  assert result == <<"Hello, world":utf16>>
  Nil
}

pub fn utf32_pattern_match_test() -> Nil {
  let result = {
    let assert <<"Hello, world":utf32>> = <<"Hello, world":utf32>>
  }
  assert result == <<"Hello, world":utf32>>
  Nil
}

pub fn utf16_bytes_test() -> Nil {
  let left = <<"Hello, 🌍!":utf16>>
  let right = <<
    0, 72, 0, 101, 0, 108, 0, 108, 0, 111, 0, 44, 0, 32, 216, 60, 223, 13, 0, 33,
  >>
  assert left == right
  Nil
}

pub fn utf32_bytes_test() -> Nil {
  let left = <<"Hello, 🌍!":utf32>>
  let right = <<
    0, 0, 0, 72, 0, 0, 0, 101, 0, 0, 0, 108, 0, 0, 0, 108, 0, 0, 0, 111, 0, 0, 0,
    44, 0, 0, 0, 32, 0, 1, 243, 13, 0, 0, 0, 33,
  >>
  assert left == right
  Nil
}

pub fn utf16_pattern_matching_bytes_test() -> Nil {
  let result = {
    let assert <<"Hello, 🌍!":utf16>> = <<
      0, 72, 0, 101, 0, 108, 0, 108, 0, 111, 0, 44, 0, 32, 216, 60, 223, 13, 0,
      33,
    >>
  }
  assert result == <<"Hello, 🌍!":utf16>>
  Nil
}

pub fn utf32_pattern_matching_bytes_test() -> Nil {
  let result = {
    let assert <<"Hello, 🌍!":utf32>> = <<
      0, 0, 0, 72, 0, 0, 0, 101, 0, 0, 0, 108, 0, 0, 0, 108, 0, 0, 0, 111, 0, 0,
      0, 44, 0, 0, 0, 32, 0, 1, 243, 13, 0, 0, 0, 33,
    >>
  }
  assert result == <<"Hello, 🌍!":utf32>>
  Nil
}

pub fn utf16_bytes_little_endian_test() -> Nil {
  let left = <<"Hello, 🌍!":utf16-little>>
  let right = <<
    72, 0, 101, 0, 108, 0, 108, 0, 111, 0, 44, 0, 32, 0, 60, 216, 13, 223, 33, 0,
  >>
  assert left == right
  Nil
}

pub fn utf32_bytes_little_endian_test() -> Nil {
  let left = <<"Hello, 🌍!":utf32-little>>
  let right = <<
    72, 0, 0, 0, 101, 0, 0, 0, 108, 0, 0, 0, 108, 0, 0, 0, 111, 0, 0, 0, 44, 0,
    0, 0, 32, 0, 0, 0, 13, 243, 1, 0, 33, 0, 0, 0,
  >>
  assert left == right
  Nil
}

pub fn utf16_pattern_matching_little_endian_test() -> Nil {
  let result = {
    let assert <<"Hello, 🌍!":utf16-little>> = <<
      72, 0, 101, 0, 108, 0, 108, 0, 111, 0, 44, 0, 32, 0, 60, 216, 13, 223, 33,
      0,
    >>
  }
  assert result == <<"Hello, 🌍!":utf16-little>>
  Nil
}

pub fn utf32_pattern_matching_little_endian_test() -> Nil {
  let result = {
    let assert <<"Hello, 🌍!":utf32-little>> = <<
      72, 0, 0, 0, 101, 0, 0, 0, 108, 0, 0, 0, 108, 0, 0, 0, 111, 0, 0, 0, 44, 0,
      0, 0, 32, 0, 0, 0, 13, 243, 1, 0, 33, 0, 0, 0,
    >>
  }
  assert result == <<"Hello, 🌍!":utf32-little>>
  Nil
}

pub fn utf16_codepoint_test() -> Nil {
  // 🌍
  let codepoint = ffi.utf_codepoint(127_757)
  let result = <<codepoint:utf16_codepoint>>
  assert result == <<216, 60, 223, 13>>
  Nil
}

pub fn utf16_codepoint_little_endian_test() -> Nil {
  // 🌍
  let codepoint = ffi.utf_codepoint(127_757)
  let result = <<codepoint:utf16_codepoint-little>>
  assert result == <<60, 216, 13, 223>>
  Nil
}

pub fn utf32_codepoint_test() -> Nil {
  // 🌍
  let codepoint = ffi.utf_codepoint(127_757)
  let result = <<codepoint:utf32_codepoint>>
  assert result == <<0, 1, 243, 13>>
  Nil
}

pub fn utf32_codepoint_little_endian_test() -> Nil {
  // 🌍
  let codepoint = ffi.utf_codepoint(127_757)
  let result = <<codepoint:utf32_codepoint-little>>
  assert result == <<13, 243, 1, 0>>
  Nil
}
