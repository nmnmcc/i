import (
  i/core
  int = std/int
  string = std/string
  list = std/list
  order = (Less, Equal, Greater) = std/order
  option = (Some, None) = std/option
)


@test
absolute_value_positive_test = () {
  assert(int.abs(123) == 123)
  Nil
}

@test
absolute_value_negative_test = () {
  assert(int.abs(-123) == 123)
  Nil
}

@test
clamp_within_range_test = () {
  assert(int.clamp(40, min: 30, max: 50) == 40)
  Nil
}

@test
clamp_below_min_test = () {
  assert(int.clamp(20, min: 30, max: 50) == 30)
  Nil
}

@test
clamp_above_max_test = () {
  assert(int.clamp(60, min: 30, max: 50) == 50)
  Nil
}

@test
clamp_inverted_range_above_test = () {
  assert(int.clamp(100, min: 50, max: 30) == 50)
  Nil
}

@test
clamp_inverted_range_within_test = () {
  assert(int.clamp(40, min: 50, max: 30) == 40)
  Nil
}

@test
to_string_positive_test = () {
  assert(int.to_string(123) == "123")
  Nil
}

@test
to_string_negative_test = () {
  assert(int.to_string(-123) == "-123")
  Nil
}

@test
to_string_positive_2_test = () {
  assert(int.to_string(123) == "123")
  Nil
}

@test
parse_positive_test = () {
  assert(int.parse("123") == Ok(123))
  Nil
}

@test
parse_negative_test = () {
  assert(int.parse("-123") == Ok(-123))
  Nil
}

@test
parse_leading_zero_test = () {
  assert(int.parse("0123") == Ok(123))
  Nil
}

@test
parse_empty_test = () {
  assert(int.parse("") == Error(Nil))
  Nil
}

@test
parse_invalid_test = () {
  assert(int.parse("what") == Error(Nil))
  Nil
}

@test
parse_float_test = () {
  assert(int.parse("1.23") == Error(Nil))
  Nil
}

@test
base_parse_hex_test = () {
  assert(int.parse_base("100", 16) == Ok(256))
  Nil
}

@test
base_parse_hex_negative_test = () {
  assert(int.parse_base("-100", 16) == Ok(-256))
  Nil
}

@test
base_parse_base_1_test = () {
  assert(int.parse_base("100", 1) == Error(Nil))
  Nil
}

@test
base_parse_base_37_test = () {
  assert(int.parse_base("100", 37) == Error(Nil))
  Nil
}

@test
base_parse_invalid_hex_test = () {
  assert(int.parse_base("AG", 16) == Error(Nil))
  Nil
}

@test
to_base_string_hex_test = () {
  assert(int.to_base(100, 16) == Ok("64"))
  Nil
}

@test
to_base_string_hex_negative_test = () {
  assert(int.to_base(-100, 16) == Ok("-64"))
  Nil
}

@test
to_base_string_base_1_test = () {
  assert(int.to_base(100, 1) == Error(Nil))
  Nil
}

@test
to_base_string_base_37_test = () {
  assert(int.to_base(100, 37) == Error(Nil))
  Nil
}

@test
to_base2_positive_test = () {
  assert(int.to_base2(100) == "1100100")
  Nil
}

@test
to_base2_negative_test = () {
  assert(int.to_base2(-100) == "-1100100")
  Nil
}

@test
to_base8_positive_test = () {
  assert(int.to_base8(100) == "144")
  Nil
}

@test
to_base8_negative_test = () {
  assert(int.to_base8(-100) == "-144")
  Nil
}

@test
to_base16_positive_test = () {
  assert(int.to_base16(100) == "64")
  Nil
}

@test
to_base16_negative_test = () {
  assert(int.to_base16(-100) == "-64")
  Nil
}

@test
to_base16_large_test = () {
  assert(int.to_base16(43_981) == "ABCD")
  Nil
}

@test
to_base16_large_negative_test = () {
  assert(int.to_base16(-43_981) == "-ABCD")
  Nil
}

@test
to_base36_positive_test = () {
  assert(int.to_base36(100) == "2S")
  Nil
}

@test
to_base36_negative_test = () {
  assert(int.to_base36(-100) == "-2S")
  Nil
}

@test
to_float_one_test = () {
  assert(int.to_float(1) == 1.0)
  Nil
}

@test
to_float_five_test = () {
  assert(int.to_float(5) == 5.0)
  Nil
}

