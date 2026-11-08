//// Functions for working with integers.
////
//// ## Division by zero
////
//// In Erlang division by zero results in a crash, however Gleam does not have
//// partial functions and operators in core so instead division by zero returns
//// zero, a behaviour taken from Pony, Coq, and Lean.
////
//// This may seem unexpected at first, but it is no less mathematically valid
//// than crashing or returning a special value. Division by zero is undefined
//// in mathematics.

import i/float
import i/order.{type Order}

/// Returns the absolute value of the input.
///
/// ## Examples
///
/// ```i
/// assert int.absolute_value(-12) == 12
/// ```
///
/// ```i
/// assert int.absolute_value(10) == 10
/// ```
///
pub fn absolute_value(x: Int) -> Int {
  case x >= 0 {
    True -> x
    False -> x * -1
  }
}

/// Returns the result of the base being raised to the power of the
/// exponent, as a `Float`.
///
/// ## Examples
///
/// ```i
/// assert int.power(2, -1.0) == Ok(0.5)
/// ```
///
/// ```i
/// assert int.power(2, 2.0) == Ok(4.0)
/// ```
///
/// ```i
/// assert int.power(8, 1.5) == Ok(22.627416997969522)
/// ```
///
/// ```i
/// assert 4 |> int.power(of: 2.0) == Ok(16.0)
/// ```
///
/// ```i
/// assert int.power(-1, 0.5) == Error(Nil)
/// ```
///
pub fn power(base: Int, of exponent: Float) -> Result(Float, Nil) {
  base
  |> to_float
  |> float.power(exponent)
}

/// Returns the square root of the input as a `Float`.
///
/// ## Examples
///
/// ```i
/// assert int.square_root(4) == Ok(2.0)
/// ```
///
/// ```i
/// assert int.square_root(-16) == Error(Nil)
/// ```
///
pub fn square_root(x: Int) -> Result(Float, Nil) {
  x
  |> to_float
  |> float.square_root()
}

/// Parses a given string as an int if possible.
///
/// ## Examples
///
/// ```i
/// assert int.parse("2") == Ok(2)
/// ```
///
/// ```i
/// assert int.parse("ABC") == Error(Nil)
/// ```
///
pub fn parse(string: String) -> Result(Int, Nil) {
  do_base_parse(string, 10)
}

/// Parses a given string as an int in a given base, returning an error if the
/// input was not a valid number for the given base.
///
/// Supports only bases 2 to 36, for values outside of which this function
/// returns an `Error(Nil)`.
///
/// ## Examples
///
/// ```i
/// assert int.base_parse("10", 2) == Ok(2)
/// ```
///
/// ```i
/// assert int.base_parse("30", 16) == Ok(48)
/// ```
///
/// ```i
/// assert int.base_parse("1C", 36) == Ok(48)
/// ```
///
/// ```i
/// assert int.base_parse("48", 1) == Error(Nil)
/// ```
///
/// ```i
/// assert int.base_parse("48", 37) == Error(Nil)
/// ```
///
pub fn base_parse(string: String, base: Int) -> Result(Int, Nil) {
  case base >= 2 && base <= 36 {
    True -> do_base_parse(string, base)
    False -> Error(Nil)
  }
}

fn do_base_parse(string: String, base: Int) -> Result(Int, Nil) {
  case <<string:utf8>> {
    <<45, rest:bits>> -> parse_digits(rest, base, -1, 0, False)
    <<43, rest:bits>> -> parse_digits(rest, base, 1, 0, False)
    bytes -> parse_digits(bytes, base, 1, 0, False)
  }
}

fn parse_digits(
  bytes: BitArray,
  base: Int,
  sign: Int,
  accumulator: Int,
  seen_digit: Bool,
) -> Result(Int, Nil) {
  case bytes {
    <<>> ->
      case seen_digit {
        True -> Ok(sign * accumulator)
        False -> Error(Nil)
      }
    <<byte, rest:bits>> -> {
      let digit = case byte {
        n if n >= 48 && n <= 57 -> n - 48
        n if n >= 65 && n <= 90 -> n - 65 + 10
        n if n >= 97 && n <= 122 -> n - 97 + 10
        _ -> 36
      }
      case digit < base {
        True -> parse_digits(rest, base, sign, accumulator * base + digit, True)
        False -> Error(Nil)
      }
    }
    _ -> Error(Nil)
  }
}

