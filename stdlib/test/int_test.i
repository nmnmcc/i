(
	i/fail
	testlib = (test) = i/test
	i/core
	int = i/int
	string = i/string
	list = i/list
	order = (Less, Equal, Greater) = i/order
	option = (Some, None) = i/option
)

test("absolute_value_positive_test", () {
	assert(int.abs(123) == 123)
	()
})

test("absolute_value_negative_test", () {
	assert(int.abs(-123) == 123)
	()
})

test("clamp_within_range_test", () {
	assert(int.clamp(40, 30, 50) == 40)
	()
})

test("clamp_below_min_test", () {
	assert(int.clamp(20, 30, 50) == 30)
	()
})

test("clamp_above_max_test", () {
	assert(int.clamp(60, 30, 50) == 50)
	()
})

test("clamp_inverted_range_above_test", () {
	assert(int.clamp(100, 50, 30) == 50)
	()
})

test("clamp_inverted_range_within_test", () {
	assert(int.clamp(40, 50, 30) == 40)
	()
})

test("to_string_positive_test", () {
	assert(int.to_string(123) == "123")
	()
})

test("to_string_negative_test", () {
	assert(int.to_string(-123) == "-123")
	()
})

test("to_string_positive_2_test", () {
	assert(int.to_string(123) == "123")
	()
})

test("parse_positive_test", () {
	assert(fail.catch(() { int.parse("123") }) == Ok(123))
	()
})

test("parse_negative_test", () {
	assert(fail.catch(() { int.parse("-123") }) == Ok(-123))
	()
})

test("parse_leading_zero_test", () {
	assert(fail.catch(() { int.parse("0123") }) == Ok(123))
	()
})

test("parse_empty_test", () {
	assert(
		fail.catch(() { int.parse("") }) ==
			Error(int.IntError(offset = 0, reason = "empty")),
	)
	()
})

test("parse_invalid_test", () {
	assert(
		fail.catch(() { int.parse("what") }) ==
			Error(int.IntError(offset = 0, reason = "invalid digit")),
	)
	()
})

test("parse_float_test", () {
	assert(
		fail.catch(() { int.parse("1.23") }) ==
			Error(int.IntError(offset = 1, reason = "invalid digit")),
	)
	()
})

test("base_parse_hex_test", () {
	assert(fail.catch(() { int.parse_base("100", 16) }) == Ok(256))
	()
})

test("base_parse_hex_negative_test", () {
	assert(fail.catch(() { int.parse_base("-100", 16) }) == Ok(-256))
	()
})

test("base_parse_base_1_test", () {
	assert(
		fail.catch(() { int.parse_base("100", 1) }) ==
			Error(int.IntError(offset = 0, reason = "base out of range")),
	)
	()
})

test("base_parse_base_37_test", () {
	assert(
		fail.catch(() { int.parse_base("100", 37) }) ==
			Error(int.IntError(offset = 0, reason = "base out of range")),
	)
	()
})

test("base_parse_invalid_hex_test", () {
	assert(
		fail.catch(() { int.parse_base("AG", 16) }) ==
			Error(
				int.IntError(
					offset = 1,
					reason = "digit out of range for base",
				),
			),
	)
	()
})

test("to_base_string_hex_test", () {
	assert(fail.catch(() { int.to_base(100, 16) }) == Ok("64"))
	()
})

test("to_base_string_hex_negative_test", () {
	assert(fail.catch(() { int.to_base(-100, 16) }) == Ok("-64"))
	()
})

test("to_base_string_base_1_test", () {
	assert(fail.catch(() { int.to_base(100, 1) }) == Error(()))
	()
})

test("to_base_string_base_37_test", () {
	assert(fail.catch(() { int.to_base(100, 37) }) == Error(()))
	()
})

test("to_base2_positive_test", () {
	assert(int.to_base2(100) == "1100100")
	()
})

test("to_base2_negative_test", () {
	assert(int.to_base2(-100) == "-1100100")
	()
})

test("to_base8_positive_test", () {
	assert(int.to_base8(100) == "144")
	()
})

test("to_base8_negative_test", () {
	assert(int.to_base8(-100) == "-144")
	()
})

