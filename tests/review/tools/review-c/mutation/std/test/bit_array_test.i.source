import (
	i/core
	bit_array = std/bit_array
	result = std/result
	list = std/list
	string = std/string
	order = (Less, Equal, Greater) = std/order
)

@test
bit_size_empty_test = () {
	assert(bit_array.bit_size(<<>>) == 0)
	Nil
}

@test
bit_size_one_byte_test = () {
	assert(bit_array.bit_size(<<0>>) == 8)
	Nil
}

@test
bit_size_32_bits_test = () {
	assert(bit_array.bit_size(<<-1:u(32)>>) == 32)
	Nil
}

@test
bit_size_one_bit_test = () {
	assert(bit_array.bit_size(<<0:u1>>) == 1)
	Nil
}

@test
bit_size_three_bits_test = () {
	assert(bit_array.bit_size(<<7:u3>>) == 3)
	Nil
}

@test
bit_size_190_bits_test = () {
	assert(bit_array.bit_size(<<-1:u190>>) == 190)
	Nil
}

@test
byte_size_empty_test = () {
	assert(bit_array.byte_size(<<>>) == 0)
	Nil
}

@test
byte_size_five_bytes_test = () {
	assert(bit_array.byte_size(<<0, 1, 2, 3, 4>>) == 5)
	Nil
}

@test
byte_size_unaligned_test = () {
	assert(bit_array.byte_size(<<1, 2, 3:u6>>) == 3)
	Nil
}

@test
pad_to_bytes_empty_test = () {
	assert(bit_array.pad_to_bytes(<<>>) == <<>>)
	Nil
}

@test
pad_to_bytes_one_byte_test = () {
	assert(bit_array.pad_to_bytes(<<0xAB>>) == <<0xAB>>)
	Nil
}

@test
pad_to_bytes_two_bytes_test = () {
	assert(bit_array.pad_to_bytes(<<0xAB, 0x12>>) == <<0xAB, 0x12>>)
	Nil
}

@test
pad_to_bytes_one_bit_test = () {
	assert(bit_array.pad_to_bytes(<<1:u1>>) == <<0x80>>)
	Nil
}

@test
pad_to_bytes_seven_bits_test = () {
	assert(bit_array.pad_to_bytes(<<-1:u7>>) == <<0xFE>>)
	Nil
}

@test
pad_to_bytes_unaligned_test = () {
	assert(bit_array.pad_to_bytes(<<0xAB, 0x12, 3:u3>>) == <<0xAB, 0x12, 0x60>>)
	Nil
}

@test
pad_to_bytes_slice_test = () {
	<<a:bits(12), _:u4>> = <<0xAB, 0xFF>>
	assert(bit_array.pad_to_bytes(a) == <<0xAB, 0xF0>>)
	Nil
}

// https://github.com/gleam-lang/stdlib/issues/934
@test
pad_to_bytes_crash_test = () {
	<<_:u3, rest:bits>> = <<0xAB, 0xB0>>
	assert(bit_array.pad_to_bytes(rest) == <<93, 128>>)
	Nil
}

@test
not_equal_test = () {
	assert(bit_array.from_string("test") != bit_array.from_string("asdf"))
	Nil
}

@test
append_strings_test = () {
	assert(
		bit_array.append(
			bit_array.from_string("Test")
			bit_array.from_string(" Me")
		) ==
			bit_array.from_string("Test Me")
	)
	Nil
}

@test
append_empty_test = () {
	assert(bit_array.append(<<1, 2>>, <<>>) == <<1, 2>>)
	Nil
}

@test
append_bytes_test = () {
	assert(bit_array.append(<<1, 2>>, <<3, 4>>) == <<1, 2, 3, 4>>)
	Nil
}

@test
append_unaligned_test = () {
	assert(bit_array.append(<<1, 2:u4>>, <<3>>) == <<1, 2:u4, 3>>)
	Nil
}

@test
concat_single_test = () {
	assert(bit_array.concat([<<1, 2>>]) == <<1, 2>>)
	Nil
}