/// Prints a given int to a string.
///
/// ## Examples
///
/// ```i
/// assert int.to_string(2) == "2"
/// ```
///
pub fn to_string(x: Int) -> String {
  do_to_base_string(x, 10)
}

/// Prints a given int to a string using the base number provided.
/// Supports only bases 2 to 36, for values outside of which this function returns an `Error(Nil)`.
/// For common bases (2, 8, 16, 36), use the `to_baseN` functions.
///
/// ## Examples
///
/// ```i
/// assert int.to_base_string(2, 2) == Ok("10")
/// ```
///
/// ```i
/// assert int.to_base_string(48, 16) == Ok("30")
/// ```
///
/// ```i
/// assert int.to_base_string(48, 36) == Ok("1C")
/// ```
///
/// ```i
/// assert int.to_base_string(48, 1) == Error(Nil)
/// ```
///
/// ```i
/// assert int.to_base_string(48, 37) == Error(Nil)
/// ```
///
pub fn to_base_string(x: Int, base: Int) -> Result(String, Nil) {
  case base >= 2 && base <= 36 {
    True -> Ok(do_to_base_string(x, base))
    False -> Error(Nil)
  }
}

fn do_to_base_string(value: Int, base: Int) -> String {
  case value {
    0 -> "0"
    n if n < 0 -> "-" <> format_digits(-n, base, "")
    n -> format_digits(n, base, "")
  }
}

fn format_digits(value: Int, base: Int, accumulator: String) -> String {
  case value {
    0 -> accumulator
    _ ->
      format_digits(
        value / base,
        base,
        digit_string(value % base) <> accumulator,
      )
  }
}

fn digit_string(digit: Int) -> String {
  case digit {
    0 -> "0"
    1 -> "1"
    2 -> "2"
    3 -> "3"
    4 -> "4"
    5 -> "5"
    6 -> "6"
    7 -> "7"
    8 -> "8"
    9 -> "9"
    10 -> "A"
    11 -> "B"
    12 -> "C"
    13 -> "D"
    14 -> "E"
    15 -> "F"
    16 -> "G"
    17 -> "H"
    18 -> "I"
    19 -> "J"
    20 -> "K"
    21 -> "L"
    22 -> "M"
    23 -> "N"
    24 -> "O"
    25 -> "P"
    26 -> "Q"
    27 -> "R"
    28 -> "S"
    29 -> "T"
    30 -> "U"
    31 -> "V"
    32 -> "W"
    33 -> "X"
    34 -> "Y"
    _ -> "Z"
  }
}

/// Prints a given int to a string using base-2.
///
/// ## Examples
///
/// ```i
/// assert int.to_base2(2) == "10"
/// ```
///
pub fn to_base2(x: Int) -> String {
  do_to_base_string(x, 2)
}

/// Prints a given int to a string using base-8.
///
/// ## Examples
///
/// ```i
/// assert int.to_base8(15) == "17"
/// ```
///
pub fn to_base8(x: Int) -> String {
  do_to_base_string(x, 8)
}

/// Prints a given int to a string using base-16.
///
/// ## Examples
///
/// ```i
/// assert int.to_base16(48) == "30"
/// ```
///
pub fn to_base16(x: Int) -> String {
  do_to_base_string(x, 16)
}

/// Prints a given int to a string using base-36.
///
/// ## Examples
///
/// ```i
/// assert int.to_base36(48) == "1C"
/// ```
///
pub fn to_base36(x: Int) -> String {
  do_to_base_string(x, 36)
}