test("to_base16_positive_test", () {
	assert(int.to_base16(100) == "64")
	()
})

test("to_base16_negative_test", () {
	assert(int.to_base16(-100) == "-64")
	()
})

test("to_base16_large_test", () {
	assert(int.to_base16(43_981) == "ABCD")
	()
})

test("to_base16_large_negative_test", () {
	assert(int.to_base16(-43_981) == "-ABCD")
	()
})

test("to_base36_positive_test", () {
	assert(int.to_base36(100) == "2S")
	()
})

test("to_base36_negative_test", () {
	assert(int.to_base36(-100) == "-2S")
	()
})

test("to_float_one_test", () {
	assert(int.to_float(1) == 1.0)
	()
})

test("to_float_five_test", () {
	assert(int.to_float(5) == 5.0)
	()
})

test("to_float_zero_test", () {
	assert(int.to_float(0) == 0.0)
	()
})

test("to_float_negative_test", () {
	assert(int.to_float(-5) == -.5.0)
	()
})

test("compare_equal_zero_test", () {
	assert(int.compare(0, 0) == order.Equal())
	()
})

test("compare_equal_one_test", () {
	assert(int.compare(1, 1) == order.Equal())
	()
})

test("compare_lt_test", () {
	assert(int.compare(0, 1) == order.Less())
	()
})

test("compare_negative_lt_test", () {
	assert(int.compare(-2, -1) == order.Less())
	()
})

test("compare_gt_test", () {
	assert(int.compare(2, 1) == order.Greater())
	()
})

test("compare_negative_gt_test", () {
	assert(int.compare(-1, -2) == order.Greater())
	()
})

test("min_equal_test", () {
	assert(int.min(0, 0) == 0)
	()
})

test("min_first_smaller_test", () {
	assert(int.min(0, 1) == 0)
	()
})

test("min_second_smaller_test", () {
	assert(int.min(1, 0) == 0)
	()
})

test("min_negative_smaller_test", () {
	assert(int.min(-1, 2) == -1)
	()
})

test("min_both_mixed_test", () {
	assert(int.min(2, -2) == -2)
	()
})

test("min_both_negative_test", () {
	assert(int.min(-1, -1) == -1)
	()
})

test("max_equal_test", () {
	assert(int.max(0, 0) == 0)
	()
})

test("max_second_larger_test", () {
	assert(int.max(0, 1) == 1)
	()
})

test("max_first_larger_test", () {
	assert(int.max(1, 0) == 1)
	()
})

test("max_positive_larger_test", () {
	assert(int.max(-1, 2) == 2)
	()
})

test("max_mixed_test", () {
	assert(int.max(2, -2) == 2)
	()
})

test("max_both_negative_test", () {
	assert(int.max(-1, -1) == -1)
	()
})

test("is_even_zero_test", () {
	assert(int.is_even(0))
	()
})

test("is_even_two_test", () {
	assert(int.is_even(2))
	()
})

test("is_even_negative_two_test", () {
	assert(int.is_even(-2))
	()
})

test("is_even_large_test", () {
	assert(int.is_even(10_006))
	()
})

test("is_even_one_false_test", () {
	assert(
		core.not(int.is_even(1)),
	)
	()
})

test("is_even_negative_three_false_test", () {
	assert(
		core.not(int.is_even(-3)),
	)
	()
})

test("is_even_large_odd_false_test", () {
	assert(
		core.not(int.is_even(10_005)),
	)
	()
})

test("is_odd_zero_false_test", () {
	assert(
		core.not(int.is_odd(0)),
	)
	()
})

test("is_odd_two_false_test", () {
	assert(
		core.not(int.is_odd(2)),
	)
	()
})

test("is_odd_negative_two_false_test", () {
	assert(
		core.not(int.is_odd(-2)),
	)
	()
})

test("is_odd_large_even_false_test", () {
	assert(
		core.not(int.is_odd(10_006)),
	)
	()
})

test("is_odd_one_test", () {
	assert(int.is_odd(1))
	()
})

test("is_odd_negative_three_test", () {
	assert(int.is_odd(-3))
	()
})

