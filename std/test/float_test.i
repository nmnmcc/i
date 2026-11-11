(
	i/core
	float = std/float
	int = std/int
	list = std/list
	order = (Less, Equal, Greater) = std/order
	option = (Some, None) = std/option
	property = std/property
	result = std/result
)

@test
parse_positive_test = () {
	assert(float.parse("1.23") == Ok(1.23))
	Nil
}

@test
parse_positive_with_plus_test = () {
	assert(float.parse("+1.23") == Ok(1.23))
	Nil
}

@test
parse_negative_test = () {
	assert(float.parse("-1.23") == Ok(-.1.23))
	Nil
}

@test
parse_whole_test = () {
	assert(float.parse("5.0") == Ok(5.0))
	Nil
}

@test
parse_many_decimal_places_test = () {
	assert(float.parse("0.123456789") == Ok(0.123456789))
	Nil
}

@test
parse_scientific_test = () {
	assert(float.parse("1.234e10") == Ok(1.234e10))
	Nil
}

@test
parse_scientific_negative_positive_test = () {
	assert(float.parse("-1.234e+10") == Ok(-.1.234e10))
	Nil
}

@test
parse_scientific_negative_exponent_test = () {
	assert(float.parse("1.234e-10") == Ok(1.234e-10))
	Nil
}

@test
parse_empty_test = () {
	assert(float.parse("") == Error(Nil))
	Nil
}

@test
parse_invalid_test = () {
	assert(float.parse("what") == Error(Nil))
	Nil
}

@test
parse_integer_test = () {
	assert(float.parse("1") == Error(Nil))
	Nil
}

@test
to_string_zero_test = () {
	assert(float.to_string(0.0) == "0.0")
	Nil
}

@test
to_string_small_positive_test = () {
	assert(float.to_string(0.0123) == "0.0123")
	Nil
}

@test
to_string_small_negative_test = () {
	assert(float.to_string(-.0.0123) == "-0.0123")
	Nil
}

@test
to_string_positive_test = () {
	assert(float.to_string(12.67) == "12.67")
	Nil
}

@test
to_string_negative_test = () {
	assert(float.to_string(-.12.67) == "-12.67")
	Nil
}

@test
to_string_whole_positive_test = () {
	assert(float.to_string(123.0) == "123.0")
	Nil
}

@test
to_string_whole_negative_test = () {
	assert(float.to_string(-.123.0) == "-123.0")
	Nil
}

@test
to_string_scientific_positive_test = () {
	assert(float.to_string(3.0e26) == "3.0e26")
	Nil
}

@test
to_string_scientific_negative_test = () {
	assert(float.to_string(-.3.0e26) == "-3.0e26")
	Nil
}

@test
to_string_scientific_small_positive_test = () {
	assert(float.to_string(3.0e-26) == "3.0e-26")
	Nil
}

@test
to_string_scientific_small_negative_test = () {
	assert(float.to_string(-.3.0e-26) == "-3.0e-26")
	Nil
}

@test
to_string_scientific_large_test = () {
	assert(float.to_string(456.12e78) == "4.5612e80")
	Nil
}

@test
to_string_scientific_large_negative_test = () {
	assert(float.to_string(-.456.12e78) == "-4.5612e80")
	Nil
}

@test
to_string_scientific_small_test = () {
	assert(float.to_string(456.12e-78) == "4.5612e-76")
	Nil
}

@test
to_string_scientific_small_negative_2_test = () {
	assert(float.to_string(-.456.12e-78) == "-4.5612e-76")
	Nil
}

@test
clamp_within_range_test = () {
	assert(float.clamp(1.4, min: 1.3, max: 1.5) == 1.4)
	Nil
}

@test
clamp_below_min_test = () {
	assert(float.clamp(1.2, min: 1.3, max: 1.5) == 1.3)
	Nil
}

@test
clamp_above_max_test = () {
	assert(float.clamp(1.6, min: 1.3, max: 1.5) == 1.5)
	Nil
}

@test
clamp_inverted_range_test = () {
	assert(float.clamp(1.2, min: 1.4, max: 0.6) == 1.2)
	Nil
}

@test
compare_equal_zero_test = () {
	assert(float.compare(0.0, 0.0) == order.Equal)
	Nil
}

@test
compare_equal_test = () {
	assert(float.compare(0.1, 0.1) == order.Equal)
	Nil
}

@test
compare_lt_test = () {
	assert(float.compare(0.0, 0.1) == order.Less)
	Nil
}

@test
compare_negative_lt_test = () {
	assert(float.compare(-.2.0, -.1.9) == order.Less)
	Nil
}

@test
compare_gt_test = () {
	assert(float.compare(2.0, 1.9) == order.Greater)
	Nil
}

@test
compare_negative_gt_test = () {
	assert(float.compare(-.1.9, -.2.0) == order.Greater)
	Nil
}

@test
loosely_compare_lt_no_tolerance_test = () {
	assert(float.loosely_compare(10.2, 10.5, tolerating: 0.0) == order.Less)
	Nil
}

@test
loosely_compare_eq_with_tolerance_test = () {
	assert(
		float.loosely_compare(10.2, with: 10.5, tolerating: 0.31) == order.Equal
	)
	Nil
}

@test
loosely_compare_eq_reversed_test = () {
	assert(float.loosely_compare(10.5, 10.2, 0.31) == order.Equal)
	Nil
}

@test
loosely_compare_lt_insufficient_tolerance_test = () {
	assert(float.loosely_compare(10.2, 10.5, 0.29) == order.Less)
	Nil
}

@test
loosely_compare_gt_insufficient_tolerance_test = () {
	assert(float.loosely_compare(10.5, 10.2, 0.29) == order.Greater)
	Nil
}

@test
loosely_compare_negative_eq_test = () {
	assert(float.loosely_compare(-.10.2, -.10.5, 0.31) == order.Equal)
	Nil
}