/// Takes an int and returns its value as a float.
///
/// ## Examples
///
/// ```i
/// assert int.to_float(5) == 5.0
/// ```
///
/// ```i
/// assert int.to_float(0) == 0.0
/// ```
///
/// ```i
/// assert int.to_float(-3) == -3.0
/// ```
///
@external(wasm, "ilang:runtime/v1", "int_to_float")
pub fn to_float(x: Int) -> Float

/// Restricts an int between two bounds.
///
/// Note: If the `min` argument is larger than the `max` argument then they
/// will be swapped, so the minimum bound is always lower than the maximum
/// bound.
///
/// ## Examples
///
/// ```i
/// assert int.clamp(40, min: 50, max: 60) == 50
/// ```
///
/// ```i
/// assert int.clamp(40, min: 50, max: 30) == 40
/// ```
///
pub fn clamp(x: Int, min min_bound: Int, max max_bound: Int) -> Int {
  case min_bound >= max_bound {
    True -> x |> min(min_bound) |> max(max_bound)
    False -> x |> min(max_bound) |> max(min_bound)
  }
}

/// Compares two ints, returning an order.
///
/// ## Examples
///
/// ```i
/// assert int.compare(2, 3) == Lt
/// ```
///
/// ```i
/// assert int.compare(4, 3) == Gt
/// ```
///
/// ```i
/// assert int.compare(3, 3) == Eq
/// ```
///
pub fn compare(a: Int, with b: Int) -> Order {
  case a == b {
    True -> order.Eq
    False ->
      case a < b {
        True -> order.Lt
        False -> order.Gt
      }
  }
}

/// Compares two ints, returning the smaller of the two.
///
/// ## Examples
///
/// ```i
/// assert int.min(2, 3) == 2
/// ```
///
pub fn min(a: Int, b: Int) -> Int {
  case a < b {
    True -> a
    False -> b
  }
}

/// Compares two ints, returning the larger of the two.
///
/// ## Examples
///
/// ```i
/// assert int.max(2, 3) == 3
/// ```
///
pub fn max(a: Int, b: Int) -> Int {
  case a > b {
    True -> a
    False -> b
  }
}

/// Returns whether the value provided is even.
///
/// ## Examples
///
/// ```i
/// assert int.is_even(2)
/// ```
///
/// ```i
/// assert !int.is_even(3)
/// ```
///
pub fn is_even(x: Int) -> Bool {
  x % 2 == 0
}

/// Returns whether the value provided is odd.
///
/// ## Examples
///
/// ```i
/// assert int.is_odd(3)
/// ```
///
/// ```i
/// assert !int.is_odd(2)
/// ```
///
pub fn is_odd(x: Int) -> Bool {
  x % 2 != 0
}

/// Returns the negative of the value provided.
///
/// ## Examples
///
/// ```i
/// assert int.negate(1) == -1
/// ```
///
pub fn negate(x: Int) -> Int {
  -1 * x
}

/// Sums a list of ints.
///
/// ## Example
///
/// ```i
/// assert int.sum([1, 2, 3]) == 6
/// ```
///
pub fn sum(numbers: List(Int)) -> Int {
  sum_loop(numbers, 0)
}

fn sum_loop(numbers: List(Int), initial: Int) -> Int {
  case numbers {
    [first, ..rest] -> sum_loop(rest, first + initial)
    [] -> initial
  }
}

/// Multiplies a list of ints and returns the product.
///
/// ## Example
///
/// ```i
/// assert int.product([2, 3, 4]) == 24
/// ```
///
pub fn product(numbers: List(Int)) -> Int {
  product_loop(numbers, 1)
}

fn product_loop(numbers: List(Int), initial: Int) -> Int {
  case numbers {
    [first, ..rest] -> product_loop(rest, first * initial)
    [] -> initial
  }
}

/// Generates a random int between zero and the given maximum.
///
/// The lower number is inclusive, the upper number is exclusive.
///
/// ## Examples
///
/// ```i
/// int.random(10)
/// // -> 4
/// ```
///
/// ```i
/// int.random(1)
/// // -> 0
/// ```
///
/// ```i
/// int.random(-1)
/// // -> -1
/// ```
///
pub fn random(max: Int) -> Int {
  { float.random() *. to_float(max) }
  |> float.floor
  |> float.round
}