@test
concat_multiple_test = () {
	assert(bit_array.concat([<<1, 2>>, <<3>>, <<4>>]) == <<1, 2, 3, 4>>)
	Nil
}

@test
concat_unaligned_test = () {
	assert(
		bit_array.concat([<<-1:u(32)>>, <<0:u1>>, <<0:u(0)>>]) ==
			<<255, 255, 255, 255, 0:u1>>
	)
	Nil
}

@test
concat_complex_test = () {
	assert(
		bit_array.concat([<<-20:u6, 2>>, <<3:u4>>, <<7:u3>>, <<-1:u(64)>>]) ==
			<<176, 8, 255, 255, 255, 255, 255, 255, 255, 255, 31:u5>>
	)
	Nil
}

@test
concat_unaligned_2_test = () {
	assert(bit_array.concat([<<1, 2:u4>>, <<3>>]) == <<1, 2:u4, 3>>)
	Nil
}

@test
concat_unaligned_3_test = () {
	assert(
		bit_array.concat([<<-1:u(32)>>, <<0:u1>>, <<0:u(0)>>]) ==
			<<255, 255, 255, 255, 0:u1>>
	)
	Nil
}

@test
concat_empty_list_test = () {
	assert(bit_array.concat([]) == <<>>)
	assert(bit_array.concat([<<>>, <<>>]) == <<>>)
	Nil
}

@test
concat_bits_test = () {
	assert(bit_array.concat([<<1:u1>>, <<2:u2>>, <<255>>]) == <<223, 7:u3>>)
	assert(
		bit_array.concat([<<5:u3>>, <<5:u3>>, <<5:u3>>]) == <<5:u3, 5:u3, 5:u3>>
	)
	assert(
		bit_array.bit_size(bit_array.concat([<<0:u5>>, <<0:u5>>, <<0:u5>>])) ==
			15
	)
	Nil
}

@test
concat_slices_test = () {
	case <<170, 85, 255>> {
		<<_:u3, middle:bits(13), rest:bits>> -> {
			assert(
				bit_array.concat([middle, rest, middle]) ==
					<<middle:bits, rest:bits, middle:bits>>
			)
			Nil
		}
		_ -> panic("no match")
	}
}

@test
concat_long_list_test = () {
	parts = list.repeat(<<1:u1>>, 10_000)
	assert(bit_array.bit_size(bit_array.concat(parts)) == 10_000)
	assert(bit_array.concat(list.repeat(<<255>>, 3)) == <<255, 255, 255>>)
	Nil
}

@test
concat_complex_2_test = () {
	assert(
		bit_array.concat([<<-20:u6, 2>>, <<3:u4>>, <<7:u3>>, <<-1:u(64)>>]) ==
			<<176, 8, 255, 255, 255, 255, 255, 255, 255, 255, 31:u5>>
	)
	Nil
}

@test
slice_full_test = () {
	assert(bit_array.slice(<<"hello":utf8>>, 0, 5) == Ok(<<"hello":utf8>>))
	Nil
}

@test
slice_empty_test = () {
	assert(bit_array.slice(<<"hello":utf8>>, 0, 0) == Ok(<<"":utf8>>))
	Nil
}

@test
slice_middle_test = () {
	assert(bit_array.slice(<<"hello":utf8>>, 2, 2) == Ok(<<"ll":utf8>>))
	Nil
}

@test
slice_negative_length_test = () {
	assert(bit_array.slice(<<"hello":utf8>>, 5, -2) == Ok(<<"lo":utf8>>))
	Nil
}

@test
slice_empty_string_test = () {
	assert(bit_array.slice(<<"":utf8>>, 0, 0) == Ok(<<"":utf8>>))
	Nil
}

@test
slice_out_of_bounds_test = () {
	assert(bit_array.slice(<<"hello":utf8>>, 6, 0) == Error(Nil))
	Nil
}

@test
slice_invalid_negative_test = () {
	assert(bit_array.slice(<<"hello":utf8>>, 1, -2) == Error(Nil))
	Nil
}