@test
loosely_equals_no_tolerance_test = () {
	assert(core.not(float.loosely_equals(10.2, 10.5, tolerating: 0.0)))
	Nil
}

@test
loosely_equals_with_tolerance_test = () {
	assert(float.loosely_equals(10.2, with: 10.5, tolerating: 0.31))
	Nil
}

@test
loosely_equals_reversed_test = () {
	assert(float.loosely_equals(10.5, 10.2, 0.31))
	Nil
}

@test
loosely_equals_insufficient_tolerance_first_test = () {
	assert(core.not(float.loosely_equals(10.2, 10.5, 0.29)))
	Nil
}

@test
loosely_equals_insufficient_tolerance_second_test = () {
	assert(core.not(float.loosely_equals(10.5, 10.2, 0.29)))
	Nil
}

@test
loosely_equals_negative_test = () {
	assert(float.loosely_equals(-.10.2, -.10.5, 0.31))
	Nil
}

@test
ceiling_positive_test = () {
	assert(float.ceiling(8.1) == 9.0)
	Nil
}

@test
ceiling_negative_test = () {
	assert(float.ceiling(-.8.1) == -.8.0)
	Nil
}

@test
ceiling_whole_negative_test = () {
	assert(float.ceiling(-.8.0) == -.8.0)
	Nil
}

@test
floor_positive_test = () {
	assert(float.floor(8.1) == 8.0)
	Nil
}

@test
floor_negative_test = () {
	assert(float.floor(-.8.1) == -.9.0)
	Nil
}

@test
floor_whole_negative_test = () {
	assert(float.floor(-.8.0) == -.8.0)
	Nil
}

@test
round_down_test = () {
	assert(float.round(8.1) == 8)
	Nil
}

@test
round_down_4_test = () {
	assert(float.round(8.4) == 8)
	Nil
}

@test
round_down_499_test = () {
	assert(float.round(8.499) == 8)
	Nil
}

@test
round_up_test = () {
	assert(float.round(8.5) == 9)
	Nil
}

@test
round_negative_test = () {
	assert(float.round(-.8.1) == -8)
	Nil
}

@test
round_negative_half_test = () {
	assert(float.round(-.7.5) == -8)
	Nil
}

@test
truncate_positive_1_test = () {
	assert(float.truncate(8.1) == 8)
	Nil
}

@test
truncate_positive_4_test = () {
	assert(float.truncate(8.4) == 8)
	Nil
}

@test
truncate_positive_499_test = () {
	assert(float.truncate(8.499) == 8)
	Nil
}

@test
truncate_positive_5_test = () {
	assert(float.truncate(8.5) == 8)
	Nil
}

@test
truncate_negative_1_test = () {
	assert(float.truncate(-.8.1) == -8)
	Nil
}

@test
truncate_negative_5_test = () {
	assert(float.truncate(-.7.5) == -7)
	Nil
}

@test
to_precision_round_down_test = () {
	assert(float.to_precision(2.43434348473, 2) == 2.43)
	Nil
}

@test
to_precision_round_up_test = () {
	assert(float.to_precision(2.43534348473, 2) == 2.44)
	Nil
}

@test
to_precision_negative_round_up_test = () {
	assert(float.to_precision(-.2.43534348473, 2) == -.2.44)
	Nil
}

@test
to_precision_negative_precision_up_test = () {
	assert(float.to_precision(547_890.453444, -3) == 548_000.0)
	Nil
}

@test
to_precision_negative_precision_down_test = () {
	assert(float.to_precision(547_490.453444, -3) == 547_000.0)
	Nil
}

@test
to_precision_negative_both_test = () {
	assert(float.to_precision(-.547_490.453444, -3) == -.547_000.0)
	Nil
}

@test
to_precision_zero_test = () {
	assert(float.to_precision(435.3224, 0) == 435.0)
	Nil
}

@test
to_precision_negative_zero_test = () {
	assert(float.to_precision(435.3224, -0) == 435.0)
	Nil
}

@test
to_precision_floating_point_fix_test = () {
	assert(float.to_precision(184.20000000000002, 2) == 184.2)
	Nil
}

@test
to_precision_large_test = () {
	assert(
		float.to_precision(12_345_678_912_345_678_912_345_678.0, -19) ==
			1_234_568.0e19
	)
	Nil
}

@test
min_equal_test = () {
	assert(float.min(0.0, 0.0) == 0.0)
	Nil
}

@test
min_first_smaller_test = () {
	assert(float.min(0.3, 1.5) == 0.3)
	Nil
}

@test
min_second_smaller_test = () {
	assert(float.min(1.0, 0.0) == 0.0)
	Nil
}

@test
min_negative_smaller_test = () {
	assert(float.min(-.1.7, 2.5) == -.1.7)
	Nil
}

@test
min_both_negative_equal_test = () {
	assert(float.min(-.2.2, -.2.2) == -.2.2)
	Nil
}

@test
min_both_negative_equal_2_test = () {
	assert(float.min(-.1.0, -.1.0) == -.1.0)
	Nil
}

@test
min_both_negative_test = () {
	assert(float.min(-.1.1, -.1.0) == -.1.1)
	Nil
}

@test
max_equal_test = () {
	assert(float.max(0.0, 0.0) == 0.0)
	Nil
}

@test
max_second_larger_test = () {
	assert(float.max(0.3, 1.5) == 1.5)
	Nil
}

@test
max_first_larger_test = () {
	assert(float.max(1.0, 0.0) == 1.0)
	Nil
}

@test
max_positive_larger_test = () {
	assert(float.max(-.1.7, 2.5) == 2.5)
	Nil
}

@test
max_both_negative_equal_test = () {
	assert(float.max(-.2.2, -.2.2) == -.2.2)
	Nil
}