/// Performs a truncated integer division.
///
/// Returns division of the inputs as a `Result`: If the given divisor equals
/// `0`, this function returns an `Error`.
///
/// ## Examples
///
/// ```i
/// assert int.divide(0, 1) == Ok(0)
/// ```
///
/// ```i
/// assert int.divide(1, 0) == Error(Nil)
/// ```
///
/// ```i
/// assert int.divide(5, 2) == Ok(2)
/// ```
///
/// ```i
/// assert int.divide(-99, 2) == Ok(-49)
/// ```
///
pub fn divide(dividend: Int, by divisor: Int) -> Result(Int, Nil) {
  case divisor {
    0 -> Error(Nil)
    divisor -> Ok(dividend / divisor)
  }
}

/// Computes the remainder of an integer division of inputs as a `Result`.
///
/// Returns division of the inputs as a `Result`: If the given divisor equals
/// `0`, this function returns an `Error`.
///
/// Most of the time you will want to use the `%` operator instead of this
/// function.
///
/// ## Examples
///
/// ```i
/// assert int.remainder(3, 2) == Ok(1)
/// ```
///
/// ```i
/// assert int.remainder(1, 0) == Error(Nil)
/// ```
///
/// ```i
/// assert int.remainder(10, -1) == Ok(0)
/// ```
///
/// ```i
/// assert int.remainder(13, by: 3) == Ok(1)
/// ```
///
/// ```i
/// assert int.remainder(-13, by: 3) == Ok(-1)
/// ```
///
/// ```i
/// assert int.remainder(13, by: -3) == Ok(1)
/// ```
///
/// ```i
/// assert int.remainder(-13, by: -3) == Ok(-1)
/// ```
///
pub fn remainder(dividend: Int, by divisor: Int) -> Result(Int, Nil) {
  case divisor {
    0 -> Error(Nil)
    divisor -> Ok(dividend % divisor)
  }
}

/// Computes the modulo of an integer division of inputs as a `Result`.
///
/// Returns division of the inputs as a `Result`: If the given divisor equals
/// `0`, this function returns an `Error`.
///
/// Note that this is different from `int.remainder` and `%` in that the
/// computed value will always have the same sign as the `divisor`.
///
/// ## Examples
///
/// ```i
/// assert int.modulo(3, 2) == Ok(1)
/// ```
///
/// ```i
/// assert int.modulo(1, 0) == Error(Nil)
/// ```
///
/// ```i
/// assert int.modulo(10, -1) == Ok(0)
/// ```
///
/// ```i
/// assert int.modulo(13, by: 3) == Ok(1)
/// ```
///
/// ```i
/// assert int.modulo(-13, by: 3) == Ok(2)
/// ```
///
/// ```i
/// assert int.modulo(13, by: -3) == Ok(-2)
/// ```
///
pub fn modulo(dividend: Int, by divisor: Int) -> Result(Int, Nil) {
  case divisor {
    0 -> Error(Nil)
    _ -> {
      let remainder = dividend % divisor
      case remainder * divisor < 0 {
        True -> Ok(remainder + divisor)
        False -> Ok(remainder)
      }
    }
  }
}

/// Performs a *floored* integer division, which means that the result will
/// always be rounded towards negative infinity.
///
/// If you want to perform truncated integer division (rounding towards zero),
/// use `int.divide()` or the `/` operator instead.
///
/// Returns division of the inputs as a `Result`: If the given divisor equals
/// `0`, this function returns an `Error`.
///
/// ## Examples
///
/// ```i
/// assert int.floor_divide(1, 0) == Error(Nil)
/// ```
///
/// ```i
/// assert int.floor_divide(5, 2) == Ok(2)
/// ```
///
/// ```i
/// assert int.floor_divide(6, -4) == Ok(-2)
/// ```
///
/// ```i
/// assert int.floor_divide(-99, 2) == Ok(-50)
/// ```
///
pub fn floor_divide(dividend: Int, by divisor: Int) -> Result(Int, Nil) {
  case divisor {
    0 -> Error(Nil)
    divisor ->
      case dividend * divisor < 0 && dividend % divisor != 0 {
        True -> Ok(dividend / divisor - 1)
        False -> Ok(dividend / divisor)
      }
  }
}