@test
slice_negative_start_test = () {
	assert(bit_array.slice(bit_array.from_string("hello"), -1, 1) == Error(Nil))
	Nil
}

@test
slice_length_too_long_test = () {
	assert(bit_array.slice(bit_array.from_string("hello"), 1, 6) == Error(Nil))
	Nil
}

@test
slice_chained_test = () {
	assert(
		bit_array.from_string("ab")
			|> bit_array.slice(1, 1)
			|> result.then((b) { bit_array.slice(b, 0, 1) }) == Ok(<<"b":utf8>>)
	)
	Nil
}

@test
slice_unaligned_error_test = () {
	assert(bit_array.slice(<<0, 1, 2:u7>>, 0, 3) == Error(Nil))
	Nil
}

@test
slice_large_error_test = () {
	assert(
		bit_array.slice(
			<<0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15>>
			8
			12
		) ==
			Error(Nil)
	)
	Nil
}

@test
to_string_empty_test = () {
	assert(bit_array.to_string(<<>>) == Ok(""))
	Nil
}

@test
to_string_empty_utf8_test = () {
	assert(bit_array.to_string(<<"":utf8>>) == Ok(""))
	Nil
}

@test
to_string_hello_test = () {
	assert(bit_array.to_string(<<"Hello":utf8>>) == Ok("Hello"))
	Nil
}

@test
to_string_unicode_test = () {
	assert(bit_array.to_string(<<"ø":utf8>>) == Ok("ø"))
	Nil
}

@test
to_string_invalid_test = () {
	assert(bit_array.to_string(<<255>>) == Error(Nil))
	Nil
}

@test
to_string_unaligned_error_test = () {
	assert(bit_array.to_string(<<"ø":utf8, 2:u4>>) == Error(Nil))
	Nil
}

@test
to_string_offset_test = () {
	<<_:u3, x:bits>> = <<0:u3, "ø":utf8>>
	assert(bit_array.to_string(x) == Ok("ø"))
	Nil
}

@test
is_utf8_empty_test = () {
	assert(bit_array.is_utf8(<<>>))
	Nil
}

@test
is_utf8_empty_string_test = () {
	assert(bit_array.is_utf8(<<"":utf8>>))
	Nil
}

@test
is_utf8_hello_test = () {
	assert(bit_array.is_utf8(<<"Hello":utf8>>))
	Nil
}

@test
is_utf8_unicode_test = () {
	assert(bit_array.is_utf8(<<"ø":utf8>>))
	Nil
}

@test
is_utf8_invalid_test = () {
	assert(core.not(bit_array.is_utf8(<<255>>)))
	Nil
}

@test
base64_encode_padded_test = () {
	assert(bit_array.base64_encode(<<255, 127, 254, 252>>, True) == "/3/+/A==")
	Nil
}

@test
base64_encode_padded_5_bytes_test = () {
	assert(
		bit_array.base64_encode(<<255, 127, 254, 252, 100>>, True) == "/3/+/GQ="
	)
	Nil
}

@test
base64_encode_unpadded_test = () {
	assert(bit_array.base64_encode(<<255, 127, 254, 252>>, False) == "/3/+/A")
	Nil
}

@test
base64_encode_zeros_test = () {
	assert(bit_array.base64_encode(<<0, 0, 0>>, True) == "AAAA")
	Nil
}

@test
base64_encode_empty_test = () {
	assert(bit_array.base64_encode(<<>>, True) == "")
	Nil
}

@test
base64_encode_large_test = () {
	assert(
		string.repeat("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", 1024 * 32)
			|> bit_array.from_string
			|> bit_array.base64_encode(True) == string.repeat(
				"QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFB"
				1024 * 32
			)
	)
	Nil
}

@test
base64_encode_unaligned_test = () {
	assert(bit_array.base64_encode(<<-1:u7>>, True) == "/g==")
	Nil
}

@test
base64_encode_unaligned_2_test = () {
	assert(bit_array.base64_encode(<<0xFA, 5:u3>>, True) == "+qA=")
	Nil
}

