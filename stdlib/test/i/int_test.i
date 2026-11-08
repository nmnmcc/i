import i/int
import i/list
import i/order

pub fn absolute_value_positive_test() -> Nil {
  assert int.absolute_value(123) == 123
  Nil
}

pub fn absolute_value_negative_test() -> Nil {
  assert int.absolute_value(-123) == 123
  Nil
}

pub fn clamp_within_range_test() -> Nil {
  assert int.clamp(40, min: 30, max: 50) == 40
  Nil
}

pub fn clamp_below_min_test() -> Nil {
  assert int.clamp(20, min: 30, max: 50) == 30
  Nil
}

pub fn clamp_above_max_test() -> Nil {
  assert int.clamp(60, min: 30, max: 50) == 50
  Nil
}

pub fn clamp_inverted_range_above_test() -> Nil {
  assert int.clamp(100, min: 50, max: 30) == 50
  Nil
}

pub fn clamp_inverted_range_within_test() -> Nil {
  assert int.clamp(40, min: 50, max: 30) == 40
  Nil
}

pub fn to_string_positive_test() -> Nil {
  assert int.to_string(123) == "123"
  Nil
}

pub fn to_string_negative_test() -> Nil {
  assert int.to_string(-123) == "-123"
  Nil
}

pub fn to_string_positive_2_test() -> Nil {
  assert int.to_string(123) == "123"
  Nil
}

pub fn parse_positive_test() -> Nil {
  assert int.parse("123") == Ok(123)
  Nil
}

pub fn parse_negative_test() -> Nil {
  assert int.parse("-123") == Ok(-123)
  Nil
}

pub fn parse_leading_zero_test() -> Nil {
  assert int.parse("0123") == Ok(123)
  Nil
}

pub fn parse_empty_test() -> Nil {
  assert int.parse("") == Error(Nil)
  Nil
}

pub fn parse_invalid_test() -> Nil {
  assert int.parse("what") == Error(Nil)
  Nil
}

pub fn parse_float_test() -> Nil {
  assert int.parse("1.23") == Error(Nil)
  Nil
}

pub fn base_parse_hex_test() -> Nil {
  assert int.base_parse("100", 16) == Ok(256)
  Nil
}

pub fn base_parse_hex_negative_test() -> Nil {
  assert int.base_parse("-100", 16) == Ok(-256)
  Nil
}

pub fn base_parse_base_1_test() -> Nil {
  assert int.base_parse("100", 1) == Error(Nil)
  Nil
}

pub fn base_parse_base_37_test() -> Nil {
  assert int.base_parse("100", 37) == Error(Nil)
  Nil
}

pub fn base_parse_invalid_hex_test() -> Nil {
  assert int.base_parse("AG", 16) == Error(Nil)
  Nil
}

pub fn to_base_string_hex_test() -> Nil {
  assert int.to_base_string(100, 16) == Ok("64")
  Nil
}

pub fn to_base_string_hex_negative_test() -> Nil {
  assert int.to_base_string(-100, 16) == Ok("-64")
  Nil
}

pub fn to_base_string_base_1_test() -> Nil {
  assert int.to_base_string(100, 1) == Error(Nil)
  Nil
}

pub fn to_base_string_base_37_test() -> Nil {
  assert int.to_base_string(100, 37) == Error(Nil)
  Nil
}

pub fn to_base2_positive_test() -> Nil {
  assert int.to_base2(100) == "1100100"
  Nil
}

pub fn to_base2_negative_test() -> Nil {
  assert int.to_base2(-100) == "-1100100"
  Nil
}

pub fn to_base8_positive_test() -> Nil {
  assert int.to_base8(100) == "144"
  Nil
}

pub fn to_base8_negative_test() -> Nil {
  assert int.to_base8(-100) == "-144"
  Nil
}

pub fn to_base16_positive_test() -> Nil {
  assert int.to_base16(100) == "64"
  Nil
}

pub fn to_base16_negative_test() -> Nil {
  assert int.to_base16(-100) == "-64"
  Nil
}

pub fn to_base16_large_test() -> Nil {
  assert int.to_base16(43_981) == "ABCD"
  Nil
}