test("is_odd_large_test", () {
	assert(int.is_odd(10_005))
	()
})

test("square_root_four_test", () {
	assert(fail.catch(() { int.square_root(4) }) == Ok(2.0))
	()
})

test("square_root_sixteen_test", () {
	assert(fail.catch(() { int.square_root(16) }) == Ok(4.0))
	()
})

test("square_root_zero_test", () {
	assert(fail.catch(() { int.square_root(0) }) == Ok(0.0))
	()
})

test("square_root_negative_error_test", () {
	assert(fail.catch(() { int.square_root(-4) }) == Error(()))
	()
})

test("negate_negative_test", () {
	assert(int.negate(-1) == 1)
	()
})

test("negate_positive_test", () {
	assert(int.negate(2) == -2)
	()
})

test("negate_zero_test", () {
	assert(int.negate(0) == 0)
	()
})

test("sum_empty_test", () {
	assert(int.sum([]) == 0)
	()
})

test("sum_non_empty_test", () {
	assert(int.sum([1, 2, 3]) == 6)
	()
})

test("product_empty_test", () {
	assert(int.product([]) == 1)
	()
})

test("product_single_test", () {
	assert(int.product([4]) == 4)
	()
})

test("product_multiple_test", () {
	assert(int.product([1, 2, 3]) == 6)
	()
})

test("divide_by_self_test", () {
	assert(fail.catch(() { int.divide(1, 1) }) == Ok(1))
	()
})

test("divide_by_zero_test", () {
	assert(fail.catch(() { int.divide(1, 0) }) == Error(()))
	()
})

test("divide_zero_by_one_test", () {
	assert(fail.catch(() { int.divide(0, 1) }) == Ok(0))
	()
})

test("divide_one_by_zero_test", () {
	assert(fail.catch(() { int.divide(1, 0) }) == Error(()))
	()
})

test("divide_truncates_test", () {
	assert(fail.catch(() { int.divide(5, 2) }) == Ok(2))
	()
})

test("divide_negative_test", () {
	assert(fail.catch(() { int.divide(-99, 2) }) == Ok(-49))
	()
})

test("remainder_basic_test", () {
	assert(fail.catch(() { int.remainder(3, 2) }) == Ok(1))
	()
})

test("remainder_by_zero_test", () {
	assert(fail.catch(() { int.remainder(1, 0) }) == Error(()))
	()
})

test("remainder_negative_divisor_test", () {
	assert(fail.catch(() { int.remainder(10, -1) }) == Ok(0))
	()
})

test("remainder_positive_test", () {
	assert(fail.catch(() { int.remainder(13, 3) }) == Ok(1))
	()
})

test("remainder_negative_dividend_test", () {
	assert(fail.catch(() { int.remainder(-13, 3) }) == Ok(-1))
	()
})

test("remainder_positive_by_negative_test", () {
	assert(fail.catch(() { int.remainder(13, -3) }) == Ok(1))
	()
})

test("remainder_both_negative_test", () {
	assert(fail.catch(() { int.remainder(-13, -3) }) == Ok(-1))
	()
})

test("modulo_basic_test", () {
	assert(fail.catch(() { int.modulo(3, 2) }) == Ok(1))
	()
})

test("modulo_by_zero_test", () {
	assert(fail.catch(() { int.modulo(1, 0) }) == Error(()))
	()
})

test("modulo_negative_divisor_test", () {
	assert(fail.catch(() { int.modulo(10, -1) }) == Ok(0))
	()
})

test("modulo_positive_test", () {
	assert(fail.catch(() { int.modulo(13, 3) }) == Ok(1))
	()
})

test("modulo_negative_dividend_test", () {
	assert(fail.catch(() { int.modulo(-13, 3) }) == Ok(2))
	()
})

test("modulo_positive_by_negative_test", () {
	assert(fail.catch(() { int.modulo(13, -3) }) == Ok(-2))
	()
})

test("modulo_both_negative_test", () {
	assert(fail.catch(() { int.modulo(-13, -3) }) == Ok(-1))
	()
})

