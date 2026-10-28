//// Exact, host-independent decimal conversion for the Wasm floating point type.
////
//// Decimal input is rounded once, using integer ratios and IEEE 754 round-to-
//// nearest, ties-to-even. Output searches the shortest decimal grid that
//// round-trips to the original binary64 value. At most seventeen significant
//// digits are required; all decisions use integers rather than host formatting.

const fraction_unit = 4_503_599_627_370_496

const significand_limit = 9_007_199_254_740_992

const sign_bit = 9_223_372_036_854_775_808

@external(wasm, "wleam:runtime/v1", "float_to_bits")
fn to_bits(value: Float) -> Int

@external(wasm, "wleam:runtime/v1", "float_from_bits")
fn from_bits(bits: Int) -> Float

/// Parses the same decimal syntax as `gleam/float.parse` without delegating
/// parsing or rounding to the host.
pub fn parse(text: String) -> Result(Float, Nil) {
  let #(negative, bytes) = case <<text:utf8>> {
    <<45, rest:bytes>> -> #(True, rest)
    <<43, rest:bytes>> -> #(False, rest)
    bytes -> #(False, bytes)
  }
  whole(bytes, negative, 0, 0, 0)
}

fn whole(
  bytes: BitArray,
  negative: Bool,
  coefficient: Int,
  significant_digits: Int,
  count: Int,
) -> Result(Float, Nil) {
  case bytes {
    <<digit, rest:bytes>> if digit >= 48 && digit <= 57 ->
      whole(
        rest,
        negative,
        coefficient * 10 + digit - 48,
        count_significant(significant_digits, digit - 48),
        count + 1,
      )
    <<46, rest:bytes>> if count > 0 ->
      fractional(rest, negative, coefficient, significant_digits, 0)
    _ -> Error(Nil)
  }
}

fn fractional(
  bytes: BitArray,
  negative: Bool,
  coefficient: Int,
  significant_digits: Int,
  fractional_digits: Int,
) -> Result(Float, Nil) {
  case bytes {
    <<digit, rest:bytes>> if digit >= 48 && digit <= 57 ->
      fractional(
        rest,
        negative,
        coefficient * 10 + digit - 48,
        count_significant(significant_digits, digit - 48),
        fractional_digits + 1,
      )
    <<>> if fractional_digits > 0 ->
      decimal(negative, coefficient, significant_digits, -fractional_digits)
    <<marker, rest:bytes>>
      if fractional_digits > 0 && { marker == 69 || marker == 101 }
    -> {
      let #(exponent_negative, digits) = case rest {
        <<45, tail:bytes>> -> #(True, tail)
        <<43, tail:bytes>> -> #(False, tail)
        _ -> #(False, rest)
      }
      let limit = significant_digits + fractional_digits + 400
      case exponent(digits, 0, 0, limit) {
        Error(_) -> Error(Nil)
        Ok(exponent) -> {
          let exponent = case exponent_negative {
            True -> -exponent
            False -> exponent
          }
          decimal(
            negative,
            coefficient,
            significant_digits,
            exponent - fractional_digits,
          )
        }
      }
    }
    _ -> Error(Nil)
  }
}

fn count_significant(previous: Int, digit: Int) -> Int {
  case previous == 0 && digit == 0 {
    True -> 0
    False -> previous + 1
  }
}

// Once an exponent exceeds every possible cancellation by the already-read
// mantissa, its exact magnitude is irrelevant. Still consume every byte so an
// invalid suffix never becomes a successful parse.
fn exponent(
  bytes: BitArray,
  value: Int,
  count: Int,
  limit: Int,
) -> Result(Int, Nil) {
  case bytes {
    <<digit, rest:bytes>> if digit >= 48 && digit <= 57 -> {
      let next = case value > limit {
        True -> limit + 1
        False -> value * 10 + digit - 48
      }
      exponent(rest, next, count + 1, limit)
    }
    <<>> if count > 0 -> Ok(value)
    _ -> Error(Nil)
  }
}

fn decimal(
  negative: Bool,
  coefficient: Int,
  digits: Int,
  exponent: Int,
) -> Result(Float, Nil) {
  let sign = case negative {
    True -> sign_bit
    False -> 0
  }
  let order = digits + exponent - 1
  case coefficient, order {
    0, _ -> Ok(from_bits(sign))
    _, order if order > 308 -> Error(Nil)
    _, order if order < -324 -> Ok(from_bits(sign))
    _, _ -> {
      let #(numerator, denominator) = decimal_ratio(coefficient, exponent)
      case ratio_bits(numerator, denominator) {
        Ok(bits) -> Ok(from_bits(sign + bits))
        Error(_) -> Error(Nil)
      }
    }
  }
}

fn decimal_ratio(coefficient: Int, exponent: Int) -> #(Int, Int) {
  case exponent >= 0 {
    True -> #(coefficient * power(10, exponent), 1)
    False -> #(coefficient, power(10, -exponent))
  }
}