pub fn to_base16_large_negative_test() -> Nil {
  assert int.to_base16(-43_981) == "-ABCD"
  Nil
}

pub fn to_base36_positive_test() -> Nil {
  assert int.to_base36(100) == "2S"
  Nil
}

pub fn to_base36_negative_test() -> Nil {
  assert int.to_base36(-100) == "-2S"
  Nil
}

pub fn to_float_one_test() -> Nil {
  assert int.to_float(1) == 1.0
  Nil
}

pub fn to_float_five_test() -> Nil {
  assert int.to_float(5) == 5.0
  Nil
}

pub fn to_float_zero_test() -> Nil {
  assert int.to_float(0) == 0.0
  Nil
}

pub fn to_float_negative_test() -> Nil {
  assert int.to_float(-5) == -5.0
  Nil
}

pub fn compare_equal_zero_test() -> Nil {
  assert int.compare(0, 0) == order.Eq
  Nil
}

pub fn compare_equal_one_test() -> Nil {
  assert int.compare(1, 1) == order.Eq
  Nil
}

pub fn compare_lt_test() -> Nil {
  assert int.compare(0, 1) == order.Lt
  Nil
}

pub fn compare_negative_lt_test() -> Nil {
  assert int.compare(-2, -1) == order.Lt
  Nil
}

pub fn compare_gt_test() -> Nil {
  assert int.compare(2, 1) == order.Gt
  Nil
}

pub fn compare_negative_gt_test() -> Nil {
  assert int.compare(-1, -2) == order.Gt
  Nil
}

pub fn min_equal_test() -> Nil {
  assert int.min(0, 0) == 0
  Nil
}

pub fn min_first_smaller_test() -> Nil {
  assert int.min(0, 1) == 0
  Nil
}

pub fn min_second_smaller_test() -> Nil {
  assert int.min(1, 0) == 0
  Nil
}

pub fn min_negative_smaller_test() -> Nil {
  assert int.min(-1, 2) == -1
  Nil
}

pub fn min_both_mixed_test() -> Nil {
  assert int.min(2, -2) == -2
  Nil
}

pub fn min_both_negative_test() -> Nil {
  assert int.min(-1, -1) == -1
  Nil
}

pub fn max_equal_test() -> Nil {
  assert int.max(0, 0) == 0
  Nil
}

pub fn max_second_larger_test() -> Nil {
  assert int.max(0, 1) == 1
  Nil
}

pub fn max_first_larger_test() -> Nil {
  assert int.max(1, 0) == 1
  Nil
}

pub fn max_positive_larger_test() -> Nil {
  assert int.max(-1, 2) == 2
  Nil
}

pub fn max_mixed_test() -> Nil {
  assert int.max(2, -2) == 2
  Nil
}

pub fn max_both_negative_test() -> Nil {
  assert int.max(-1, -1) == -1
  Nil
}

pub fn is_even_zero_test() -> Nil {
  assert int.is_even(0)
  Nil
}

pub fn is_even_two_test() -> Nil {
  assert int.is_even(2)
  Nil
}

pub fn is_even_negative_two_test() -> Nil {
  assert int.is_even(-2)
  Nil
}

pub fn is_even_large_test() -> Nil {
  assert int.is_even(10_006)
  Nil
}

pub fn is_even_one_false_test() -> Nil {
  assert !int.is_even(1)
  Nil
}

pub fn is_even_negative_three_false_test() -> Nil {
  assert !int.is_even(-3)
  Nil
}

pub fn is_even_large_odd_false_test() -> Nil {
  assert !int.is_even(10_005)
  Nil
}

pub fn is_odd_zero_false_test() -> Nil {
  assert !int.is_odd(0)
  Nil
}

pub fn is_odd_two_false_test() -> Nil {
  assert !int.is_odd(2)
  Nil
}

pub fn is_odd_negative_two_false_test() -> Nil {
  assert !int.is_odd(-2)
  Nil
}

pub fn is_odd_large_even_false_test() -> Nil {
  assert !int.is_odd(10_006)
  Nil
}

pub fn is_odd_one_test() -> Nil {
  assert int.is_odd(1)
  Nil
}