@test
to_float_zero_test = () {
  assert(int.to_float(0) == 0.0)
  Nil
}

@test
to_float_negative_test = () {
  assert(int.to_float(-5) == -.5.0)
  Nil
}

@test
compare_equal_zero_test = () {
  assert(int.compare(0, 0) == order.Equal)
  Nil
}

@test
compare_equal_one_test = () {
  assert(int.compare(1, 1) == order.Equal)
  Nil
}

@test
compare_lt_test = () {
  assert(int.compare(0, 1) == order.Less)
  Nil
}

@test
compare_negative_lt_test = () {
  assert(int.compare(-2, -1) == order.Less)
  Nil
}

@test
compare_gt_test = () {
  assert(int.compare(2, 1) == order.Greater)
  Nil
}

@test
compare_negative_gt_test = () {
  assert(int.compare(-1, -2) == order.Greater)
  Nil
}

@test
min_equal_test = () {
  assert(int.min(0, 0) == 0)
  Nil
}

@test
min_first_smaller_test = () {
  assert(int.min(0, 1) == 0)
  Nil
}

@test
min_second_smaller_test = () {
  assert(int.min(1, 0) == 0)
  Nil
}

@test
min_negative_smaller_test = () {
  assert(int.min(-1, 2) == -1)
  Nil
}

@test
min_both_mixed_test = () {
  assert(int.min(2, -2) == -2)
  Nil
}

@test
min_both_negative_test = () {
  assert(int.min(-1, -1) == -1)
  Nil
}

@test
max_equal_test = () {
  assert(int.max(0, 0) == 0)
  Nil
}

@test
max_second_larger_test = () {
  assert(int.max(0, 1) == 1)
  Nil
}

@test
max_first_larger_test = () {
  assert(int.max(1, 0) == 1)
  Nil
}

@test
max_positive_larger_test = () {
  assert(int.max(-1, 2) == 2)
  Nil
}

@test
max_mixed_test = () {
  assert(int.max(2, -2) == 2)
  Nil
}

@test
max_both_negative_test = () {
  assert(int.max(-1, -1) == -1)
  Nil
}

@test
is_even_zero_test = () {
  assert(int.is_even(0))
  Nil
}

@test
is_even_two_test = () {
  assert(int.is_even(2))
  Nil
}

@test
is_even_negative_two_test = () {
  assert(int.is_even(-2))
  Nil
}

@test
is_even_large_test = () {
  assert(int.is_even(10_006))
  Nil
}

@test
is_even_one_false_test = () {
  assert(core.not(int.is_even(1)))
  Nil
}

@test
is_even_negative_three_false_test = () {
  assert(core.not(int.is_even(-3)))
  Nil
}

@test
is_even_large_odd_false_test = () {
  assert(core.not(int.is_even(10_005)))
  Nil
}

@test
is_odd_zero_false_test = () {
  assert(core.not(int.is_odd(0)))
  Nil
}

@test
is_odd_two_false_test = () {
  assert(core.not(int.is_odd(2)))
  Nil
}

@test
is_odd_negative_two_false_test = () {
  assert(core.not(int.is_odd(-2)))
  Nil
}

@test
is_odd_large_even_false_test = () {
  assert(core.not(int.is_odd(10_006)))
  Nil
}

@test
is_odd_one_test = () {
  assert(int.is_odd(1))
  Nil
}

@test
is_odd_negative_three_test = () {
  assert(int.is_odd(-3))
  Nil
}

@test
is_odd_large_test = () {
  assert(int.is_odd(10_005))
  Nil
}

@test
square_root_four_test = () {
  assert(int.square_root(4) == Ok(2.0))
  Nil
}

@test
square_root_sixteen_test = () {
  assert(int.square_root(16) == Ok(4.0))
  Nil
}

@test
square_root_zero_test = () {
  assert(int.square_root(0) == Ok(0.0))
  Nil
}

@test
square_root_negative_error_test = () {
  assert(int.square_root(-4) == Error(Nil))
  Nil
}

@test
negate_negative_test = () {
  assert(int.negate(-1) == 1)
  Nil
}

@test
negate_positive_test = () {
  assert(int.negate(2) == -2)
  Nil
}

@test
negate_zero_test = () {
  assert(int.negate(0) == 0)
  Nil
}

@test
sum_empty_test = () {
  assert(int.sum([]) == 0)
  Nil
}