/// Adds two integers together.
///
/// It's the function equivalent of the `+` operator.
/// This function is useful in higher order functions or pipes.
///
/// ## Examples
///
/// ```i
/// assert int.add(1, 2) == 3
/// ```
///
/// ```i
/// import i/list
///
/// assert list.fold([1, 2, 3], 0, int.add) == 6
/// ```
///
/// ```i
/// assert 3 |> int.add(2) == 5
/// ```
///
pub fn add(a: Int, b: Int) -> Int {
  a + b
}

/// Multiplies two integers together.
///
/// It's the function equivalent of the `*` operator.
/// This function is useful in higher order functions or pipes.
///
/// ## Examples
///
/// ```i
/// assert int.multiply(2, 4) == 8
/// ```
///
/// ```i
/// import i/list
///
/// assert list.fold([2, 3, 4], 1, int.multiply) == 24
/// ```
///
/// ```i
/// assert 3 |> int.multiply(2) == 6
/// ```
///
pub fn multiply(a: Int, b: Int) -> Int {
  a * b
}

/// Subtracts one int from another.
///
/// It's the function equivalent of the `-` operator.
/// This function is useful in higher order functions or pipes.
///
/// ## Examples
///
/// ```i
/// assert int.subtract(3, 1) == 2
/// ```
///
/// ```i
/// import i/list
///
/// assert list.fold([1, 2, 3], 10, int.subtract) == 4
/// ```
///
/// ```i
/// assert 3 |> int.subtract(2) == 1
/// ```
///
/// ```i
/// assert 3 |> int.subtract(2, _) == -1
/// ```
///
pub fn subtract(a: Int, b: Int) -> Int {
  a - b
}

/// Calculates the bitwise AND of its arguments.
///
/// Most the time you should use the bit array syntaxes instead of manipulating
/// bits as ints with bitwise functions.
///
/// Integers have arbitrary precision. Negative values use infinite
/// two's-complement sign extension, consistently across Wasm hosts.
///
@external(wasm, "ilang:runtime/v1", "int_bitwise_and")
pub fn bitwise_and(x: Int, y: Int) -> Int {
  binary_bits(x, y, 0, 1, 0)
}

/// Calculates the bitwise NOT of its argument.
///
/// Most the time you should use the bit array syntaxes instead of manipulating
/// bits as ints with bitwise functions.
///
/// Integers have arbitrary precision. Negative values use infinite
/// two's-complement sign extension, consistently across Wasm hosts.
///
@external(wasm, "ilang:runtime/v1", "int_bitwise_not")
pub fn bitwise_not(x: Int) -> Int {
  -1 - x
}

/// Calculates the bitwise OR of its arguments.
///
/// Most the time you should use the bit array syntaxes instead of manipulating
/// bits as ints with bitwise functions.
///
/// Integers have arbitrary precision. Negative values use infinite
/// two's-complement sign extension, consistently across Wasm hosts.
///
@external(wasm, "ilang:runtime/v1", "int_bitwise_or")
pub fn bitwise_or(x: Int, y: Int) -> Int {
  binary_bits(x, y, 1, 1, 0)
}

/// Calculates the bitwise XOR of its arguments.
///
/// Most the time you should use the bit array syntaxes instead of manipulating
/// bits as ints with bitwise functions.
///
/// Integers have arbitrary precision. Negative values use infinite
/// two's-complement sign extension, consistently across Wasm hosts.
///
@external(wasm, "ilang:runtime/v1", "int_bitwise_xor")
pub fn bitwise_exclusive_or(x: Int, y: Int) -> Int {
  binary_bits(x, y, 2, 1, 0)
}