pub fn is_odd_negative_three_test() -> Nil {
  assert int.is_odd(-3)
  Nil
}

pub fn is_odd_large_test() -> Nil {
  assert int.is_odd(10_005)
  Nil
}

pub fn power_two_squared_test() -> Nil {
  assert int.power(2, 2.0) == Ok(4.0)
  Nil
}

pub fn power_negative_cubed_test() -> Nil {
  assert int.power(-5, 3.0) == Ok(-125.0)
  Nil
}

pub fn power_zero_exponent_test() -> Nil {
  assert int.power(10, 0.0) == Ok(1.0)
  Nil
}

pub fn power_fractional_exponent_test() -> Nil {
  assert int.power(16, 0.5) == Ok(4.0)
  Nil
}

pub fn power_negative_exponent_test() -> Nil {
  assert int.power(2, -1.0) == Ok(0.5)
  Nil
}

pub fn power_negative_base_fractional_exponent_error_test() -> Nil {
  // int.power(-1, 0.5) is equivalent to int.square_root(-1) and should
  // return an error as an imaginary number would otherwise have to be
  // returned
  assert int.power(-1, 0.5) == Error(Nil)
  Nil
}

pub fn power_negative_base_fractional_exponent_error_2_test() -> Nil {
  // Check another case with a negative base and fractional exponent
  assert int.power(-1, 1.5) == Error(Nil)
  Nil
}

pub fn power_zero_base_negative_exponent_error_test() -> Nil {
  // float.power(0, -1) is equivalent to 1 / 0 and is expected
  // to be an error
  assert int.power(0, -1.0) == Error(Nil)
  Nil
}

pub fn power_negative_base_negative_exponent_test() -> Nil {
  // Check that a negative base and exponent is fine as long as the
  // exponent is not fractional
  assert int.power(-2, -1.0) == Ok(-0.5)
  Nil
}

pub fn square_root_four_test() -> Nil {
  assert int.square_root(4) == Ok(2.0)
  Nil
}

pub fn square_root_sixteen_test() -> Nil {
  assert int.square_root(16) == Ok(4.0)
  Nil
}

pub fn square_root_zero_test() -> Nil {
  assert int.square_root(0) == Ok(0.0)
  Nil
}

pub fn square_root_negative_error_test() -> Nil {
  assert int.square_root(-4) == Error(Nil)
  Nil
}

pub fn negate_negative_test() -> Nil {
  assert int.negate(-1) == 1
  Nil
}

pub fn negate_positive_test() -> Nil {
  assert int.negate(2) == -2
  Nil
}

pub fn negate_zero_test() -> Nil {
  assert int.negate(0) == 0
  Nil
}

pub fn sum_empty_test() -> Nil {
  assert int.sum([]) == 0
  Nil
}

pub fn sum_non_empty_test() -> Nil {
  assert int.sum([1, 2, 3]) == 6
  Nil
}

pub fn product_empty_test() -> Nil {
  assert int.product([]) == 1
  Nil
}

pub fn product_single_test() -> Nil {
  assert int.product([4]) == 4
  Nil
}

pub fn product_multiple_test() -> Nil {
  assert int.product([1, 2, 3]) == 6
  Nil
}

pub fn random_test() -> Nil {
  use _, _ <- int.range(from: 0, to: 101, with: Nil)

  assert int.random(0) == 0

  assert int.random(1) == 0

  assert int.random(-1) == -1

  assert list.contains([0, 1], int.random(2))

  assert list.contains([0, 1, 2], int.random(3))

  Nil
  Nil
}

pub fn divide_by_self_test() -> Nil {
  assert int.divide(1, 1) == Ok(1)
  Nil
}

pub fn divide_by_zero_test() -> Nil {
  assert int.divide(1, 0) == Error(Nil)
  Nil
}

pub fn divide_zero_by_one_test() -> Nil {
  assert int.divide(0, by: 1) == Ok(0)
  Nil
}

pub fn divide_one_by_zero_test() -> Nil {
  assert int.divide(1, by: 0) == Error(Nil)
  Nil
}

pub fn divide_truncates_test() -> Nil {
  assert int.divide(5, by: 2) == Ok(2)
  Nil
}