fn ratio_bits(numerator: Int, denominator: Int) -> Result(Int, Nil) {
  case numerator {
    0 -> Ok(0)
    _ -> {
      let exponent = radix_order(numerator, denominator, 2)
      case exponent {
        exponent if exponent > 1023 -> Error(Nil)
        exponent if exponent < -1022 ->
          Ok(round_ratio(numerator * power(2, 1074), denominator))
        _ -> {
          let shift = 52 - exponent
          let significand = case shift >= 0 {
            True -> round_ratio(numerator * power(2, shift), denominator)
            False -> round_ratio(numerator, denominator * power(2, -shift))
          }
          case significand == significand_limit {
            True if exponent == 1023 -> Error(Nil)
            True -> Ok({ exponent + 1024 } * fraction_unit)
            False ->
              Ok(
                { exponent + 1023 }
                * fraction_unit
                + significand
                - fraction_unit,
              )
          }
        }
      }
    }
  }
}

// Find floor(log_radix(numerator / denominator)) using logarithmically many
// integer comparisons. Repeated squaring avoids walking all 1074 binary places
// for subnormals or all 324 decimal places for their shortest representation.
fn radix_order(numerator: Int, denominator: Int, radix: Int) -> Int {
  case numerator >= denominator {
    True -> {
      let #(order, _) = order_factors(numerator, denominator, radix, 1, [])
      order
    }
    False -> {
      let #(order, scaled) = order_factors(denominator, numerator, radix, 1, [])
      case denominator == scaled {
        True -> -order
        False -> -order - 1
      }
    }
  }
}

fn order_factors(
  numerator: Int,
  denominator: Int,
  factor: Int,
  exponent: Int,
  factors: List(#(Int, Int)),
) -> #(Int, Int) {
  case numerator >= denominator * factor {
    True ->
      order_factors(numerator, denominator, factor * factor, exponent * 2, [
        #(factor, exponent),
        ..factors
      ])
    False -> order_search(numerator, denominator, factors, 0)
  }
}

fn order_search(
  numerator: Int,
  denominator: Int,
  factors: List(#(Int, Int)),
  order: Int,
) -> #(Int, Int) {
  case factors {
    [] -> #(order, denominator)
    [#(factor, exponent), ..rest] -> {
      let scaled = denominator * factor
      case numerator >= scaled {
        True -> order_search(numerator, scaled, rest, order + exponent)
        False -> order_search(numerator, denominator, rest, order)
      }
    }
  }
}

fn round_ratio(numerator: Int, denominator: Int) -> Int {
  let quotient = numerator / denominator
  let twice_remainder = { numerator % denominator } * 2
  case
    twice_remainder > denominator
    || { twice_remainder == denominator && quotient % 2 == 1 }
  {
    True -> quotient + 1
    False -> quotient
  }
}

fn power(base: Int, exponent: Int) -> Int {
  power_loop(base, exponent, 1)
}

fn power_loop(base: Int, exponent: Int, accumulator: Int) -> Int {
  case exponent {
    0 -> accumulator
    1 -> accumulator * base
    _ -> {
      let accumulator = case exponent % 2 {
        0 -> accumulator
        _ -> accumulator * base
      }
      power_loop(base * base, exponent / 2, accumulator)
    }
  }
}

/// Formats a finite binary64 value with the shortest decimal mantissa that
/// parses to the same bit pattern, including the sign of zero.
pub fn to_string(value: Float) -> String {
  let bits = to_bits(value)
  let negative = bits >= sign_bit
  let bits = bits % sign_bit
  let prefix = case negative {
    True -> "-"
    False -> ""
  }
  case bits {
    0 -> prefix <> "0.0"
    _ -> {
      let exponent = bits / fraction_unit
      case exponent == 2047 {
        True -> panic as "A non-finite float cannot be formatted"
        False -> {
          let mantissa = case exponent {
            0 -> bits % fraction_unit
            _ -> fraction_unit + bits % fraction_unit
          }
          let binary_exponent = case exponent {
            0 -> -1074
            _ -> exponent - 1075
          }
          // All three values share a denominator. Quarter-ULP units represent
          // the closer predecessor at normal powers of two exactly. The first
          // normal binade still has the same spacing as its subnormal neighbor.
          let center = mantissa * 4
          let lower = case exponent > 1 && bits % fraction_unit == 0 {
            True -> center - 1
            False -> center - 2
          }
          let upper = center + 2
          let #(numerator, denominator, lower, upper) = case
            binary_exponent >= 2
          {
            True -> {
              let scale = power(2, binary_exponent - 2)
              #(center * scale, 1, lower * scale, upper * scale)
            }
            False -> #(center, power(2, 2 - binary_exponent), lower, upper)
          }
          let order = radix_order(numerator, denominator, 10)
          let #(numerator, denominator, lower, upper) = case order >= 0 {
            True -> #(numerator, denominator * power(10, order), lower, upper)
            False -> {
              let scale = power(10, -order)
              #(numerator * scale, denominator, lower * scale, upper * scale)
            }
          }
          let #(coefficient, exponent) =
            shortest(
              numerator,
              denominator,
              lower,
              upper,
              mantissa % 2 == 0,
              order,
              1,
            )
          prefix <> render(coefficient, exponent)
        }
      }
    }
  }
}

