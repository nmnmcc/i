import gleam/bit_array
import gleam/int

pub fn arbitrary_precision_parse_roundtrip_test() -> Nil {
  let decimal = "1606938044258990275541962092341162602522202993782792835301376"
  let assert Ok(value) = int.parse(decimal)
  assert value == int.bitwise_shift_left(1, 200)
  assert int.to_string(value) == decimal
  assert int.parse("-" <> decimal) == Ok(0 - value)
  assert int.to_string(0 - value) == "-" <> decimal
  assert int.parse("+" <> decimal) == Ok(value)
  assert int.parse(" 1") == Error(Nil)
  assert int.parse("1 ") == Error(Nil)
  assert int.parse("+") == Error(Nil)
  assert int.parse("١") == Error(Nil)
}

pub fn arbitrary_precision_base_roundtrip_test() -> Nil {
  let value = int.bitwise_shift_left(1, 256) - 1
  let assert Ok(hex) = int.to_base_string(value, 16)
  assert hex
    == "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"
  assert int.base_parse(hex, 16) == Ok(value)
  assert int.base_parse("-abcdef", 16) == Ok(-11_259_375)
  assert int.base_parse("Zz", 36) == Ok(1295)
  assert int.base_parse("2", 2) == Error(Nil)
  assert int.base_parse("0", 1) == Error(Nil)
  assert int.base_parse("0", 37) == Error(Nil)
}

pub fn infinite_twos_complement_test() -> Nil {
  let high = int.bitwise_shift_left(1, 150)
  assert int.bitwise_not(high) == 0 - high - 1
  assert int.bitwise_and(-1, high) == high
  assert int.bitwise_or(-1, high) == -1
  assert int.bitwise_exclusive_or(-1, high) == 0 - high - 1
  assert int.bitwise_and(0 - high - 1, high) == 0
  assert int.bitwise_or(0 - high - 1, high) == -1
  assert int.bitwise_exclusive_or(0 - high, 0 - high) == 0
}

pub fn signed_shift_direction_test() -> Nil {
  assert int.bitwise_shift_right(-3, 1) == -2
  assert int.bitwise_shift_left(-3, -1) == -2
  assert int.bitwise_shift_right(-3, -2) == -12
  assert int.bitwise_shift_right(-3, 1_000_000) == -1
  assert int.bitwise_shift_right(3, 1_000_000) == 0
  assert int.bitwise_shift_left(3, 100)
    == 3_802_951_800_684_688_204_490_109_616_128
}

pub fn integer_division_zero_and_sign_test() -> Nil {
  assert -7 / 3 == -2
  assert 7 / -3 == -2
  assert -7 % 3 == -1
  assert 7 % -3 == 1
  assert 13 / 0 == 0
  assert -13 % 0 == 0
}

pub fn non_aligned_binary_roundtrip_test() -> Nil {
  let bits = <<5:3, 17:5, 7:3>>
  assert bit_array.bit_size(bits) == 11
  assert bit_array.byte_size(bits) == 2
  assert bit_array.pad_to_bytes(bits) == <<177, 224>>
  assert bit_array.base16_encode(bits) == "B1E0"
  assert bit_array.base64_encode(bits, True) == "seA="
  assert bit_array.concat([<<5:3>>, <<17:5>>, <<7:3>>]) == bits
  assert bit_array.slice(bits, 0, 1) == Error(Nil)
  assert bit_array.to_string(bits) == Error(Nil)
}

pub fn utf8_validation_boundaries_test() -> Nil {
  assert bit_array.to_string(<<0xF0, 0x9F, 0x8C, 0x8D>>) == Ok("🌍")
  assert bit_array.to_string(<<0xC0, 0x80>>) == Error(Nil)
  assert bit_array.to_string(<<0xED, 0xA0, 0x80>>) == Error(Nil)
  assert bit_array.to_string(<<0xF4, 0x90, 0x80, 0x80>>) == Error(Nil)
  assert bit_array.to_string(<<0xE2, 0x82>>) == Error(Nil)
  assert bit_array.to_string(<<0x80>>) == Error(Nil)
}

pub fn base64_padding_and_whitespace_test() -> Nil {
  assert bit_array.base64_decode(" \tZ\ng\r== ") == Ok(<<"f":utf8>>)
  assert bit_array.base64_decode("Zg") == Ok(<<"f":utf8>>)
  assert bit_array.base64_decode("Zm8") == Ok(<<"fo":utf8>>)
  assert bit_array.base64_decode("Zm9v") == Ok(<<"foo":utf8>>)
  assert bit_array.base64_decode("Z") == Error(Nil)
  assert bit_array.base64_decode("Z=g=") == Error(Nil)
  assert bit_array.base64_decode("Zm9v====") == Error(Nil)
  assert bit_array.base64_decode("Zg==Zg==") == Error(Nil)
}

pub fn binary_slice_backwards_test() -> Nil {
  let bytes = <<0, 1, 2, 3, 4>>
  assert bit_array.slice(bytes, 4, -3) == Ok(<<1, 2, 3>>)
  assert bit_array.slice(bytes, 5, 0) == Ok(<<>>)
  assert bit_array.slice(bytes, 0, -1) == Error(Nil)
  assert bit_array.slice(bytes, 6, 0) == Error(Nil)
  assert bit_array.slice(bytes, 0, 6) == Error(Nil)
}