test("floor_divide_by_self_test", () {
	assert(fail.catch(() { int.floor_divide(1, 1) }) == Ok(1))
	()
})

test("floor_divide_by_zero_test", () {
	assert(fail.catch(() { int.floor_divide(1, 0) }) == Error(()))
	()
})

test("floor_divide_zero_by_one_test", () {
	assert(fail.catch(() { int.floor_divide(0, 1) }) == Ok(0))
	()
})

test("floor_divide_one_by_zero_test", () {
	assert(fail.catch(() { int.floor_divide(1, 0) }) == Error(()))
	()
})

test("floor_divide_truncates_test", () {
	assert(fail.catch(() { int.floor_divide(5, 2) }) == Ok(2))
	()
})

test("floor_divide_negative_divisor_test", () {
	assert(fail.catch(() { int.floor_divide(6, -4) }) == Ok(-2))
	()
})

test("floor_divide_negative_dividend_test", () {
	assert(fail.catch(() { int.floor_divide(-99, 2) }) == Ok(-50))
	()
})

test("floor_divide_negative_small_test", () {
	assert(fail.catch(() { int.floor_divide(-1, 2) }) == Ok(-1))
	()
})

test("add_test", () {
	assert(int.add(1, 2) == 3)
	()
})

test("add_larger_test", () {
	assert(int.add(3, 2) == 5)
	()
})

test("multiply_test", () {
	assert(int.multiply(2, 4) == 8)
	()
})

test("multiply_larger_test", () {
	assert(int.multiply(3, 2) == 6)
	()
})

test("subtract_test", () {
	assert(int.subtract(3, 1) == 2)
	()
})

test("subtract_smaller_test", () {
	assert(int.subtract(3, 2) == 1)
	()
})

test("subtract_negative_result_test", () {
	assert(int.subtract(2, 3) == -1)
	()
})

test("bitwise_and_test", () {
	assert(int.bitwise_and(9, 3) == 1)
	()
})

test("bitwise_and_32bit_test", () {
	// To check compatibility with JavaScript, try a 32 bit unsigned integer
	// (signed integers are in the range -2147483648 to +2147483647, while
	//  32 bit unsigned integers are in the range 0 to +4294967295).
	assert(int.bitwise_and(2_147_483_648, 2_147_483_648) == 2_147_483_648)
	()
})

test("bitwise_not_test", () {
	assert(int.bitwise_not(2) == -3)
	()
})

test("bitwise_not_32bit_test", () {
	// To check compatibility with JavaScript, try a 32 bit unsigned integer.
	assert(int.bitwise_not(2_147_483_648) == -2_147_483_649)
	()
})

test("bitwise_or_test", () {
	assert(int.bitwise_or(9, 3) == 11)
	()
})

test("bitwise_or_32bit_test", () {
	// To check compatibility with JavaScript, try a 32 bit unsigned integer.
	assert(int.bitwise_or(1, 2_147_483_648) == 2_147_483_649)
	()
})

test("bitwise_exclusive_or_test", () {
	assert(int.bitwise_xor(9, 3) == 10)
	()
})

test("bitwise_exclusive_or_32bit_test", () {
	// To check compatibility with JavaScript, try a 32 bit unsigned integer.
	assert(int.bitwise_xor(0, 2_147_483_648) == 2_147_483_648)
	()
})

test("bitwise_shift_left_positive_test", () {
	assert(int.bitwise_shift_left(1, 2) == 4)
	()
})

test("bitwise_shift_left_negative_shift_test", () {
	assert(int.bitwise_shift_left(1, -2) == 0)
	()
})

test("bitwise_shift_left_negative_value_test", () {
	assert(int.bitwise_shift_left(-1, 2) == -4)
	()
})

test("bitwise_shift_left_both_negative_test", () {
	assert(int.bitwise_shift_left(-1, -2) == -1)
	()
})

test("bitwise_shift_right_positive_test", () {
	assert(int.bitwise_shift_right(1, 2) == 0)
	()
})

test("bitwise_shift_right_negative_shift_test", () {
	assert(int.bitwise_shift_right(1, -2) == 4)
	()
})

test("bitwise_shift_right_negative_value_test", () {
	assert(int.bitwise_shift_right(-1, 2) == -1)
	()
})

