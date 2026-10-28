// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

pub fn size_8_literal_test() -> Nil {
  let left = <<257:size(8)>>
  let right = <<1>>
  assert left == right
  Nil
}

pub fn size_8_variable_test() -> Nil {
  let i = 257
  let left = <<i:size(8)>>
  let right = <<1>>
  assert left == right
  Nil
}

pub fn size_16_literal_test() -> Nil {
  let left = <<257:size(16)>>
  let right = <<1, 1>>
  assert left == right
  Nil
}

pub fn size_16_variable_test() -> Nil {
  let i = 257
  let left = <<i:size(16)>>
  let right = <<1, 1>>
  assert left == right
  Nil
}

pub fn size_24_literal_test() -> Nil {
  let left = <<257:size(24)>>
  let right = <<0, 1, 1>>
  assert left == right
  Nil
}

pub fn size_24_variable_test() -> Nil {
  let i = 257
  let left = <<i:size(24)>>
  let right = <<0, 1, 1>>
  assert left == right
  Nil
}

pub fn size_40_literal_test() -> Nil {
  let left = <<4_294_967_297:size(40)>>
  let right = <<1, 0, 0, 0, 1>>
  assert left == right
  Nil
}

pub fn size_40_variable_test() -> Nil {
  let i = 4_294_967_297
  let left = <<i:size(40)>>
  let right = <<1, 0, 0, 0, 1>>
  assert left == right
  Nil
}

pub fn size_24_little_literal_test() -> Nil {
  let left = <<100_000:24-little>>
  let right = <<160, 134, 1>>
  assert left == right
  Nil
}

pub fn size_24_little_variable_test() -> Nil {
  let i = 100_000
  let left = <<i:24-little>>
  let right = <<160, 134, 1>>
  assert left == right
  Nil
}

pub fn size_32_big_negative_literal_test() -> Nil {
  let left = <<-1:32-big>>
  let right = <<255, 255, 255, 255>>
  assert left == right
  Nil
}

pub fn size_32_big_negative_variable_test() -> Nil {
  let i = -1
  let left = <<i:32-big>>
  let right = <<255, 255, 255, 255>>
  assert left == right
  Nil
}

pub fn size_32_little_large_literal_test() -> Nil {
  let left = <<100_000_000_000:32-little>>
  let right = <<0, 232, 118, 72>>
  assert left == right
  Nil
}

pub fn size_32_little_large_variable_test() -> Nil {
  let i = 100_000_000_000
  let left = <<i:32-little>>
  let right = <<0, 232, 118, 72>>
  assert left == right
  Nil
}

pub fn negative_size_literal_test() -> Nil {
  let left = <<>>
  let right = <<256:size(-1)>>
  assert left == right
  Nil
}

pub fn negative_size_variable_test() -> Nil {
  let i = 256
  let left = <<i:size(-1)>>
  let right = <<>>
  assert left == right
  Nil
}

// JS Number.MAX_SAFE_INTEGER
pub fn size_64_max_safe_int_literal_test() -> Nil {
  let left = <<9_007_199_254_740_991:size(64)>>
  let right = <<0, 31, 255, 255, 255, 255, 255, 255>>
  assert left == right
  Nil
}

pub fn size_64_max_safe_int_variable_test() -> Nil {
  let i = 9_007_199_254_740_991
  let left = <<i:size(64)>>
  let right = <<0, 31, 255, 255, 255, 255, 255, 255>>
  assert left == right
  Nil
}

pub fn size_unit_test() -> Nil {
  let i = 9_007_199_254_740_991
  let left = <<i:size(4)-unit(16)>>
  let right = <<0, 31, 255, 255, 255, 255, 255, 255>>
  assert left == right
  Nil
}

pub fn dynamic_size_unit_test() -> Nil {
  let size = 5
  let left = <<405:size(size)-unit(2)>>
  let right = <<101, 1:2>>
  assert left == right
  Nil
}

pub fn pattern_bits_size_expression_test() -> Nil {
  let assert <<len, payload:bits-size(len * 8 - 4)>> = <<4, 1, 2, 3, 4:4>>
  assert payload == <<1, 2, 3, 4:4>>
  Nil
}

pub fn pattern_bytes_size_expression_test() -> Nil {
  let assert <<len, payload:bytes-size(len / 8 + 2)>> = <<32, 1, 2, 3, 4, 5, 6>>
  assert payload == <<1, 2, 3, 4, 5, 6>>
  Nil
}

pub fn pattern_bits_size_with_variable_test() -> Nil {
  let additional = 5
  let assert <<len, payload:bits-size(len + additional * 8)>> = <<
    8, 1, 2, 3, 4, 5, 6,
  >>
  assert payload == <<1, 2, 3, 4, 5, 6>>
  Nil
}

pub fn pattern_bits_size_block_expression_test() -> Nil {
  let assert <<len, payload:bits-size({ len + 1 } * 8)>> = <<3, 1, 2, 3, 4>>
  assert payload == <<1, 2, 3, 4>>
  Nil
}

pub fn pattern_negative_size_test() -> Nil {
  let result = case <<1, 2, 3, 4>> {
    <<a, b:size(a - 100_000), _c:size(b)>> -> 1
    _ -> 2
  }
  assert result == 2
  Nil
}