// The sign-extension base cases make this operate on infinite two's-complement
// integers. The tail-recursive loop uses no fixed-width host representation.
fn binary_bits(
  a: Int,
  b: Int,
  operation: Int,
  place: Int,
  accumulator: Int,
) -> Int {
  case a, b {
    0, 0 -> accumulator
    -1, -1 ->
      case operation {
        2 -> accumulator
        _ -> accumulator - place
      }
    -1, 0 | 0, -1 ->
      case operation {
        0 -> accumulator
        _ -> accumulator - place
      }
    _, _ -> {
      let low_a = case a % 2 == 0 {
        True -> 0
        False -> 1
      }
      let low_b = case b % 2 == 0 {
        True -> 0
        False -> 1
      }
      let low = case operation {
        0 -> low_a * low_b
        1 ->
          case low_a + low_b == 0 {
            True -> 0
            False -> 1
          }
        _ -> { low_a + low_b } % 2
      }
      binary_bits(
        { a - low_a } / 2,
        { b - low_b } / 2,
        operation,
        place * 2,
        accumulator + low * place,
      )
    }
  }
}

/// Calculates the result of an arithmetic left bitshift.
///
/// Most the time you should use the bit array syntaxes instead of manipulating
/// bits as ints with bitwise functions.
///
/// Integers have arbitrary precision. Negative values use infinite
/// two's-complement sign extension, consistently across Wasm hosts.
///
@external(wasm, "ilang:runtime/v1", "int_bitwise_shift_left")
pub fn bitwise_shift_left(x: Int, y: Int) -> Int {
  case x, y {
    0, _ -> 0
    _, n if n < 0 -> shift_right_loop(x, -n)
    _, n -> x * power_of_two(n, 2, 1)
  }
}

fn power_of_two(exponent: Int, base: Int, accumulator: Int) -> Int {
  case exponent {
    0 -> accumulator
    1 -> accumulator * base
    n -> {
      let accumulator = case n % 2 {
        0 -> accumulator
        _ -> accumulator * base
      }
      power_of_two(n / 2, base * base, accumulator)
    }
  }
}

/// Calculates the result of an arithmetic right bitshift.
///
/// Most the time you should use the bit array syntaxes instead of manipulating
/// bits as ints with bitwise functions.
///
/// Integers have arbitrary precision. Negative values use infinite
/// two's-complement sign extension, consistently across Wasm hosts.
///
@external(wasm, "ilang:runtime/v1", "int_bitwise_shift_right")
pub fn bitwise_shift_right(x: Int, y: Int) -> Int {
  case y < 0 {
    True -> bitwise_shift_left(x, -y)
    False -> shift_right_loop(x, y)
  }
}

fn shift_right_loop(value: Int, count: Int) -> Int {
  case value, count {
    _, 0 -> value
    0, _ -> 0
    -1, _ -> -1
    n, _ -> {
      let low = case n % 2 == 0 {
        True -> 0
        False -> 1
      }
      shift_right_loop({ n - low } / 2, count - 1)
    }
  }
}

/// Run a function for each int between ints `from` and `to`.
///
/// `from` is inclusive, and `to` is exclusive.
///
/// ## Examples
///
/// ```i
/// assert int.range(from: 0, to: 3, with: "", run: fn(acc, i) {
///     acc <> int.to_string(i)
///   })
///   == "012"
/// ```
///
/// ```i
/// assert int.range(from: 1, to: -2, with: [], run: list.prepend) == [-1, 0, 1]
/// ```
///
pub fn range(
  from start: Int,
  to stop: Int,
  with acc: acc,
  run reducer: fn(acc, Int) -> acc,
) -> acc {
  let increment = case start < stop {
    True -> 1
    False -> -1
  }
  range_loop(start, stop, increment, acc, reducer)
}

fn range_loop(
  current: Int,
  stop: Int,
  increment: Int,
  acc: acc,
  reducer: fn(acc, Int) -> acc,
) -> acc {
  case current == stop {
    True -> acc
    False -> {
      let acc = reducer(acc, current)
      let current = current + increment
      range_loop(current, stop, increment, acc, reducer)
    }
  }
}
