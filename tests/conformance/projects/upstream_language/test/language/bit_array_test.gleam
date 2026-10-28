// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 The Gleam contributors

import ffi
import gleam/bit_array

pub fn utf8_emoji_equal_test() -> Nil {
  let left = <<"Gleam":utf8, "👍":utf8>>
  let right = <<"Gleam":utf8, "👍":utf8>>
  assert left == right
  Nil
}

pub fn utf8_emoji_not_equal_test() -> Nil {
  let left = <<"Gleam":utf8, "👍":utf8>>
  let right = <<"👍":utf8>>
  assert left != right
  Nil
}

pub fn utf8_ascii_test() -> Nil {
  let left = <<"abc":utf8>>
  let right = <<97, 98, 99>>
  assert left == right
  Nil
}

pub fn utf8_unicode_escape_test() -> Nil {
  let left = <<"😀":utf8>>
  let right = <<"\u{1F600}":utf8>>
  assert left == right
  Nil
}

pub fn nested_bit_array_test() -> Nil {
  let left = <<<<1>>:bits, 2>>
  let right = <<1, 2>>
  assert left == right
  Nil
}

pub fn int_segment_test() -> Nil {
  let left = <<1>>
  let right = <<1:int>>
  assert left == right
  Nil
}

pub fn int_16_test() -> Nil {
  let left = <<80_000:16>>
  let right = <<56, 128>>
  assert left == right
  Nil
}

pub fn negative_int_16_test() -> Nil {
  let left = <<-80_000:16>>
  let right = <<199, 128>>
  assert left == right
  Nil
}

pub fn negative_int_64_test() -> Nil {
  let left = <<-1:64>>
  let right = <<255, 255, 255, 255, 255, 255, 255, 255>>
  assert left == right
  Nil
}

pub fn negative_int_56_test() -> Nil {
  let left = <<-489_391_639_457_909_760:56>>
  let right = <<53, 84, 229, 150, 16, 180, 0>>
  assert left == right
  Nil
}

pub fn float_16_zero_test() -> Nil {
  let left = <<0, 0>>
  let right = <<0.0:float-16>>
  assert left == right
  Nil
}

pub fn float_16_little_negative_test() -> Nil {
  let left = <<0x00, 0xbc>>
  let right = <<-1.0:float-16-little>>
  assert left == right
  Nil
}

pub fn float_16_little_test() -> Nil {
  let left = <<0xf0, 0x3c>>
  let right = <<1.234375:float-16-little>>
  assert left == right
  Nil
}

pub fn float_16_negative_large_test() -> Nil {
  let left = <<0xfb, 0xff>>
  let right = <<-65_504.0:float-16>>
  assert left == right
  Nil
}

pub fn float_64_test() -> Nil {
  let left = <<63, 240, 0, 0, 0, 0, 0, 0>>
  let right = <<1.0:float>>
  assert left == right
  Nil
}

pub fn float_32_test() -> Nil {
  let left = <<63, 128, 0, 0>>
  let right = <<1.0:float-32>>
  assert left == right
  Nil
}

pub fn float_64_little_test() -> Nil {
  let left = <<0, 0, 0, 0, 0, 0, 240, 63>>
  let right = <<1.0:float-64-little>>
  assert left == right
  Nil
}

pub fn float_64_big_test() -> Nil {
  let left = <<63, 240, 0, 0, 0, 0, 0, 0>>
  let right = <<1.0:float-64-big>>
  assert left == right
  Nil
}

pub fn pattern_match_utf8_test() -> Nil {
  let result = case <<0x20, "😀👍":utf8, 0x20>> {
    <<" ":utf8, "😀👍":utf8, 0x20>> -> True
    _ -> False
  }
  assert result == True
  Nil
}

pub fn pattern_match_bytes_sliced_test() -> Nil {
  let assert <<_, b:bytes-3, _>> = <<1, 2, 3, 4, 5>>
  let assert <<_, rest:bytes>> = b
  assert rest == <<3, 4>>
  Nil
}