@test
sum_non_empty_test = () {
  assert(int.sum([1, 2, 3]) == 6)
  Nil
}

@test
product_empty_test = () {
  assert(int.product([]) == 1)
  Nil
}

@test
product_single_test = () {
  assert(int.product([4]) == 4)
  Nil
}

@test
product_multiple_test = () {
  assert(int.product([1, 2, 3]) == 6)
  Nil
}

@test
divide_by_self_test = () {
  assert(int.divide(1, 1) == Ok(1))
  Nil
}

@test
divide_by_zero_test = () {
  assert(int.divide(1, 0) == Error(Nil))
  Nil
}

@test
divide_zero_by_one_test = () {
  assert(int.divide(0, by: 1) == Ok(0))
  Nil
}

@test
divide_one_by_zero_test = () {
  assert(int.divide(1, by: 0) == Error(Nil))
  Nil
}

@test
divide_truncates_test = () {
  assert(int.divide(5, by: 2) == Ok(2))
  Nil
}

@test
divide_negative_test = () {
  assert(int.divide(-99, by: 2) == Ok(-49))
  Nil
}

@test
remainder_basic_test = () {
  assert(int.remainder(3, 2) == Ok(1))
  Nil
}

@test
remainder_by_zero_test = () {
  assert(int.remainder(1, 0) == Error(Nil))
  Nil
}

@test
remainder_negative_divisor_test = () {
  assert(int.remainder(10, -1) == Ok(0))
  Nil
}

@test
remainder_positive_test = () {
  assert(int.remainder(13, by: 3) == Ok(1))
  Nil
}

@test
remainder_negative_dividend_test = () {
  assert(int.remainder(-13, by: 3) == Ok(-1))
  Nil
}

@test
remainder_positive_by_negative_test = () {
  assert(int.remainder(13, by: -3) == Ok(1))
  Nil
}

@test
remainder_both_negative_test = () {
  assert(int.remainder(-13, by: -3) == Ok(-1))
  Nil
}

@test
modulo_basic_test = () {
  assert(int.modulo(3, 2) == Ok(1))
  Nil
}

@test
modulo_by_zero_test = () {
  assert(int.modulo(1, 0) == Error(Nil))
  Nil
}

@test
modulo_negative_divisor_test = () {
  assert(int.modulo(10, -1) == Ok(0))
  Nil
}

@test
modulo_positive_test = () {
  assert(int.modulo(13, by: 3) == Ok(1))
  Nil
}

@test
modulo_negative_dividend_test = () {
  assert(int.modulo(-13, by: 3) == Ok(2))
  Nil
}

@test
modulo_positive_by_negative_test = () {
  assert(int.modulo(13, by: -3) == Ok(-2))
  Nil
}

@test
modulo_both_negative_test = () {
  assert(int.modulo(-13, by: -3) == Ok(-1))
  Nil
}

@test
floor_divide_by_self_test = () {
  assert(int.floor_divide(1, 1) == Ok(1))
  Nil
}

@test
floor_divide_by_zero_test = () {
  assert(int.floor_divide(1, 0) == Error(Nil))
  Nil
}

@test
floor_divide_zero_by_one_test = () {
  assert(int.floor_divide(0, by: 1) == Ok(0))
  Nil
}

@test
floor_divide_one_by_zero_test = () {
  assert(int.floor_divide(1, by: 0) == Error(Nil))
  Nil
}

@test
floor_divide_truncates_test = () {
  assert(int.floor_divide(5, by: 2) == Ok(2))
  Nil
}

@test
floor_divide_negative_divisor_test = () {
  assert(int.floor_divide(6, by: -4) == Ok(-2))
  Nil
}

@test
floor_divide_negative_dividend_test = () {
  assert(int.floor_divide(-99, by: 2) == Ok(-50))
  Nil
}

@test
floor_divide_negative_small_test = () {
  assert(int.floor_divide(-1, by: 2) == Ok(-1))
  Nil
}

@test
add_test = () {
  assert(int.add(1, 2) == 3)
  Nil
}

@test
add_larger_test = () {
  assert(int.add(3, 2) == 5)
  Nil
}

@test
multiply_test = () {
  assert(int.multiply(2, 4) == 8)
  Nil
}

@test
multiply_larger_test = () {
  assert(int.multiply(3, 2) == 6)
  Nil
}