@test
base64_encode_unaligned_3_test = () {
	assert(
		bit_array.base64_encode(<<0xFA, 0xBC, 0x6D, 1:u1>>, True) == "+rxtgA=="
	)
	Nil
}

@test
base64_decode_padded_test = () {
	assert(bit_array.base64_decode("/3/+/A==") == Ok(<<255, 127, 254, 252>>))
	Nil
}

@test
base64_decode_unpadded_test = () {
	assert(bit_array.base64_decode("/3/+/A") == Ok(<<255, 127, 254, 252>>))
	Nil
}

@test
base64_decode_zeros_test = () {
	assert(bit_array.base64_decode("AAAA") == Ok(<<0, 0, 0>>))
	Nil
}

@test
base64_decode_empty_test = () {
	assert(bit_array.base64_decode("") == Ok(<<>>))
	Nil
}

@test
base64_decode_invalid_test = () {
	assert(bit_array.base64_decode(")!") == Error(Nil))
	Nil
}

@test
base64_decode_invalid_padding_test = () {
	assert(bit_array.base64_decode("=AAA") == Error(Nil))
	Nil
}

@test
base64_url_encode_padded_test = () {
	assert(
		bit_array.base64_url_encode(<<255, 127, 254, 252>>, True) == "_3_-_A=="
	)
	Nil
}

@test
base64_url_encode_unpadded_test = () {
	assert(
		bit_array.base64_url_encode(<<255, 127, 254, 252>>, False) == "_3_-_A"
	)
	Nil
}

@test
base64_url_encode_zeros_test = () {
	assert(bit_array.base64_url_encode(<<0, 0, 0>>, True) == "AAAA")
	Nil
}

@test
base64_url_encode_empty_test = () {
	assert(bit_array.base64_url_encode(<<>>, True) == "")
	Nil
}

@test
base64_url_decode_padded_test = () {
	assert(
		bit_array.base64_url_decode("_3_-_A==") == Ok(<<255, 127, 254, 252>>)
	)
	Nil
}

@test
base64_url_decode_unpadded_test = () {
	assert(bit_array.base64_url_decode("_3_-_A") == Ok(<<255, 127, 254, 252>>))
	Nil
}

@test
base64_url_decode_zeros_test = () {
	assert(bit_array.base64_url_decode("AAAA") == Ok(<<0, 0, 0>>))
	Nil
}

@test
base64_url_decode_empty_test = () {
	assert(bit_array.base64_url_decode("") == Ok(<<>>))
	Nil
}

@test
base64_url_decode_invalid_test = () {
	assert(bit_array.base64_url_decode(")!") == Error(Nil))
	Nil
}

@test
base64_decode_crash_regression_1_test = () {
	assert(
		bit_array.base64_decode(
			"aGktdGhlcmU.uWUWvrAleKQ2jsWcU97H-RPJ5qRRcE_s"
		) ==
			Error(Nil)
	)
	Nil
}

@test
base16_encode_empty_test = () {
	assert(bit_array.base16_encode(<<"":utf8>>) == "")
	Nil
}

@test
base16_encode_f_test = () {
	assert(bit_array.base16_encode(<<"f":utf8>>) == "66")
	Nil
}

@test
base16_encode_fo_test = () {
	assert(bit_array.base16_encode(<<"fo":utf8>>) == "666F")
	Nil
}

@test
base16_encode_foo_test = () {
	assert(bit_array.base16_encode(<<"foo":utf8>>) == "666F6F")
	Nil
}

@test
base16_encode_foob_test = () {
	assert(bit_array.base16_encode(<<"foob":utf8>>) == "666F6F62")
	Nil
}

@test
base16_encode_fooba_test = () {
	assert(bit_array.base16_encode(<<"fooba":utf8>>) == "666F6F6261")
	Nil
}

@test
base16_encode_foobar_test = () {
	assert(bit_array.base16_encode(<<"foobar":utf8>>) == "666F6F626172")
	Nil
}