fn shortest(
  numerator: Int,
  denominator: Int,
  lower: Int,
  upper: Int,
  inclusive: Bool,
  exponent: Int,
  digits: Int,
) -> #(Int, Int) {
  let rounded = round_ratio(numerator, denominator)
  let candidate = rounded * denominator
  case inside(candidate, lower, upper, inclusive) {
    True -> trim_zeros(rounded, exponent)
    False ->
      case inside(candidate - denominator, lower, upper, inclusive) {
        True -> trim_zeros(rounded - 1, exponent)
        False ->
          case inside(candidate + denominator, lower, upper, inclusive) {
            True -> trim_zeros(rounded + 1, exponent)
            False if digits < 17 ->
              shortest(
                numerator * 10,
                denominator,
                lower * 10,
                upper * 10,
                inclusive,
                exponent - 1,
                digits + 1,
              )
            False -> panic as "Binary64 decimal conversion invariant failed"
          }
      }
  }
}

// A decimal inside these exact midpoints rounds to this float. Midpoint ties
// belong to an even significand; max-finite therefore excludes the overflow
// midpoint. No candidate needs another general decimal-to-binary conversion.
fn inside(value: Int, lower: Int, upper: Int, inclusive: Bool) -> Bool {
  case inclusive {
    True -> value >= lower && value <= upper
    False -> value > lower && value < upper
  }
}

fn trim_zeros(coefficient: Int, exponent: Int) -> #(Int, Int) {
  case coefficient % 10 {
    0 -> trim_zeros(coefficient / 10, exponent + 1)
    _ -> #(coefficient, exponent)
  }
}

// Match the fixed/scientific presentation of Erlang's shortest float format.
// The mantissa itself is obtained independently by exact integer arithmetic.
fn render(coefficient: Int, exponent: Int) -> String {
  let digits = digits(coefficient, [])
  let length = length(digits, 0)
  let scientific_exponent = exponent + length - 1
  let lower = case length {
    1 -> -4
    _ -> -length - 2
  }
  let upper = case length == 1 || scientific_exponent >= 10 {
    True -> 2
    False -> 1
  }
  let exact_integer = case exponent >= 0 {
    True -> coefficient * power(10, exponent) < significand_limit
    False -> True
  }
  case exponent >= lower && exponent <= upper && exact_integer {
    True -> fixed(digits, length, exponent)
    False -> scientific(digits, scientific_exponent)
  }
}

fn fixed(digits: List(String), length: Int, exponent: Int) -> String {
  case exponent >= 0, length + exponent > 0 {
    True, _ -> join(digits, "") <> zeroes(exponent, "") <> ".0"
    False, True -> {
      let #(whole, fractional) = split(digits, length + exponent, "")
      whole <> "." <> join(fractional, "")
    }
    False, False -> "0." <> zeroes(-length - exponent, "") <> join(digits, "")
  }
}

fn scientific(digits: List(String), exponent: Int) -> String {
  let mantissa = case digits {
    [first] -> first <> ".0"
    [first, ..rest] -> first <> "." <> join(rest, "")
    [] -> panic as "An empty decimal mantissa cannot be formatted"
  }
  let exponent = case exponent < 0 {
    True -> "-" <> unsigned_text(-exponent)
    False -> unsigned_text(exponent)
  }
  mantissa <> "e" <> exponent
}

fn unsigned_text(value: Int) -> String {
  case value {
    0 -> "0"
    _ -> join(digits(value, []), "")
  }
}

fn digits(value: Int, accumulator: List(String)) -> List(String) {
  case value {
    0 -> accumulator
    _ -> digits(value / 10, [digit_text(value % 10), ..accumulator])
  }
}

fn digit_text(digit: Int) -> String {
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
    _ -> panic as "Invalid decimal digit"
  }
}

fn length(items: List(a), accumulator: Int) -> Int {
  case items {
    [] -> accumulator
    [_, ..rest] -> length(rest, accumulator + 1)
  }
}

fn join(parts: List(String), accumulator: String) -> String {
  case parts {
    [] -> accumulator
    [part, ..rest] -> join(rest, accumulator <> part)
  }
}

fn zeroes(count: Int, accumulator: String) -> String {
  case count {
    count if count <= 0 -> accumulator
    _ -> zeroes(count - 1, accumulator <> "0")
  }
}

fn split(
  parts: List(String),
  count: Int,
  accumulator: String,
) -> #(String, List(String)) {
  case count, parts {
    0, _ -> #(accumulator, parts)
    _, [] -> #(accumulator, [])
    _, [part, ..rest] -> split(rest, count - 1, accumulator <> part)
  }
}