pub fn divide_negative_test() -> Nil {
  assert int.divide(-99, by: 2) == Ok(-49)
  Nil
}

pub fn remainder_basic_test() -> Nil {
  assert int.remainder(3, 2) == Ok(1)
  Nil
}

pub fn remainder_by_zero_test() -> Nil {
  assert int.remainder(1, 0) == Error(Nil)
  Nil
}

pub fn remainder_negative_divisor_test() -> Nil {
  assert int.remainder(10, -1) == Ok(0)
  Nil
}

pub fn remainder_positive_test() -> Nil {
  assert int.remainder(13, by: 3) == Ok(1)
  Nil
}

pub fn remainder_negative_dividend_test() -> Nil {
  assert int.remainder(-13, by: 3) == Ok(-1)
  Nil
}

pub fn remainder_positive_by_negative_test() -> Nil {
  assert int.remainder(13, by: -3) == Ok(1)
  Nil
}

pub fn remainder_both_negative_test() -> Nil {
  assert int.remainder(-13, by: -3) == Ok(-1)
  Nil
}

pub fn modulo_basic_test() -> Nil {
  assert int.modulo(3, 2) == Ok(1)
  Nil
}

pub fn modulo_by_zero_test() -> Nil {
  assert int.modulo(1, 0) == Error(Nil)
  Nil
}

pub fn modulo_negative_divisor_test() -> Nil {
  assert int.modulo(10, -1) == Ok(0)
  Nil
}

pub fn modulo_positive_test() -> Nil {
  assert int.modulo(13, by: 3) == Ok(1)
  Nil
}

pub fn modulo_negative_dividend_test() -> Nil {
  assert int.modulo(-13, by: 3) == Ok(2)
  Nil
}

pub fn modulo_positive_by_negative_test() -> Nil {
  assert int.modulo(13, by: -3) == Ok(-2)
  Nil
}

pub fn modulo_both_negative_test() -> Nil {
  assert int.modulo(-13, by: -3) == Ok(-1)
  Nil
}

pub fn floor_divide_by_self_test() -> Nil {
  assert int.floor_divide(1, 1) == Ok(1)
  Nil
}

pub fn floor_divide_by_zero_test() -> Nil {
  assert int.floor_divide(1, 0) == Error(Nil)
  Nil
}

pub fn floor_divide_zero_by_one_test() -> Nil {
  assert int.floor_divide(0, by: 1) == Ok(0)
  Nil
}

pub fn floor_divide_one_by_zero_test() -> Nil {
  assert int.floor_divide(1, by: 0) == Error(Nil)
  Nil
}

pub fn floor_divide_truncates_test() -> Nil {
  assert int.floor_divide(5, by: 2) == Ok(2)
  Nil
}

pub fn floor_divide_negative_divisor_test() -> Nil {
  assert int.floor_divide(6, by: -4) == Ok(-2)
  Nil
}

pub fn floor_divide_negative_dividend_test() -> Nil {
  assert int.floor_divide(-99, by: 2) == Ok(-50)
  Nil
}

pub fn floor_divide_negative_small_test() -> Nil {
  assert int.floor_divide(-1, by: 2) == Ok(-1)
  Nil
}

pub fn add_test() -> Nil {
  assert int.add(1, 2) == 3
  Nil
}

pub fn add_larger_test() -> Nil {
  assert int.add(3, 2) == 5
  Nil
}

pub fn multiply_test() -> Nil {
  assert int.multiply(2, 4) == 8
  Nil
}

pub fn multiply_larger_test() -> Nil {
  assert int.multiply(3, 2) == 6
  Nil
}

pub fn subtract_test() -> Nil {
  assert int.subtract(3, 1) == 2
  Nil
}

pub fn subtract_smaller_test() -> Nil {
  assert int.subtract(3, 2) == 1
  Nil
}

pub fn subtract_negative_result_test() -> Nil {
  assert int.subtract(2, 3) == -1
  Nil
}

pub fn bitwise_and_test() -> Nil {
  assert int.bitwise_and(9, 3) == 1
  Nil
}