@test
base16_encode_binary_test = () {
	assert(
		bit_array.base16_encode(<<161, 178, 195, 212, 229, 246, 120, 145>>) ==
			"A1B2C3D4E5F67891"
	)
	Nil
}

@test
base16_encode_unaligned_7_test = () {
	assert(bit_array.base16_encode(<<-1:u7>>) == "FE")
	Nil
}

@test
base16_encode_unaligned_3_test = () {
	assert(bit_array.base16_encode(<<0xFA, 5:u3>>) == "FAA0")
	Nil
}

@test
base16_encode_unaligned_4_test = () {
	assert(bit_array.base16_encode(<<0xFA, 5:u4>>) == "FA50")
	Nil
}

@test
base16_encode_unaligned_1_test = () {
	assert(bit_array.base16_encode(<<0xFA, 0xBC, 0x6D, 1:u1>>) == "FABC6D80")
	Nil
}

@test
base16_decode_empty_test = () {
	assert(bit_array.base16_decode("") == Ok(<<>>))
	Nil
}

@test
base16_decode_f_test = () {
	assert(bit_array.base16_decode("66") == Ok(<<"f":utf8>>))
	Nil
}

@test
base16_decode_fo_test = () {
	assert(bit_array.base16_decode("666F") == Ok(<<"fo":utf8>>))
	Nil
}

@test
base16_decode_foo_test = () {
	assert(bit_array.base16_decode("666F6F") == Ok(<<"foo":utf8>>))
	Nil
}

@test
base16_decode_foob_test = () {
	assert(bit_array.base16_decode("666F6F62") == Ok(<<"foob":utf8>>))
	Nil
}

@test
base16_decode_fooba_test = () {
	assert(bit_array.base16_decode("666F6F6261") == Ok(<<"fooba":utf8>>))
	Nil
}

@test
base16_decode_foobar_test = () {
	assert(bit_array.base16_decode("666F6F626172") == Ok(<<"foobar":utf8>>))
	Nil
}

@test
base16_decode_binary_test = () {
	assert(
		bit_array.base16_decode("A1B2C3D4E5F67891") ==
			Ok(<<161, 178, 195, 212, 229, 246, 120, 145>>)
	)
	Nil
}

@test
base16_decode_invalid_test = () {
	// Not a hex string
	assert(bit_array.base16_decode("?") == Error(Nil))
	Nil
}

@test
base16_decode_lowercase_test = () {
	// Lowercase hex
	assert(
		bit_array.base16_decode("a1b2c3d4e5f67891") ==
			Ok(<<161, 178, 195, 212, 229, 246, 120, 145>>)
	)
	Nil
}

@test
inspect_empty_test = () {
	assert(bit_array.inspect(<<>>) == "<<>>")
	Nil
}

@test
inspect_single_byte_test = () {
	assert(bit_array.inspect(<<80>>) == "<<80>>")
	Nil
}

@test
inspect_multiple_bytes_test = () {
	assert(bit_array.inspect(<<0, 20, 0x20, 255>>) == "<<0, 20, 32, 255>>")
	Nil
}

@test
inspect_unaligned_test = () {
	assert(bit_array.inspect(<<4:u5>>) == "<<4:u5>>")
	Nil
}

@test
inspect_unaligned_2_test = () {
	assert(bit_array.inspect(<<100, 5:u3>>) == "<<100, 5:u3>>")
	Nil
}

@test
inspect_unaligned_3_test = () {
	assert(bit_array.inspect(<<5:u3, 11:u4, 1:u2>>) == "<<182, 1:u1>>")
	Nil
}

@test
compare_equal_test = () {
	assert(bit_array.compare(<<4:u5>>, <<4:u5>>) == Equal)
	Nil
}

@test
compare_gt_test = () {
	assert(bit_array.compare(<<4:u5, 3:u3>>, <<4:u5, 2:u3>>) == Greater)
	Nil
}

@test
compare_lt_test = () {
	assert(bit_array.compare(<<4:u5, 3:u3>>, <<4:u5, 4:u3>>) == Less)
	Nil
}