@test
subtract_test = () {
  assert(int.subtract(3, 1) == 2)
  Nil
}

@test
subtract_smaller_test = () {
  assert(int.subtract(3, 2) == 1)
  Nil
}

@test
subtract_negative_result_test = () {
  assert(int.subtract(2, 3) == -1)
  Nil
}

@test
bitwise_and_test = () {
  assert(int.bitwise_and(9, 3) == 1)
  Nil
}

@test
bitwise_and_32bit_test = () {
  // To check compatibility with JavaScript, try a 32 bit unsigned integer
  // (signed integers are in the range -2147483648 to +2147483647, while
  //  32 bit unsigned integers are in the range 0 to +4294967295).
  assert(int.bitwise_and(2_147_483_648, 2_147_483_648) == 2_147_483_648)
  Nil
}

@test
bitwise_not_test = () {
  assert(int.bitwise_not(2) == -3)
  Nil
}

@test
bitwise_not_32bit_test = () {
  // To check compatibility with JavaScript, try a 32 bit unsigned integer.
  assert(int.bitwise_not(2_147_483_648) == -2_147_483_649)
  Nil
}

@test
bitwise_or_test = () {
  assert(int.bitwise_or(9, 3) == 11)
  Nil
}

@test
bitwise_or_32bit_test = () {
  // To check compatibility with JavaScript, try a 32 bit unsigned integer.
  assert(int.bitwise_or(1, 2_147_483_648) == 2_147_483_649)
  Nil
}

@test
bitwise_exclusive_or_test = () {
  assert(int.bitwise_xor(9, 3) == 10)
  Nil
}

@test
bitwise_exclusive_or_32bit_test = () {
  // To check compatibility with JavaScript, try a 32 bit unsigned integer.
  assert(int.bitwise_xor(0, 2_147_483_648) == 2_147_483_648)
  Nil
}

@test
bitwise_shift_left_positive_test = () {
  assert(int.bitwise_shift_left(1, 2) == 4)
  Nil
}

@test
bitwise_shift_left_negative_shift_test = () {
  assert(int.bitwise_shift_left(1, -2) == 0)
  Nil
}

@test
bitwise_shift_left_negative_value_test = () {
  assert(int.bitwise_shift_left(-1, 2) == -4)
  Nil
}

@test
bitwise_shift_left_both_negative_test = () {
  assert(int.bitwise_shift_left(-1, -2) == -1)
  Nil
}

@test
bitwise_shift_right_positive_test = () {
  assert(int.bitwise_shift_right(1, 2) == 0)
  Nil
}

@test
bitwise_shift_right_negative_shift_test = () {
  assert(int.bitwise_shift_right(1, -2) == 4)
  Nil
}

@test
bitwise_shift_right_negative_value_test = () {
  assert(int.bitwise_shift_right(-1, 2) == -1)
  Nil
}

@test
bitwise_shift_right_both_negative_test = () {
  assert(int.bitwise_shift_right(-1, -2) == -4)
  Nil
}

@test
bitwise_and_zero_test = () {
  assert(int.bitwise_and(0, 0) == 0)
  Nil
}

@test
bitwise_and_identity_test = () {
  assert(int.bitwise_and(0xFF, 0xFF) == 0xFF)
  Nil
}

@test
bitwise_and_clear_test = () {
  assert(int.bitwise_and(0xFF, 0) == 0)
  Nil
}

@test
bitwise_and_large_numbers_test = () {
  // Test with numbers larger than 32 bits
  assert(int.bitwise_and(0x123456789, 0xFFFF) == 0x6789)
  Nil
}

@test
bitwise_or_zero_test = () {
  assert(int.bitwise_or(0, 0) == 0)
  Nil
}

@test
bitwise_or_identity_test = () {
  assert(int.bitwise_or(0xFF, 0) == 0xFF)
  Nil
}

@test
bitwise_or_combine_test = () {
  assert(int.bitwise_or(0xF0, 0x0F) == 0xFF)
  Nil
}

@test
bitwise_or_large_numbers_test = () {
  // Test with numbers larger than 32 bits
  assert(int.bitwise_or(0x100000000, 0xFF) == 0x1000000FF)
  Nil
}

@test
bitwise_exclusive_or_zero_test = () {
  assert(int.bitwise_xor(0, 0) == 0)
  Nil
}