@test
max_both_negative_equal_2_test = () {
	assert(float.max(-.1.0, -.1.0) == -.1.0)
	Nil
}

@test
max_both_negative_test = () {
	assert(float.max(-.1.1, -.1.0) == -.1.0)
	Nil
}

@test
absolute_value_negative_test = () {
	assert(float.abs(-.1.0) == 1.0)
	Nil
}

@test
absolute_value_negative_large_test = () {
	assert(float.abs(-.20.6) == 20.6)
	Nil
}

@test
absolute_value_zero_test = () {
	assert(float.abs(0.0) == 0.0)
	Nil
}

@test
absolute_value_positive_test = () {
	assert(float.abs(1.0) == 1.0)
	Nil
}

@test
absolute_value_positive_large_test = () {
	assert(float.abs(25.2) == 25.2)
	Nil
}

@test
power_two_squared_test = () {
	assert(float.real_power(2.0, 2.0) == Ok(4.0))
	Nil
}

@test
power_negative_cubed_test = () {
	assert(float.real_power(-.5.0, 3.0) == Ok(-.125.0))
	Nil
}

@test
power_zero_exponent_test = () {
	assert(float.real_power(10.5, 0.0) == Ok(1.0))
	Nil
}

@test
power_fractional_exponent_test = () {
	assert(float.real_power(16.0, 0.5) == Ok(4.0))
	Nil
}

@test
power_negative_exponent_test = () {
	assert(float.real_power(2.0, -.1.0) == Ok(0.5))
	Nil
}

@test
power_negative_exponent_2_test = () {
	assert(float.real_power(2.0, -.1.0) == Ok(0.5))
	Nil
}

@test
power_negative_base_fractional_exponent_error_test = () {
	// float.real_power(-.1.0, 0.5) is equivalent to float.square_root(-.1.0)
	// and should return an error as an imaginary number would otherwise
	// have to be returned
	assert(float.real_power(-.1.0, 0.5) == Error(Nil))
	Nil
}

@test
power_negative_base_fractional_exponent_error_2_test = () {
	// Check another case with a negative base and fractional exponent
	assert(float.real_power(-.1.5, 1.5) == Error(Nil))
	Nil
}

@test
power_zero_base_negative_exponent_error_test = () {
	// float.real_power(0.0, -.1.0) is equivalent to 1. /. 0 and is expected
	// to be an error
	assert(float.real_power(0.0, -.1.0) == Error(Nil))
	Nil
}

@test
power_negative_base_negative_exponent_test = () {
	// Check that a negative base and exponent is fine as long as the
	// exponent is not fractional
	assert(float.real_power(-.2.0, -.1.0) == Ok(-.0.5))
	Nil
}

@test
square_root_four_test = () {
	assert(float.square_root(4.0) == Ok(2.0))
	Nil
}

@test
square_root_sixteen_test = () {
	assert(float.square_root(16.0) == Ok(4.0))
	Nil
}

@test
square_root_zero_test = () {
	assert(float.square_root(0.0) == Ok(0.0))
	Nil
}

@test
square_root_negative_error_test = () {
	assert(float.square_root(-.4.0) == Error(Nil))
	Nil
}

@test
negate_negative_test = () {
	assert(float.negate(-.1.0) == 1.0)
	Nil
}

@test
negate_positive_test = () {
	assert(float.negate(2.0) == -.2.0)
	Nil
}

@test
negate_double_test = () {
	assert(float.negate(float.negate(0.0)) == 0.0)
	Nil
}

@test
sum_empty_test = () {
	assert(float.sum([]) == 0.0)
	Nil
}

@test
sum_non_empty_test = () {
	assert(float.sum([1.0, 2.2, 3.3]) == 6.5)
	Nil
}

@test
product_empty_test = () {
	assert(float.product([]) == 1.0)
	Nil
}

@test
product_single_test = () {
	assert(float.product([4.0]) == 4.0)
	Nil
}

@test
product_multiple_test = () {
	assert(float.product([2.5, 3.2, 4.2]) == 33.6)
	Nil
}

@test
modulo_by_zero_test = () {
	assert(float.modulo(13.3, by: 0.0) == Error(Nil))
	Nil
}

@test
modulo_positive_test = () {
	assert(
		float.modulo(13.3, by: 3.3)
			|> result.unwrap(or: 0.0)
			|> float.loosely_equals(with: 0.1, tolerating: 0.001)
	)
	Nil
}

@test
modulo_negative_dividend_test = () {
	assert(
		float.modulo(-.13.3, by: 3.3)
			|> result.unwrap(or: 0.0)
			|> float.loosely_equals(with: 3.2, tolerating: 0.001)
	)
	Nil
}

@test
modulo_negative_divisor_test = () {
	assert(
		float.modulo(13.3, by: -.3.3)
			|> result.unwrap(or: 0.0)
			|> float.loosely_equals(with: -.3.2, tolerating: 0.001)
	)
	Nil
}

@test
modulo_both_negative_test = () {
	assert(
		float.modulo(-.13.3, by: -.3.3)
			|> result.unwrap(or: 0.0)
			|> float.loosely_equals(with: -.0.1, tolerating: 0.001)
	)
	Nil
}

@test
divide_by_self_test = () {
	assert(float.divide(1.0, 1.0) == Ok(1.0))
	Nil
}

@test
divide_by_zero_test = () {
	assert(float.divide(1.0, 0.0) == Error(Nil))
	Nil
}

@test
divide_zero_by_one_test = () {
	assert(float.divide(0.0, by: 1.0) == Ok(0.0))
	Nil
}

@test
divide_one_by_zero_test = () {
	assert(float.divide(1.0, by: 0.0) == Error(Nil))
	Nil
}

@test
add_test = () {
	assert(float.add(1.0, 2.0) == 3.0)
	Nil
}