@test
compare_equal_with_zero_test = () {
	assert(
		bit_array.compare(<<4:u5, 3:u3, 0:u(0)>>, <<4:u5, 3:u3, 0:u(0)>>) ==
			Equal
	)
	Nil
}

@test
compare_different_sizes_gt_test = () {
	assert(
		bit_array.compare(<<0:u2, 3:u4, 0:u(0)>>, <<0:u2, 3:u3, 0:u(0)>>) ==
			Greater
	)
	Nil
}

@test
compare_different_values_lt_test = () {
	// first is: <<33, 1:u1>>
	// second is: <<35>>
	assert(
		bit_array.compare(<<4:u5, 3:u4, 0:u(0)>>, <<4:u5, 3:u3, 0:u(0)>>) ==
			Less
	)
	Nil
}

@test
compare_lt_5_bits_test = () {
	assert(bit_array.compare(<<3:u5>>, <<4:u5>>) == Less)
	Nil
}

@test
compare_lt_7_bits_test = () {
	assert(bit_array.compare(<<3:u7>>, <<4:u7>>) == Less)
	Nil
}

@test
compare_gt_5_bits_test = () {
	assert(bit_array.compare(<<5:u5>>, <<4:u5>>) == Greater)
	Nil
}

@test
compare_longer_gt_test = () {
	assert(bit_array.compare(<<4:u8>>, <<4:u5>>) == Greater)
	Nil
}

@test
compare_shorter_lt_test = () {
	assert(bit_array.compare(<<4:u5>>, <<4:u8>>) == Less)
	Nil
}

@test
compare_zero_shorter_lt_test = () {
	assert(bit_array.compare(<<0:u5>>, <<0:u8>>) == Less)
	Nil
}

@test
compare_zero_equal_test = () {
	assert(bit_array.compare(<<0:u5>>, <<0:u5>>) == Equal)
	Nil
}

@test
compare_zero_longer_gt_test = () {
	assert(bit_array.compare(<<0:u2>>, <<0:u1>>) == Greater)
	Nil
}

@test
starts_with_empty_empty_test = () {
	assert(bit_array.starts_with(<<>>, <<>>))
	Nil
}

@test
starts_with_nonempty_empty_test = () {
	assert(bit_array.starts_with(<<0>>, <<>>))
	Nil
}

@test
starts_with_empty_nonempty_test = () {
	assert(core.not(bit_array.starts_with(<<>>, <<0>>)))
	Nil
}

@test
starts_with_single_byte_test = () {
	assert(bit_array.starts_with(<<0, 1, 2>>, <<0>>))
	Nil
}

@test
starts_with_two_bytes_test = () {
	assert(bit_array.starts_with(<<0, 1, 2>>, <<0, 1>>))
	Nil
}

@test
starts_with_full_match_test = () {
	assert(bit_array.starts_with(<<0, 1, 2>>, <<0, 1, 2>>))
	Nil
}

@test
starts_with_longer_prefix_test = () {
	assert(core.not(bit_array.starts_with(<<0, 1, 2>>, <<0, 1, 2, 3>>)))
	Nil
}

@test
starts_with_wrong_prefix_test = () {
	assert(core.not(bit_array.starts_with(<<0, 1, 2>>, <<1>>)))
	Nil
}

@test
starts_with_one_bit_test = () {
	assert(bit_array.starts_with(<<1:u1>>, <<1:u1>>))
	Nil
}

@test
starts_with_one_bit_empty_test = () {
	assert(bit_array.starts_with(<<1:u1>>, <<>>))
	Nil
}

@test
starts_with_one_bit_longer_test = () {
	assert(core.not(bit_array.starts_with(<<1:u1>>, <<1:u2>>)))
	Nil
}

@test
starts_with_long_test = () {
	assert(bit_array.starts_with(<<-1:u127>>, <<-1:u33>>))
	Nil
}

@test
starts_with_long_longer_test = () {
	assert(core.not(bit_array.starts_with(<<-1:u127>>, <<-1:u(128)>>)))
	Nil
}