test("bitwise_shift_right_both_negative_test", () {
	assert(int.bitwise_shift_right(-1, -2) == -4)
	()
})

test("bitwise_and_zero_test", () {
	assert(int.bitwise_and(0, 0) == 0)
	()
})

test("bitwise_and_identity_test", () {
	assert(int.bitwise_and(0xFF, 0xFF) == 0xFF)
	()
})

test("bitwise_and_clear_test", () {
	assert(int.bitwise_and(0xFF, 0) == 0)
	()
})

test("bitwise_and_large_numbers_test", () {
	// Test with numbers larger than 32 bits
	assert(int.bitwise_and(0x123456789, 0xFFFF) == 0x6789)
	()
})

test("bitwise_or_zero_test", () {
	assert(int.bitwise_or(0, 0) == 0)
	()
})

test("bitwise_or_identity_test", () {
	assert(int.bitwise_or(0xFF, 0) == 0xFF)
	()
})

test("bitwise_or_combine_test", () {
	assert(int.bitwise_or(0xF0, 0x0F) == 0xFF)
	()
})

test("bitwise_or_large_numbers_test", () {
	// Test with numbers larger than 32 bits
	assert(int.bitwise_or(0x100000000, 0xFF) == 0x1000000FF)
	()
})

test("bitwise_exclusive_or_zero_test", () {
	assert(int.bitwise_xor(0, 0) == 0)
	()
})

test("bitwise_exclusive_or_identity_test", () {
	assert(int.bitwise_xor(0xFF, 0) == 0xFF)
	()
})

test("bitwise_exclusive_or_invert_test", () {
	assert(int.bitwise_xor(0xF0, 0x0F) == 0xFF)
	()
})

test("bitwise_exclusive_or_self_test", () {
	assert(int.bitwise_xor(0xAB, 0xAB) == 0)
	()
})

test("bitwise_not_zero_test", () {
	assert(int.bitwise_not(0) == -1)
	()
})

test("bitwise_not_negative_one_test", () {
	assert(int.bitwise_not(-1) == 0)
	()
})

test("bitwise_not_large_number_test", () {
	// Test with number larger than 32 bits
	assert(int.bitwise_not(0x123456789) == -4_886_718_346)
	()
})

test("bitwise_shift_left_zero_shift_test", () {
	assert(int.bitwise_shift_left(0xFF, 0) == 0xFF)
	()
})

test("bitwise_shift_left_large_shift_test", () {
	assert(int.bitwise_shift_left(1, 32) == 4_294_967_296)
	()
})

test("bitwise_shift_left_large_number_test", () {
	assert(int.bitwise_shift_left(0x1000, 4) == 0x10000)
	()
})

test("bitwise_shift_right_zero_shift_test", () {
	assert(int.bitwise_shift_right(0xFF, 0) == 0xFF)
	()
})

test("bitwise_shift_right_large_shift_test", () {
	assert(int.bitwise_shift_right(0xFFFFFFFF, 16) == 0xFFFF)
	()
})

test("bitwise_shift_right_preserves_sign_test", () {
	assert(int.bitwise_shift_right(-1, 16) == -1)
	()
})

test("bitwise_shift_right_large_number_test", () {
	assert(int.bitwise_shift_right(0x123456789, 16) == 0x12345)
	()
})

test("bitwise_operations_with_max_safe_integer_test", () {
	// Test with maximum safe integer
	assert(int.bitwise_and(9_007_199_254_740_991, 0xFFFF) == 65_535)
	()
})

test("bitwise_operations_with_boundary_values_test", () {
	// Test around 32-bit boundaries
	assert(int.bitwise_or(2_147_483_647, 1) == 2_147_483_647)
	// MAX_INT32 | 1 = MAX_INT32
	assert(int.bitwise_and(2_147_483_648, 4_294_967_295) == 2_147_483_648)
	// 2^31 & 0xFFFFFFFF = 2^31
	()
})