@test
bitwise_exclusive_or_identity_test = () {
  assert(int.bitwise_xor(0xFF, 0) == 0xFF)
  Nil
}

@test
bitwise_exclusive_or_invert_test = () {
  assert(int.bitwise_xor(0xF0, 0x0F) == 0xFF)
  Nil
}

@test
bitwise_exclusive_or_self_test = () {
  assert(int.bitwise_xor(0xAB, 0xAB) == 0)
  Nil
}

@test
bitwise_not_zero_test = () {
  assert(int.bitwise_not(0) == -1)
  Nil
}

@test
bitwise_not_negative_one_test = () {
  assert(int.bitwise_not(-1) == 0)
  Nil
}

@test
bitwise_not_large_number_test = () {
  // Test with number larger than 32 bits
  assert(int.bitwise_not(0x123456789) == -4_886_718_346)
  Nil
}

@test
bitwise_shift_left_zero_shift_test = () {
  assert(int.bitwise_shift_left(0xFF, 0) == 0xFF)
  Nil
}

@test
bitwise_shift_left_large_shift_test = () {
  assert(int.bitwise_shift_left(1, 32) == 4_294_967_296)
  Nil
}

@test
bitwise_shift_left_large_number_test = () {
  assert(int.bitwise_shift_left(0x1000, 4) == 0x10000)
  Nil
}

@test
bitwise_shift_right_zero_shift_test = () {
  assert(int.bitwise_shift_right(0xFF, 0) == 0xFF)
  Nil
}

@test
bitwise_shift_right_large_shift_test = () {
  assert(int.bitwise_shift_right(0xFFFFFFFF, 16) == 0xFFFF)
  Nil
}

@test
bitwise_shift_right_preserves_sign_test = () {
  assert(int.bitwise_shift_right(-1, 16) == -1)
  Nil
}

@test
bitwise_shift_right_large_number_test = () {
  assert(int.bitwise_shift_right(0x123456789, 16) == 0x12345)
  Nil
}

@test
bitwise_operations_with_max_safe_integer_test = () {
  // Test with maximum safe integer
  assert(int.bitwise_and(9_007_199_254_740_991, 0xFFFF) == 65_535)
  Nil
}

@test
bitwise_operations_with_boundary_values_test = () {
  // Test around 32-bit boundaries
  assert(int.bitwise_or(2_147_483_647, 1) == 2_147_483_647)
  // MAX_INT32 | 1 = MAX_INT32
  assert(int.bitwise_and(2_147_483_648, 4_294_967_295) == 2_147_483_648)
  // 2^31 & 0xFFFFFFFF = 2^31
  Nil
}

@test
bitwise_shift_edge_cases_test = () {
  // Test edge cases for shifts
  assert(int.bitwise_shift_left(1, 63) == 9_223_372_036_854_775_808)
  assert(int.bitwise_shift_right(9_223_372_036_854_775_808, 63) == 1)
  Nil
}

@test
range_ascending_test = () {
  assert(int.range(from: 0, to: 3, with: "", run: (acc, i) {
      acc <> int.to_string(i)
    })
    == "012")
  Nil
}

@test
range_descending_test = () {
  assert(int.range(from: 3, to: 0, with: "", run: (acc, i) {
      acc <> int.to_string(i)
    })
    == "321")
  Nil
}

@test
range_empty_test = () {
  assert(int.range(from: 5, to: 5, with: "", run: (acc, i) {
      acc <> int.to_string(i)
    })
    == "")
  Nil
}

@test
range_with_list_prepend_test = () {
  assert(int.range(from: 1, to: -2, with: [], run: list.prepend) == [-1, 0, 1])
  Nil
}

@test
range_negative_to_positive_test = () {
  assert(int.range(from: -2, to: 2, with: [], run: (acc, i) { [i, ..acc] })
    == [1, 0, -1, -2])
  Nil
}

@test
range_single_element_test = () {
  assert(int.range(from: 0, to: 1, with: [], run: list.prepend) == [0])
  Nil
}

@test
range_sum_test = () {
  assert(int.range(from: 1, to: 5, with: 0, run: int.add) == 10)
  Nil
}

@test
power_two_squared_test = () {
  assert(int.power(2, 2) == Ok(4))
}

@test
power_negative_cubed_test = () {
  assert(int.power(-5, 3) == Ok(-125))
}

@test
power_zero_exponent_test = () {
  assert(int.power(10, 0) == Ok(1))
  assert(int.power(0, 0) == Ok(1))
}