@test
add_larger_test = () {
	assert(float.add(3.0, 2.0) == 5.0)
	Nil
}

@test
multiply_test = () {
	assert(float.multiply(2.0, 4.0) == 8.0)
	Nil
}

@test
multiply_larger_test = () {
	assert(float.multiply(3.0, 2.0) == 6.0)
	Nil
}

@test
subtract_test = () {
	assert(float.subtract(3.0, 1.0) == 2.0)
	Nil
}

@test
subtract_smaller_test = () {
	assert(float.subtract(3.0, 2.0) == 1.0)
	Nil
}

@test
subtract_negative_result_test = () {
	assert(float.subtract(2.0, 3.0) == -.1.0)
	Nil
}

@test
logarithm_one_test = () {
	assert(
		float.logarithm(1.0)
			|> result.unwrap(or: 1.0)
			|> float.loosely_equals(with: 0.0, tolerating: 0.001)
	)
	Nil
}

@test
logarithm_e_test = () {
	assert(
		float.logarithm(2.718281828459045)
			|> result.unwrap(or: 0.0)
			|> float.loosely_equals(with: 1.0, tolerating: 0.001)
	)
	Nil
}

@test
logarithm_ten_test = () {
	assert(
		float.logarithm(10.0)
			|> result.unwrap(or: 0.0)
			|> float.loosely_equals(with: 2.302585092994046, tolerating: 0.001)
	)
	Nil
}

@test
logarithm_hundred_test = () {
	assert(
		float.logarithm(100.0)
			|> result.unwrap(or: 0.0)
			|> float.loosely_equals(with: 4.605170185988092, tolerating: 0.001)
	)
	Nil
}

@test
logarithm_half_test = () {
	assert(
		float.logarithm(0.5)
			|> result.unwrap(or: 0.0)
			|> float.loosely_equals(
				with:       -.0.6931471805599453
				tolerating: 0.001
			)
	)
	Nil
}

@test
logarithm_tenth_test = () {
	assert(
		float.logarithm(0.1)
			|> result.unwrap(or: 0.0)
			|> float.loosely_equals(
				with:       -.2.3025850929940455
				tolerating: 0.001
			)
	)
	Nil
}

@test
logarithm_zero_error_test = () {
	assert(float.logarithm(0.0) == Error(Nil))
	Nil
}

@test
logarithm_negative_one_error_test = () {
	assert(float.logarithm(-.1.0) == Error(Nil))
	Nil
}

@test
logarithm_negative_hundred_error_test = () {
	assert(float.logarithm(-.100.0) == Error(Nil))
	Nil
}

@test
logarithm_negative_tenth_error_test = () {
	assert(float.logarithm(-.0.1) == Error(Nil))
	Nil
}

@test
exponential_zero_test = () {
	assert(
		float.loosely_equals(
			result.unwrap(float.exponential(0.0), 0.0)
			with:       1.0
			tolerating: 0.001
		)
	)
	Nil
}

@test
exponential_one_test = () {
	assert(
		float.loosely_equals(
			result.unwrap(float.exponential(1.0), 0.0)
			with:       2.718281828459045
			tolerating: 0.001
		)
	)
	Nil
}

@test
exponential_two_test = () {
	assert(
		float.loosely_equals(
			result.unwrap(float.exponential(2.0), 0.0)
			with:       7.38905609893065
			tolerating: 0.001
		)
	)
	Nil
}

@test
exponential_negative_one_test = () {
	assert(
		float.loosely_equals(
			result.unwrap(float.exponential(-.1.0), 0.0)
			with:       0.36787944117144233
			tolerating: 0.001
		)
	)
	Nil
}

@test
exponential_five_test = () {
	assert(
		float.loosely_equals(
			result.unwrap(float.exponential(5.0), 0.0)
			with:       148.4131591025766
			tolerating: 0.001
		)
	)
	Nil
}

@test
exponential_negative_five_test = () {
	assert(
		float.loosely_equals(
			result.unwrap(float.exponential(-.5.0), 0.0)
			with:       0.006737946999085467
			tolerating: 0.001
		)
	)
	Nil
}

@test
exponential_small_test = () {
	assert(
		float.loosely_equals(
			result.unwrap(float.exponential(0.000001), 0.0)
			with:       1.0000010000005
			tolerating: 0.001
		)
	)
	Nil
}

@test
exponential_large_negative_test = () {
	assert(
		float.loosely_equals(
			result.unwrap(float.exponential(-.100.0), 0.0)
			with:       3.720075976020836e-44
			tolerating: 0.001
		)
	)
	Nil
}

// Tests for functions that are new in std, or that changed from the old
// library.

@test
parse_exact_rounding_test = () {
	// The expected bits are the results of an independent decimal parser
	// (CPython), with both midpoint ties, the normal/subnormal boundary, and
	// signed zero.
	cases = [
		("0.0", 0)
		("-0.0", 9_223_372_036_854_775_808)
		("+1.23", 4_608_218_246_714_312_622)
		("-1.23", 13_831_590_283_569_088_430)
		("0.1", 4_591_870_180_066_957_722)
		("1.234e10", 4_757_767_320_795_873_280)
		("1.234e-10", 4_458_833_837_745_914_792)
		("1.7976931348623157e308", 9_218_868_437_227_405_311)
		("2.2250738585072014e-308", 4_503_599_627_370_496)
		("2.225073858507201e-308", 4_503_599_627_370_495)
		("5.0e-324", 1)
		("2.4703282292062327e-324", 0)
		("2.4703282292062328e-324", 1)
		("-1.0e-400", 9_223_372_036_854_775_808)
		("9007199254740993.0", 4_845_873_199_050_653_696)
		(
			"1.00000000000000011102230246251565404236316680908203125"
			4_607_182_418_800_017_408
		)
		(
			"1.00000000000000033306690738754696212708950042724609375"
			4_607_182_418_800_017_410
		)
		("1.0E2", 4_636_737_291_354_636_288)
	]
	list.each(cases, (c) {
		(text, bits) = c
		Ok(value) = float.parse(text)
		assert(core.float_to_bits(value) == bits)
	})
}