test("bitwise_shift_edge_cases_test", () {
	// Test edge cases for shifts
	assert(int.bitwise_shift_left(1, 63) == 9_223_372_036_854_775_808)
	assert(int.bitwise_shift_right(9_223_372_036_854_775_808, 63) == 1)
	()
})

test("range_ascending_test", () {
	assert(int.range(0, 3, "", (acc, i) { acc <> int.to_string(i) }) == "012")
	()
})

test("range_descending_test", () {
	assert(int.range(3, 0, "", (acc, i) { acc <> int.to_string(i) }) == "321")
	()
})

test("range_empty_test", () {
	assert(int.range(5, 5, "", (acc, i) { acc <> int.to_string(i) }) == "")
	()
})

test("range_with_list_prepend_test", () {
	assert(int.range(1, -2, [], list.prepend) == [-1, 0, 1])
	()
})

test("range_negative_to_positive_test", () {
	assert(int.range(-2, 2, [], (acc, i) { [i, ..acc] }) == [1, 0, -1, -2])
	()
})

test("range_single_element_test", () {
	assert(int.range(0, 1, [], list.prepend) == [0])
	()
})

test("range_sum_test", () {
	assert(int.range(1, 5, 0, int.add) == 10)
	()
})

test("power_two_squared_test", () {
	assert(fail.catch(() { int.power(2, 2) }) == Ok(4))
})

test("power_negative_cubed_test", () {
	assert(fail.catch(() { int.power(-5, 3) }) == Ok(-125))
})

test("power_zero_exponent_test", () {
	assert(fail.catch(() { int.power(10, 0) }) == Ok(1))
	assert(fail.catch(() { int.power(0, 0) }) == Ok(1))
})

test("power_big_test", () {
	assert(
		fail.catch(() { int.power(2, 100) }) ==
			Ok(1267650600228229401496703205376),
	)
	assert(
		fail.catch(() { int.power(10, 30) }) ==
			Ok(1_000_000_000_000_000_000_000_000_000_000),
	)
})

test("power_negative_exponent_test", () {
	assert(fail.catch(() { int.power(2, -1) }) == Error(()))
})

test("power_pipe_test", () {
	assert(fail.catch(() { 3 |> int.power(4) }) == Ok(81))
})

// Tests for functions that are new in std, or that changed from the old
// library.
test("parse_more_test", () {
	assert(fail.catch(() { int.parse("+7") }) == Ok(7))
	assert(fail.catch(() { int.parse("-0") }) == Ok(0))
	assert(
		fail.catch(() { int.parse("123456789012345678901234567890") }) ==
			Ok(123456789012345678901234567890),
	)
	assert(
		fail.catch(() { int.parse("") }) ==
			Error(int.IntError(offset = 0, reason = "empty")),
	)
	assert(
		fail.catch(() { int.parse("-") }) ==
			Error(int.IntError(offset = 1, reason = "missing digits")),
	)
	assert(
		fail.catch(() { int.parse("1_000") }) ==
			Error(int.IntError(offset = 1, reason = "invalid digit")),
	)
	assert(
		fail.catch(() { int.parse(" 1") }) ==
			Error(int.IntError(offset = 0, reason = "invalid digit")),
	)
	assert(
		fail.catch(() { int.parse("1e3") }) ==
			Error(int.IntError(offset = 1, reason = "invalid digit")),
	)
	assert(
		fail.catch(() { int.parse("٣") }) ==
			Error(int.IntError(offset = 0, reason = "invalid digit")),
	)
})

test("parse_error_reasons_test", () {
	assert(
		fail.catch(() { int.parse("nope") }) ==
			Error(int.IntError(offset = 0, reason = "invalid digit")),
	)
	assert(fail.catch(() { int.parse("-42") }) == Ok(-42))
	assert(
		core.inspect(fail.catch(() { int.parse("12x") })) ==
			"Error(IntError(offset = 2, reason = \"invalid digit\"))",
	)
})