pub fn matching_zero_length_segment_test() -> Nil {
  let size = 0
  let data = <<>>
  let result = case data {
    <<_:bytes-size(size), _:bytes>> -> "ok"
    _ -> "this cause should not be reached"
  }
  assert result == "ok"
  Nil
}

pub fn float_16_erlang_test() -> Nil {
  let left = <<60, 0>>
  let right = <<1.0:float-16>>
  assert left == right
  Nil
}

// https://github.com/gleam-lang/gleam/issues/3375
pub fn assignment_int_pattern_test() -> Nil {
  let assert <<10 as a, _>> = <<10, 20>>
  assert a == 10
  Nil
}

pub fn assignment_float_pattern_test() -> Nil {
  let assert <<3.14 as pi:float>> = <<3.14>>
  assert pi == 3.14
  Nil
}

pub fn assignment_string_pattern_test() -> Nil {
  let assert <<"Hello" as h:utf8, ", world!">> = <<"Hello, world!">>
  assert h == "Hello"
  Nil
}

pub fn pattern_match_utf16_codepoint_little_test() -> Nil {
  let assert <<codepoint:utf16_codepoint-little>> = <<"🌍":utf16-little>>
  assert codepoint == ffi.utf_codepoint(127_757)
  Nil
}

pub fn pattern_match_utf32_codepoint_little_test() -> Nil {
  let assert <<codepoint:utf32_codepoint-little>> = <<"🌍":utf32-little>>
  assert codepoint == ffi.utf_codepoint(127_757)
  Nil
}

pub fn unicode_overflow_test() -> Nil {
  // In Erlang, literally creating binaries can cause entries to overflow.
  // For example `<<"🌵">> == <<"5">>` evaluates to true.
  // This checks that we are not doing that.
  // See: https://github.com/gleam-lang/gleam/issues/457
  let string = "5"
  assert "🌵" != string
  Nil
}

pub fn sliced_bit_array_data_view_offset_test() -> Nil {
  let data = <<0xAA, 0xBB, 0xCC, 0xDD>>
  let assert Ok(sliced) = bit_array.slice(data, 1, 3)
  assert sliced == <<0xBB, 0xCC, 0xDD>>
  let value = ffi.read_uint16_from_bit_array(sliced)
  assert value == 48_076
  Nil
}

pub fn sliced_bit_array_data_view_byte_length_test() -> Nil {
  let data = <<0xAA, 0xBB, 0xCC, 0xDD>>
  let assert Ok(sliced) = bit_array.slice(data, 1, 2)
  assert sliced == <<0xBB, 0xCC>>
  Nil
}

// https://github.com/gleam-lang/gleam/issues/6036
pub fn negative_zero_16_bits_test() -> Nil {
  let data = <<-0.0:16>>
  assert data == <<0x8000:16>>
  Nil
}

pub fn float_16_rounds_ties_to_even_test() -> Nil {
  assert <<2047.5:float-16>> == <<0x68, 0x00>>
  assert <<2049.0:float-16>> == <<0x68, 0x00>>
  assert <<1.00048828125:float-16>> == <<0x3c, 0x00>>
  assert <<1.0009765625:float-16>> == <<0x3c, 0x01>>
  Nil
}

pub fn float_16_rounding_carries_into_exponent_test() -> Nil {
  assert <<3945.0:float-16>> == <<0x6b, 0xb4>>
  assert <<65_520.0:float-16>> == <<0x7c, 0x00>>
  Nil
}

pub fn float_16_subnormal_rounds_ties_to_even_test() -> Nil {
  assert <<2.9802322387695312e-8:float-16>> == <<0x00, 0x00>>
  assert <<8.940696716308594e-8:float-16>> == <<0x00, 0x02>>
  Nil
}

pub fn zero_size_int_segment_test() -> Nil {
  assert <<0:size(0), 1:size(8)>> == <<1>>
  assert bit_array.bit_size(<<0:size(0)>>) == 0
  Nil
}