@test
parse_invalid_forms_test = () {
	bad = [
		""
		"1"
		".1"
		"1."
		" 1.0"
		"1.0 "
		"1e1"
		"+"
		"-"
		"1.0e"
		"1.0e+"
		"1.0e-"
		"1.0_0"
		"NaN"
		"Infinity"
		"0x1.0p1"
		"1.7976931348623159e308"
		"1.0e99999999999999999999"
		"0.0e99999999999999999x"
	]
	list.each(bad, (text) { assert(float.parse(text) == Error(Nil)) })
	Ok(negative_zero) = float.parse("-0.0e99999999999999999")
	assert(float.to_string(negative_zero) == "-0.0")
}

@test
to_string_shortest_test = () {
	assert(float.to_string(0.0123) == "0.0123")
	assert(float.to_string(-.0.0123) == "-0.0123")
	assert(float.to_string(123.0) == "123.0")
	assert(float.to_string(3.0e26) == "3.0e26")
	assert(float.to_string(-.3.0e-26) == "-3.0e-26")
	assert(float.to_string(456.12e78) == "4.5612e80")
	assert(float.to_string(9_007_199_254_740_992.0) == "9.007199254740992e15")
	assert(float.to_string(5.0e-324) == "5.0e-324")
	assert(float.to_string(1.7976931348623157e308) == "1.7976931348623157e308")
}

@test
round_trip_test = () {
	// to_string then parse gives back the same bits, for many floats.
	go = (seed, count) {
		count |> {
			0 -> Nil
			_ -> {
				next = {
					seed * 6_364_136_223_846_793_005 + 1_442_695_040_888_963_407
				} %
					18_446_744_073_709_551_616
				magnitude = next % 9_218_868_437_227_405_312
				sign = next /
					9_223_372_036_854_775_808 *
					9_223_372_036_854_775_808
				input = magnitude + sign
				value = core.float_from_bits(input)
				Ok(decoded) = float.parse(float.to_string(value))
				assert(core.float_to_bits(decoded) == input)
				go(next, count - 1)
			}
		}
	}
	go(177_293_419, 200)
}

@test
power_int_test = () {
	assert(float.power(2.0, of: 10) == Ok(1024.0))
	assert(float.power(2.0, of: -1) == Ok(0.5))
	assert(float.power(-.2.0, of: 3) == Ok(-.8.0))
	assert(float.power(-.2.0, of: -1) == Ok(-.0.5))
	assert(float.power(0.0, of: 0) == Ok(1.0))
	assert(float.power(0.0, of: 3) == Ok(0.0))
	assert(float.power(0.0, of: -1) == Error(Nil))
	assert(float.power(10.0, of: 400) == Error(Nil))
	assert(float.power(10.0, of: -400) == Ok(0.0))
	assert(float.power(-.1.0, of: 1_000_000_000_000_000_000_001) == Ok(-.1.0))
	assert(float.power(1.1, of: 2) == Ok(1.2100000000000002))
	assert(float.power(10.0, of: 22) == Ok(1.0e22))
}

@test
real_power_test = () {
	assert(float.real_power(16.0, of: 0.5) == Ok(4.0))
	// fdlibm is within one ulp: the correctly rounded result is
	// 22.627416997969522, one ulp above.
	assert(float.real_power(8.0, of: 1.5) == Ok(22.62741699796952))
	assert(float.real_power(-.8.0, of: 0.5) == Error(Nil))
	assert(float.real_power(0.0, of: -.0.5) == Error(Nil))
	assert(float.real_power(2.0, of: 1024.0) == Error(Nil))
	assert(float.real_power(2.0, of: -.1074.0) == Ok(5.0e-324))
	assert(float.real_power(2.0, of: 0.0) == Ok(1.0))
}

@test
exponential_logarithm_test = () {
	assert(float.exponential(0.0) == Ok(1.0))
	assert(float.exponential(1.0) == Ok(2.718281828459045))
	assert(float.exponential(710.0) == Error(Nil))
	assert(float.exponential(-.1000.0) == Ok(0.0))
	assert(float.logarithm(1.0) == Ok(0.0))
	assert(float.logarithm(2.718281828459045) == Ok(1.0))
	assert(float.logarithm(5.0e-324) == Ok(-.744.4400719213812))
	assert(float.logarithm(-.1.0) == Error(Nil))
	Ok(x) = float.logarithm(10.0)
	assert(float.exponential(x) == Ok(10.000000000000002))
}

@test
round_more_test = () {
	assert(float.round(0.49999999999999994) == 0)
	assert(float.round(-.0.5) == -1)
	assert(float.round(2.5) == 3)
	assert(float.round(1.0e20) == 100000000000000000000)
	assert(float.truncate(-.2.7) == -2)
	assert(float.is_integer(2.0))
	assert(core.not(float.is_integer(2.5)))
}

@test
compare_zero_test = () {
	assert(float.compare(0.0, -.0.0) == Equal)
	assert(float.min(1.0, 2.0) == 1.0)
	assert(float.max(1.0, 2.0) == 2.0)
	assert(float.clamp(5.0, min: 1.0, max: 3.0) == 3.0)
}

@test
loosely_huge_test = () {
	// Far apart numbers must not stop the program by overflow.
	assert(
		core.not(
			float.loosely_equals(
				float.max_finite
				with:       -.float.max_finite
				tolerating: 1.0
			)
		)
	)
	assert(
		float.loosely_compare(
			float.max_finite
			with:       -.float.max_finite
			tolerating: 1.0
		) ==
			Greater
	)
}