pub fn bitwise_and_32bit_test() -> Nil {
  // To check compatibility with JavaScript, try a 32 bit unsigned integer
  // (signed integers are in the range -2147483648 to +2147483647, while
  //  32 bit unsigned integers are in the range 0 to +4294967295).
  assert int.bitwise_and(2_147_483_648, 2_147_483_648) == 2_147_483_648
  Nil
}

pub fn bitwise_not_test() -> Nil {
  assert int.bitwise_not(2) == -3
  Nil
}

pub fn bitwise_not_32bit_test() -> Nil {
  // To check compatibility with JavaScript, try a 32 bit unsigned integer.
  assert int.bitwise_not(2_147_483_648) == -2_147_483_649
  Nil
}

pub fn bitwise_or_test() -> Nil {
  assert int.bitwise_or(9, 3) == 11
  Nil
}

pub fn bitwise_or_32bit_test() -> Nil {
  // To check compatibility with JavaScript, try a 32 bit unsigned integer.
  assert int.bitwise_or(1, 2_147_483_648) == 2_147_483_649
  Nil
}

pub fn bitwise_exclusive_or_test() -> Nil {
  assert int.bitwise_exclusive_or(9, 3) == 10
  Nil
}

pub fn bitwise_exclusive_or_32bit_test() -> Nil {
  // To check compatibility with JavaScript, try a 32 bit unsigned integer.
  assert int.bitwise_exclusive_or(0, 2_147_483_648) == 2_147_483_648
  Nil
}

pub fn bitwise_shift_left_positive_test() -> Nil {
  assert int.bitwise_shift_left(1, 2) == 4
  Nil
}

pub fn bitwise_shift_left_negative_shift_test() -> Nil {
  assert int.bitwise_shift_left(1, -2) == 0
  Nil
}

pub fn bitwise_shift_left_negative_value_test() -> Nil {
  assert int.bitwise_shift_left(-1, 2) == -4
  Nil
}

pub fn bitwise_shift_left_both_negative_test() -> Nil {
  assert int.bitwise_shift_left(-1, -2) == -1
  Nil
}

pub fn bitwise_shift_right_positive_test() -> Nil {
  assert int.bitwise_shift_right(1, 2) == 0
  Nil
}

pub fn bitwise_shift_right_negative_shift_test() -> Nil {
  assert int.bitwise_shift_right(1, -2) == 4
  Nil
}

pub fn bitwise_shift_right_negative_value_test() -> Nil {
  assert int.bitwise_shift_right(-1, 2) == -1
  Nil
}

pub fn bitwise_shift_right_both_negative_test() -> Nil {
  assert int.bitwise_shift_right(-1, -2) == -4
  Nil
}

pub fn bitwise_and_zero_test() -> Nil {
  assert int.bitwise_and(0, 0) == 0
  Nil
}

pub fn bitwise_and_identity_test() -> Nil {
  assert int.bitwise_and(0xFF, 0xFF) == 0xFF
  Nil
}

pub fn bitwise_and_clear_test() -> Nil {
  assert int.bitwise_and(0xFF, 0) == 0
  Nil
}

pub fn bitwise_and_large_numbers_test() -> Nil {
  // Test with numbers larger than 32 bits
  assert int.bitwise_and(0x123456789, 0xFFFF) == 0x6789
  Nil
}

pub fn bitwise_or_zero_test() -> Nil {
  assert int.bitwise_or(0, 0) == 0
  Nil
}

pub fn bitwise_or_identity_test() -> Nil {
  assert int.bitwise_or(0xFF, 0) == 0xFF
  Nil
}

pub fn bitwise_or_combine_test() -> Nil {
  assert int.bitwise_or(0xF0, 0x0F) == 0xFF
  Nil
}

pub fn bitwise_or_large_numbers_test() -> Nil {
  // Test with numbers larger than 32 bits
  assert int.bitwise_or(0x100000000, 0xFF) == 0x1000000FF
  Nil
}

pub fn bitwise_exclusive_or_zero_test() -> Nil {
  assert int.bitwise_exclusive_or(0, 0) == 0
  Nil
}

pub fn bitwise_exclusive_or_identity_test() -> Nil {
  assert int.bitwise_exclusive_or(0xFF, 0) == 0xFF
  Nil
}