@test
starts_with_different_values_test = () {
	assert(core.not(bit_array.starts_with(<<0:u127>>, <<1:u127>>)))
	Nil
}

@test
starts_with_partial_byte_match_test = () {
	assert(bit_array.starts_with(<<0xFF, 0x81>>, <<0xFF, 1:u1>>))
	Nil
}

@test
starts_with_partial_byte_no_match_test = () {
	assert(core.not(bit_array.starts_with(<<0xFF, 0x81>>, <<0xFF, 0:u1>>)))
	Nil
}

// Tests for functions that are new in std, or that changed from the old
// library.

@test
base64_round_trip_test = () {
	check = (bytes: BitArray) {
		assert(
			bit_array.base64_decode(bit_array.base64_encode(bytes, True)) ==
				Ok(bytes)
		)
		assert(
			bit_array.base64_decode(bit_array.base64_encode(bytes, False)) ==
				Ok(bytes)
		)
		assert(
			bit_array.base64_url_decode(
				bit_array.base64_url_encode(bytes, True)
			) ==
				Ok(bytes)
		)
	}
	check(<<>>)
	check(<<0>>)
	check(<<255, 254>>)
	check(<<1, 2, 3>>)
	check(<<251, 255, 191, 0, 1>>)
	check(bit_array.concat(list.map(list.range(0, 255), (i) { <<i>> })))
}

@test
base64_rfc4648_test = () {
	// The test vectors of RFC 4648, section 10.
	assert(bit_array.base64_encode(<<"":utf8>>, True) == "")
	assert(bit_array.base64_encode(<<"f":utf8>>, True) == "Zg==")
	assert(bit_array.base64_encode(<<"fo":utf8>>, True) == "Zm8=")
	assert(bit_array.base64_encode(<<"foo":utf8>>, True) == "Zm9v")
	assert(bit_array.base64_encode(<<"foob":utf8>>, True) == "Zm9vYg==")
	assert(bit_array.base64_encode(<<"fooba":utf8>>, True) == "Zm9vYmE=")
	assert(bit_array.base64_encode(<<"foobar":utf8>>, True) == "Zm9vYmFy")
	assert(bit_array.base64_decode("Zm9vYmE=") == Ok(<<"fooba":utf8>>))
	assert(bit_array.base64_decode("Zm9vYmE") == Ok(<<"fooba":utf8>>))
}

@test
base64_invalid_test = () {
	assert(bit_array.base64_decode("Z") == Error(Nil))
	assert(bit_array.base64_decode("Zm9v!") == Error(Nil))
	assert(bit_array.base64_decode("Zm=v") == Error(Nil))
	assert(bit_array.base64_decode("-_8") == Error(Nil))
	assert(bit_array.base64_url_decode("-_8") == Ok(<<251, 255>>))
}

@test
base64_long_test = () {
	bytes = bit_array.concat(list.repeat(<<1, 2, 3>>, 30_000))
	encoded = bit_array.base64_encode(bytes, True)
	assert(string.byte_size(encoded) == 120_000)
	assert(bit_array.base64_decode(encoded) == Ok(bytes))
}

@test
base16_more_test = () {
	assert(bit_array.base16_encode(<<255, 1, 171>>) == "FF01AB")
	assert(bit_array.base16_decode("ff01Ab") == Ok(<<255, 1, 171>>))
	assert(bit_array.base16_decode("f") == Error(Nil))
	assert(bit_array.base16_decode("zz") == Error(Nil))
	assert(bit_array.base16_encode(<<15:u4>>) == "F0")
}

@test
concat_many_test = () {
	bits = bit_array.concat(list.repeat(<<1:u1>>, 10_000))
	assert(bit_array.bit_size(bits) == 10_000)
	assert(bit_array.byte_size(bits) == 1250)
}

@test
slice_negative_take_test = () {
	assert(bit_array.slice(<<1, 2, 3, 4>>, at: 3, take: -2) == Ok(<<2, 3>>))
	assert(bit_array.slice(<<1, 2, 3, 4>>, at: 1, take: -2) == Error(Nil))
}