@test
trigonometric_constants_and_signs_test = () {
	negative_zero = core.float_from_bits(0x8000000000000000)
	assert(core.float_to_bits(float.pi) == 0x400921fb54442d18)
	assert(core.float_to_bits(float.tau) == 0x401921fb54442d18)
	assert(float.sign_bit(negative_zero))
	assert(core.not(float.sign_bit(0.0)))
	assert(float.sign_bit(-.1.0))
	assert(core.not(float.sign_bit(1.0)))
	assert(float.sign_bit(-.float.max_finite))
	assert(core.not(float.sign_bit(float.min_positive)))
	assert(float.sign_bit(-.float.min_positive))
	assert(core.not(float.sign_bit(float.max_finite)))
	assert(
		core.float_to_bits(float.copy_sign(2.0, negative_zero)) ==
			0xc000000000000000
	)
	assert(core.float_to_bits(float.copy_sign(negative_zero, 1.0)) == 0)
	assert(float.copy_sign(1.0, -.1.0) == -.1.0)
	assert(float.copy_sign(-.1.0, 1.0) == 1.0)
	assert(float.copy_sign(2.0, 1.0) == 2.0)
	assert(float.copy_sign(-.2.0, -.1.0) == -.2.0)
	assert(float.copy_sign(float.min_positive, -.1.0) == -.float.min_positive)
	assert(float.copy_sign(float.max_finite, 1.0) == float.max_finite)
}