pub fn bitwise_exclusive_or_invert_test() -> Nil {
  assert int.bitwise_exclusive_or(0xF0, 0x0F) == 0xFF
  Nil
}

pub fn bitwise_exclusive_or_self_test() -> Nil {
  assert int.bitwise_exclusive_or(0xAB, 0xAB) == 0
  Nil
}

pub fn bitwise_not_zero_test() -> Nil {
  assert int.bitwise_not(0) == -1
  Nil
}

pub fn bitwise_not_negative_one_test() -> Nil {
  assert int.bitwise_not(-1) == 0
  Nil
}

pub fn bitwise_not_large_number_test() -> Nil {
  // Test with number larger than 32 bits
  assert int.bitwise_not(0x123456789) == -4_886_718_346
  Nil
}

pub fn bitwise_shift_left_zero_shift_test() -> Nil {
  assert int.bitwise_shift_left(0xFF, 0) == 0xFF
  Nil
}

pub fn bitwise_shift_left_large_shift_test() -> Nil {
  assert int.bitwise_shift_left(1, 32) == 4_294_967_296
  Nil
}

pub fn bitwise_shift_left_large_number_test() -> Nil {
  assert int.bitwise_shift_left(0x1000, 4) == 0x10000
  Nil
}

pub fn bitwise_shift_right_zero_shift_test() -> Nil {
  assert int.bitwise_shift_right(0xFF, 0) == 0xFF
  Nil
}

pub fn bitwise_shift_right_large_shift_test() -> Nil {
  assert int.bitwise_shift_right(0xFFFFFFFF, 16) == 0xFFFF
  Nil
}

pub fn bitwise_shift_right_preserves_sign_test() -> Nil {
  assert int.bitwise_shift_right(-1, 16) == -1
  Nil
}

pub fn bitwise_shift_right_large_number_test() -> Nil {
  assert int.bitwise_shift_right(0x123456789, 16) == 0x12345
  Nil
}

pub fn bitwise_operations_with_max_safe_integer_test() -> Nil {
  // Test with maximum safe integer
  assert int.bitwise_and(9_007_199_254_740_991, 0xFFFF) == 65_535
  Nil
}

pub fn bitwise_operations_with_boundary_values_test() -> Nil {
  // Test around 32-bit boundaries
  assert int.bitwise_or(2_147_483_647, 1) == 2_147_483_647
  // MAX_INT32 | 1 = MAX_INT32
  assert int.bitwise_and(2_147_483_648, 4_294_967_295) == 2_147_483_648
  // 2^31 & 0xFFFFFFFF = 2^31
  Nil
}

pub fn bitwise_shift_edge_cases_test() -> Nil {
  // Test edge cases for shifts
  assert int.bitwise_shift_left(1, 63) == 9_223_372_036_854_775_808
  assert int.bitwise_shift_right(9_223_372_036_854_775_808, 63) == 1
  Nil
}

pub fn range_ascending_test() -> Nil {
  assert int.range(from: 0, to: 3, with: "", run: fn(acc, i) {
      acc <> int.to_string(i)
    })
    == "012"
  Nil
}

pub fn range_descending_test() -> Nil {
  assert int.range(from: 3, to: 0, with: "", run: fn(acc, i) {
      acc <> int.to_string(i)
    })
    == "321"
  Nil
}

pub fn range_empty_test() -> Nil {
  assert int.range(from: 5, to: 5, with: "", run: fn(acc, i) {
      acc <> int.to_string(i)
    })
    == ""
  Nil
}

pub fn range_with_list_prepend_test() -> Nil {
  assert int.range(from: 1, to: -2, with: [], run: list.prepend) == [-1, 0, 1]
  Nil
}

pub fn range_negative_to_positive_test() -> Nil {
  assert int.range(from: -2, to: 2, with: [], run: fn(acc, i) { [i, ..acc] })
    == [1, 0, -1, -2]
  Nil
}

pub fn range_single_element_test() -> Nil {
  assert int.range(from: 0, to: 1, with: [], run: list.prepend) == [0]
  Nil
}

pub fn range_sum_test() -> Nil {
  assert int.range(from: 1, to: 5, with: 0, run: int.add) == 10
  Nil
}