test("parse_base_more_test", () {
	assert(fail.catch(() { int.parse_base("ff", 16) }) == Ok(255))
	assert(fail.catch(() { int.parse_base("FF", 16) }) == Ok(255))
	assert(fail.catch(() { int.parse_base("-101", 2) }) == Ok(-5))
	assert(fail.catch(() { int.parse_base("z", 36) }) == Ok(35))
	assert(
		fail.catch(() { int.parse_base("2", 2) }) ==
			Error(
				int.IntError(
					offset = 0,
					reason = "digit out of range for base",
				),
			),
	)
	assert(
		fail.catch(() { int.parse_base("1", 1) }) ==
			Error(int.IntError(offset = 0, reason = "base out of range")),
	)
	assert(
		fail.catch(() { int.parse_base("1", 37) }) ==
			Error(int.IntError(offset = 0, reason = "base out of range")),
	)
})

test("to_base_more_test", () {
	assert(fail.catch(() { int.to_base(255, 16) }) == Ok("FF"))
	assert(fail.catch(() { int.to_base(-5, 2) }) == Ok("-101"))
	assert(fail.catch(() { int.to_base(0, 7) }) == Ok("0"))
	assert(fail.catch(() { int.to_base(35, 36) }) == Ok("Z"))
	assert(fail.catch(() { int.to_base(1, 37) }) == Error(()))
	assert(fail.catch(() { int.to_base(1, 1) }) == Error(()))
	big = 1267650600228229401496703205376
	assert(
		fail.catch(() { int.to_base(big, 2) }) ==
			Ok("1" <> string.repeat("0", 100)),
	)
	assert(
		{
			e4_call_arg_0 = int.to_base16(big)
			e4_call_arg_1 = 16
			fail.catch(() { int.parse_base(e4_call_arg_0, e4_call_arg_1) })
		} ==
			Ok(big),
	)
})

test("digits_more_test", () {
	assert(fail.catch(() { int.digits(234, 10) }) == Ok([2, 3, 4]))
	assert(fail.catch(() { int.digits(-234, 10) }) == Ok([2, 3, 4]))
	assert(fail.catch(() { int.digits(0, 10) }) == Ok([0]))
	assert(fail.catch(() { int.digits(5, 2) }) == Ok([1, 0, 1]))
	assert(fail.catch(() { int.digits(5, 1) }) == Error(()))
	assert(fail.catch(() { int.undigits([1, 0, 1], 2) }) == Ok(5))
	assert(fail.catch(() { int.undigits([1, 2], 2) }) == Error(()))
	assert(fail.catch(() { int.undigits([], 10) }) == Ok(0))
})

test("division_more_test", () {
	assert(fail.catch(() { int.divide(7, 2) }) == Ok(3))
	assert(fail.catch(() { int.divide(-7, 2) }) == Ok(-3))
	assert(fail.catch(() { int.divide(7, 0) }) == Error(()))
	assert(fail.catch(() { int.remainder(-13, 3) }) == Ok(-1))
	assert(fail.catch(() { int.remainder(13, -3) }) == Ok(1))
	assert(fail.catch(() { int.modulo(-13, 3) }) == Ok(2))
	assert(fail.catch(() { int.modulo(13, -3) }) == Ok(-2))
	assert(fail.catch(() { int.modulo(-13, -3) }) == Ok(-1))
	assert(fail.catch(() { int.modulo(12, 3) }) == Ok(0))
	assert(fail.catch(() { int.floor_divide(-7, 2) }) == Ok(-4))
	assert(fail.catch(() { int.floor_divide(7, -2) }) == Ok(-4))
	assert(fail.catch(() { int.floor_divide(-8, 2) }) == Ok(-4))
	assert(fail.catch(() { int.floor_divide(1, 0) }) == Error(()))
})

test("big_numbers_test", () {
	big = 100000000000000000000
	assert(int.abs(-big) == big)
	assert(int.compare(big, big + 1) == Less())
	assert(int.max(big, -big) == big)
	assert(int.sum([big, big]) == 2 * big)
	assert(int.to_string(-big) == "-100000000000000000000")
	assert(int.is_even(big))
})

test("sum_product_long_test", () {
	xs = list.repeat(1, 1_000_000)
	assert(int.sum(xs) == 1_000_000)
	assert(int.product(xs) == 1)
})

test("range_long_test", () {
	assert(int.range(0, 1_000_000, 0, int.add) == 499_999_500_000)
})