@test
asin_reference_0_test = () {
	Ok(actual) = float.asin(-.1.0)
	expected = -.1.5707963267948966
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_1_test = () {
	Ok(actual) = float.asin(-.0.99)
	expected = -.1.4292568534704693
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_2_test = () {
	Ok(actual) = float.asin(-.0.9)
	expected = -.1.1197695149986342
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_3_test = () {
	Ok(actual) = float.asin(-.0.75)
	expected = -.0.848062078981481
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_4_test = () {
	Ok(actual) = float.asin(-.0.5)
	expected = -.0.5235987755982989
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_5_test = () {
	Ok(actual) = float.asin(-.0.1)
	expected = -.0.1001674211615598
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_6_test = () {
	Ok(actual) = float.asin(0.0)
	expected = 0.0
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_7_test = () {
	Ok(actual) = float.asin(0.1)
	expected = 0.1001674211615598
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_8_test = () {
	Ok(actual) = float.asin(0.5)
	expected = 0.5235987755982989
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_9_test = () {
	Ok(actual) = float.asin(0.75)
	expected = 0.848062078981481
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_10_test = () {
	Ok(actual) = float.asin(0.9)
	expected = 1.1197695149986342
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_11_test = () {
	Ok(actual) = float.asin(0.99)
	expected = 1.4292568534704693
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
asin_reference_12_test = () {
	Ok(actual) = float.asin(1.0)
	expected = 1.5707963267948966
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_0_test = () {
	Ok(actual) = float.acos(-.1.0)
	expected = 3.141592653589793
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_1_test = () {
	Ok(actual) = float.acos(-.0.99)
	expected = 3.000053180265366
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_2_test = () {
	Ok(actual) = float.acos(-.0.9)
	expected = 2.6905658417935308
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_3_test = () {
	Ok(actual) = float.acos(-.0.75)
	expected = 2.4188584057763776
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_4_test = () {
	Ok(actual) = float.acos(-.0.5)
	expected = 2.0943951023931957
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_5_test = () {
	Ok(actual) = float.acos(-.0.1)
	expected = 1.6709637479564565
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_6_test = () {
	Ok(actual) = float.acos(0.0)
	expected = 1.5707963267948966
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_7_test = () {
	Ok(actual) = float.acos(0.1)
	expected = 1.4706289056333368
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_8_test = () {
	Ok(actual) = float.acos(0.5)
	expected = 1.0471975511965979
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_9_test = () {
	Ok(actual) = float.acos(0.75)
	expected = 0.7227342478134157
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_10_test = () {
	Ok(actual) = float.acos(0.9)
	expected = 0.45102681179626236
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_11_test = () {
	Ok(actual) = float.acos(0.99)
	expected = 0.1415394733244273
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
acos_reference_12_test = () {
	Ok(actual) = float.acos(1.0)
	expected = 0.0
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
inverse_trig_domain_test = () {
	assert(float.asin(1.0000000000000002) == Error(Nil))
	assert(float.acos(-.1.0000000000000002) == Error(Nil))
}

@test
atan_reference_0_test = () {
	actual = float.atan(-.100.0)
	expected = -.1.5607966601082315
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_1_test = () {
	actual = float.atan(-.10.0)
	expected = -.1.4711276743037347
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_2_test = () {
	actual = float.atan(-.3.0)
	expected = -.1.2490457723982544
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_3_test = () {
	actual = float.atan(-.1.0)
	expected = -.0.7853981633974483
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_4_test = () {
	actual = float.atan(-.0.75)
	expected = -.0.6435011087932844
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_5_test = () {
	actual = float.atan(-.0.5)
	expected = -.0.4636476090008061
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_6_test = () {
	actual = float.atan(-.0.1)
	expected = -.0.09966865249116204
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_7_test = () {
	actual = float.atan(0.0)
	expected = 0.0
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_8_test = () {
	actual = float.atan(0.1)
	expected = 0.09966865249116204
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_9_test = () {
	actual = float.atan(0.5)
	expected = 0.4636476090008061
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_10_test = () {
	actual = float.atan(0.75)
	expected = 0.6435011087932844
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_11_test = () {
	actual = float.atan(1.0)
	expected = 0.7853981633974483
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_12_test = () {
	actual = float.atan(3.0)
	expected = 1.2490457723982544
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_13_test = () {
	actual = float.atan(10.0)
	expected = 1.4711276743037347
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan_reference_14_test = () {
	actual = float.atan(100.0)
	expected = 1.5607966601082315
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_0_test = () {
	actual = float.radians(-.360.0)
	expected = -.6.283185307179586
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_1_test = () {
	actual = float.radians(-.180.0)
	expected = -.3.141592653589793
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_2_test = () {
	actual = float.radians(-.90.0)
	expected = -.1.5707963267948966
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_3_test = () {
	actual = float.radians(-.45.0)
	expected = -.0.7853981633974483
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_4_test = () {
	actual = float.radians(-.1.0)
	expected = -.0.017453292519943295
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_5_test = () {
	actual = float.radians(-.0.1)
	expected = -.0.0017453292519943296
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_6_test = () {
	actual = float.radians(0.0)
	expected = 0.0
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_7_test = () {
	actual = float.radians(0.1)
	expected = 0.0017453292519943296
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_8_test = () {
	actual = float.radians(1.0)
	expected = 0.017453292519943295
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_9_test = () {
	actual = float.radians(45.0)
	expected = 0.7853981633974483
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_10_test = () {
	actual = float.radians(90.0)
	expected = 1.5707963267948966
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_11_test = () {
	actual = float.radians(180.0)
	expected = 3.141592653589793
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
radians_reference_12_test = () {
	actual = float.radians(360.0)
	expected = 6.283185307179586
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_0_test = () {
	Ok(actual) = float.degrees(-.6.0)
	expected = -.343.77467707849394
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_1_test = () {
	Ok(actual) = float.degrees(-.3.0)
	expected = -.171.88733853924697
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_2_test = () {
	Ok(actual) = float.degrees(-.1.0)
	expected = -.57.29577951308232
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_3_test = () {
	Ok(actual) = float.degrees(-.0.75)
	expected = -.42.97183463481174
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_4_test = () {
	Ok(actual) = float.degrees(-.0.5)
	expected = -.28.64788975654116
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_5_test = () {
	Ok(actual) = float.degrees(-.0.1)
	expected = -.5.729577951308233
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_6_test = () {
	Ok(actual) = float.degrees(0.0)
	expected = 0.0
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_7_test = () {
	Ok(actual) = float.degrees(0.1)
	expected = 5.729577951308233
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_8_test = () {
	Ok(actual) = float.degrees(0.5)
	expected = 28.64788975654116
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_9_test = () {
	Ok(actual) = float.degrees(0.75)
	expected = 42.97183463481174
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_10_test = () {
	Ok(actual) = float.degrees(1.0)
	expected = 57.29577951308232
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_11_test = () {
	Ok(actual) = float.degrees(3.0)
	expected = 171.88733853924697
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
degrees_reference_12_test = () {
	Ok(actual) = float.degrees(6.0)
	expected = 343.77467707849394
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_0_test = () {
	actual = float.atan2(1.0, 1.0)
	expected = 0.7853981633974483
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_1_test = () {
	actual = float.atan2(-.1.0, 1.0)
	expected = -.0.7853981633974483
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_2_test = () {
	actual = float.atan2(1.0, -.1.0)
	expected = 2.356194490192345
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_3_test = () {
	actual = float.atan2(-.1.0, -.1.0)
	expected = -.2.356194490192345
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_4_test = () {
	actual = float.atan2(1.0, 2.0)
	expected = 0.4636476090008061
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_5_test = () {
	actual = float.atan2(2.0, 1.0)
	expected = 1.1071487177940904
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_6_test = () {
	actual = float.atan2(-.2.0, 1.0)
	expected = -.1.1071487177940904
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_7_test = () {
	actual = float.atan2(2.0, -.1.0)
	expected = 2.0344439357957027
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_8_test = () {
	actual = float.atan2(0.1, 1.0)
	expected = 0.09966865249116204
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_9_test = () {
	actual = float.atan2(1.0, 0.1)
	expected = 1.4711276743037345
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_10_test = () {
	actual = float.atan2(3.0, 7.0)
	expected = 0.40489178628508343
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_11_test = () {
	actual = float.atan2(7.0, 3.0)
	expected = 1.1659045405098132
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
atan2_reference_12_test = () {
	actual = float.atan2(1.0e300, 1.0e-300)
	expected = 1.5707963267948966
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_0_test = () {
	Ok(actual) = float.hypot(0.0, 0.0)
	expected = 0.0
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_1_test = () {
	Ok(actual) = float.hypot(3.0, 4.0)
	expected = 5.0
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_2_test = () {
	Ok(actual) = float.hypot(5.0, 12.0)
	expected = 13.0
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_3_test = () {
	Ok(actual) = float.hypot(1.0, 1.0)
	expected = 1.4142135623730951
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_4_test = () {
	Ok(actual) = float.hypot(-.1.0, 1.0)
	expected = 1.4142135623730951
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_5_test = () {
	Ok(actual) = float.hypot(1.0, -.1.0)
	expected = 1.4142135623730951
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_6_test = () {
	Ok(actual) = float.hypot(0.1, 0.2)
	expected = 0.223606797749979
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_7_test = () {
	Ok(actual) = float.hypot(0.3, 0.4)
	expected = 0.5
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_8_test = () {
	Ok(actual) = float.hypot(1.0, 2.0)
	expected = 2.23606797749979
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_9_test = () {
	Ok(actual) = float.hypot(1.0, 100.0)
	expected = 100.00499987500625
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_10_test = () {
	Ok(actual) = float.hypot(5.0, 7.0)
	expected = 8.602325267042627
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_11_test = () {
	Ok(actual) = float.hypot(10.0, 1.0)
	expected = 10.04987562112089
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
hypot_reference_12_test = () {
	Ok(actual) = float.hypot(1.0e-100, 2.0e-100)
	expected = 2.2360679774997898e-100
	a = core.float_to_bits(actual)
	b = core.float_to_bits(expected)
	assert(a >= b - 1 && a <= b + 1)
}

@test
inverse_and_norm_special_values_test = () {
	negative_zero = core.float_from_bits(9223372036854775808)
	Ok(a) = float.asin(negative_zero)
	assert(core.float_to_bits(a) == core.float_to_bits(negative_zero))
	assert(
		core.float_to_bits(float.atan(negative_zero)) ==
			core.float_to_bits(negative_zero)
	)
	assert(float.acos(1.0) == Ok(0.0))
	assert(float.acos(-.1.0) == Ok(float.pi))
	assert(float.hypot(float.max_finite, float.max_finite) == Error(Nil))
	assert(float.hypot(negative_zero, 0.0) == Ok(0.0))
	assert(float.atan2(0.0, negative_zero) == float.pi)
	assert(float.atan2(negative_zero, negative_zero) == -.float.pi)
	assert(
		core.float_to_bits(float.radians(negative_zero)) ==
			core.float_to_bits(negative_zero)
	)
	Ok(d) = float.degrees(negative_zero)
	assert(core.float_to_bits(d) == core.float_to_bits(negative_zero))
	assert(float.degrees(float.max_finite) == Error(Nil))
}

@test
hypot_symmetry_property = () {
	Ok(gen) = property.int(-100000, 100000)
	property.test(
		property.pair(gen, gen)
		property.Config(123, 100, 30, 0, 100)
		(pair) {
			(x, y) = pair
			a = core.to_float(x) /. 100.0
			b = core.to_float(y) /. 100.0
			assert(float.hypot(a, b) == float.hypot(b, a))
		}
		core.inspect
	)
}

@test
atan2_quadrants_property = () {
	Ok(gen) = property.int(1, 100000)
	property.test(
		property.pair(gen, gen)
		property.Config(456, 100, 30, 0, 100)
		(pair) {
			(x, y) = pair
			a = core.to_float(x)
			b = core.to_float(y)
			assert(float.atan2(b, a) >. 0.0)
			assert(float.atan2(b, -.a) >. 1.5707963267948966)
			assert(float.atan2(-.b, a) <. 0.0)
			assert(float.atan2(-.b, -.a) <. -.1.5707963267948966)
		}
		core.inspect
	)
}

@test
sign_bit_reference_0_test = () {
	assert(float.sign_bit(0.0) == False)
	Nil
}

@test
sign_bit_reference_1_test = () {
	assert(float.sign_bit(-.0.0) == True)
	Nil
}

@test
sign_bit_reference_2_test = () {
	assert(float.sign_bit(1.0) == False)
	Nil
}

@test
sign_bit_reference_3_test = () {
	assert(float.sign_bit(-.1.0) == True)
	Nil
}

@test
sign_bit_reference_4_test = () {
	assert(float.sign_bit(1.0e-300) == False)
	Nil
}

@test
sign_bit_reference_5_test = () {
	assert(float.sign_bit(-.1.0e-300) == True)
	Nil
}

@test
sign_bit_reference_6_test = () {
	assert(float.sign_bit(1.0e300) == False)
	Nil
}

@test
sign_bit_reference_7_test = () {
	assert(float.sign_bit(-.1.0e300) == True)
	Nil
}

@test
sign_bit_reference_8_test = () {
	assert(float.sign_bit(float.min_positive) == False)
	Nil
}

@test
sign_bit_reference_9_test = () {
	assert(float.sign_bit(-.float.min_positive) == True)
	Nil
}

@test
sign_bit_reference_10_test = () {
	assert(float.sign_bit(float.max_finite) == False)
	Nil
}

@test
sign_bit_reference_11_test = () {
	assert(float.sign_bit(-.float.max_finite) == True)
	Nil
}

@test
copy_sign_reference_0_test = () {
	assert(core.float_to_bits(float.copy_sign(0.0, 1.0)) == 0)
	Nil
}

@test
copy_sign_reference_1_test = () {
	assert(
		core.float_to_bits(float.copy_sign(0.0, -.1.0)) == 9223372036854775808
	)
	Nil
}

@test
copy_sign_reference_2_test = () {
	assert(core.float_to_bits(float.copy_sign(-.0.0, 1.0)) == 0)
	Nil
}

@test
copy_sign_reference_3_test = () {
	assert(
		core.float_to_bits(float.copy_sign(-.0.0, -.1.0)) == 9223372036854775808
	)
	Nil
}

@test
copy_sign_reference_4_test = () {
	assert(core.float_to_bits(float.copy_sign(1.0, 1.0)) == 4607182418800017408)
	Nil
}

@test
copy_sign_reference_5_test = () {
	assert(
		core.float_to_bits(float.copy_sign(1.0, -.1.0)) == 13830554455654793216
	)
	Nil
}

@test
copy_sign_reference_6_test = () {
	assert(
		core.float_to_bits(float.copy_sign(-.1.0, 1.0)) == 4607182418800017408
	)
	Nil
}

@test
copy_sign_reference_7_test = () {
	assert(
		core.float_to_bits(float.copy_sign(-.1.0, -.1.0)) ==
			13830554455654793216
	)
	Nil
}

@test
copy_sign_reference_8_test = () {
	assert(core.float_to_bits(float.copy_sign(float.min_positive, 1.0)) == 1)
	Nil
}

@test
copy_sign_reference_9_test = () {
	assert(
		core.float_to_bits(float.copy_sign(float.min_positive, -.1.0)) ==
			9223372036854775809
	)
	Nil
}

@test
copy_sign_reference_10_test = () {
	assert(
		core.float_to_bits(float.copy_sign(float.max_finite, 1.0)) ==
			9218868437227405311
	)
	Nil
}

@test
copy_sign_reference_11_test = () {
	assert(
		core.float_to_bits(float.copy_sign(float.max_finite, -.1.0)) ==
			18442240474082181119
	)
	Nil
}