@test
power_big_test = () {
  assert(int.power(2, of: 100) == Ok(1267650600228229401496703205376))
  assert(int.power(10, of: 30) == Ok(1_000_000_000_000_000_000_000_000_000_000))
}

@test
power_negative_exponent_test = () {
  assert(int.power(2, -1) == Error(Nil))
}

@test
power_pipe_test = () {
  assert({ 3 |> int.power(of: 4) } == Ok(81))
}

// Tests for functions that are new in std, or that changed from the old
// library.

@test
parse_more_test = () {
  assert(int.parse("+7") == Ok(7))
  assert(int.parse("-0") == Ok(0))
  assert(int.parse("123456789012345678901234567890") == Ok(123456789012345678901234567890))
  assert(int.parse("") == Error(Nil))
  assert(int.parse("-") == Error(Nil))
  assert(int.parse("1_000") == Error(Nil))
  assert(int.parse(" 1") == Error(Nil))
  assert(int.parse("1e3") == Error(Nil))
  assert(int.parse("٣") == Error(Nil))
}

@test
parse_base_more_test = () {
  assert(int.parse_base("ff", 16) == Ok(255))
  assert(int.parse_base("FF", 16) == Ok(255))
  assert(int.parse_base("-101", 2) == Ok(-5))
  assert(int.parse_base("z", 36) == Ok(35))
  assert(int.parse_base("2", 2) == Error(Nil))
  assert(int.parse_base("1", 1) == Error(Nil))
  assert(int.parse_base("1", 37) == Error(Nil))
}

@test
to_base_more_test = () {
  assert(int.to_base(255, 16) == Ok("FF"))
  assert(int.to_base(-5, 2) == Ok("-101"))
  assert(int.to_base(0, 7) == Ok("0"))
  assert(int.to_base(35, 36) == Ok("Z"))
  assert(int.to_base(1, 37) == Error(Nil))
  assert(int.to_base(1, 1) == Error(Nil))
  big = 1267650600228229401496703205376
  assert(int.to_base(big, 2) == Ok("1" <> string.repeat("0", 100)))
  assert(int.parse_base(int.to_base16(big), 16) == Ok(big))
}

@test
digits_more_test = () {
  assert(int.digits(234, 10) == Ok([2, 3, 4]))
  assert(int.digits(-234, 10) == Ok([2, 3, 4]))
  assert(int.digits(0, 10) == Ok([0]))
  assert(int.digits(5, 2) == Ok([1, 0, 1]))
  assert(int.digits(5, 1) == Error(Nil))
  assert(int.undigits([1, 0, 1], 2) == Ok(5))
  assert(int.undigits([1, 2], 2) == Error(Nil))
  assert(int.undigits([], 10) == Ok(0))
}

@test
division_more_test = () {
  assert(int.divide(7, by: 2) == Ok(3))
  assert(int.divide(-7, by: 2) == Ok(-3))
  assert(int.divide(7, by: 0) == Error(Nil))
  assert(int.remainder(-13, by: 3) == Ok(-1))
  assert(int.remainder(13, by: -3) == Ok(1))
  assert(int.modulo(-13, by: 3) == Ok(2))
  assert(int.modulo(13, by: -3) == Ok(-2))
  assert(int.modulo(-13, by: -3) == Ok(-1))
  assert(int.modulo(12, by: 3) == Ok(0))
  assert(int.floor_divide(-7, by: 2) == Ok(-4))
  assert(int.floor_divide(7, by: -2) == Ok(-4))
  assert(int.floor_divide(-8, by: 2) == Ok(-4))
  assert(int.floor_divide(1, by: 0) == Error(Nil))
}

@test
big_numbers_test = () {
  big = 100000000000000000000
  assert(int.abs(-big) == big)
  assert(int.compare(big, big + 1) == Less)
  assert(int.max(big, -big) == big)
  assert(int.sum([big, big]) == 2 * big)
  assert(int.to_string(-big) == "-100000000000000000000")
  assert(int.is_even(big))
}

@test
sum_product_long_test = () {
  xs = list.repeat(1, 1_000_000)
  assert(int.sum(xs) == 1_000_000)
  assert(int.product(xs) == 1)
}

@test
range_long_test = () {
  assert(int.range(from: 0, to: 1_000_000, with: 0, run: int.add) == 499_999_500_000)
}
