import i/ilang_float_decimal as decimal

@external(wasm, "ilang:runtime/v1", "float_to_bits")
fn bits(value: Float) -> Int

@external(wasm, "ilang:runtime/v1", "float_from_bits")
fn from_bits(bits: Int) -> Float

// Expected binary64 bits are independent CPython decimal parser results,
// including both midpoint ties, the normal/subnormal boundary, and signed zero.
pub fn float_decimal_exact_rounding_test() {
  parse_cases([
    #("0.0", 0),
    #("-0.0", 9_223_372_036_854_775_808),
    #("+1.23", 4_608_218_246_714_312_622),
    #("-1.23", 13_831_590_283_569_088_430),
    #("0.1", 4_591_870_180_066_957_722),
    #("1.234e10", 4_757_767_320_795_873_280),
    #("1.234e-10", 4_458_833_837_745_914_792),
    #("1.7976931348623157e308", 9_218_868_437_227_405_311),
    #("2.2250738585072014e-308", 4_503_599_627_370_496),
    #("2.225073858507201e-308", 4_503_599_627_370_495),
    #("5.0e-324", 1),
    #("2.4703282292062327e-324", 0),
    #("2.4703282292062328e-324", 1),
    #("-1.0e-400", 9_223_372_036_854_775_808),
    #("9007199254740993.0", 4_845_873_199_050_653_696),
    #(
      "1.00000000000000011102230246251565404236316680908203125",
      4_607_182_418_800_017_408,
    ),
    #(
      "1.00000000000000033306690738754696212708950042724609375",
      4_607_182_418_800_017_410,
    ),
  ])
}

fn parse_cases(cases: List(#(String, Int))) -> Nil {
  case cases {
    [] -> Nil
    [#(text, expected), ..rest] -> {
      let assert Ok(value) = decimal.parse(text)
      assert bits(value) == expected
      parse_cases(rest)
    }
  }
}

pub fn float_decimal_invalid_and_overflow_test() {
  invalid_cases([
    "", "1", ".1", "1.", " 1.0", "1.0 ", "1e1", "+", "-", "1.0e", "1.0e+",
    "1.0e-", "1.0_0", "NaN", "Infinity", "0x1.0p1", "1.7976931348623159e308",
    "1.0e99999999999999999999", "0.0e99999999999999999x",
  ])
  assert decimal.to_string(0.0) == "0.0"
  let assert Ok(negative_zero) = decimal.parse("-0.0e99999999999999999")
  assert decimal.to_string(negative_zero) == "-0.0"
}

fn invalid_cases(cases: List(String)) -> Nil {
  case cases {
    [] -> Nil
    [text, ..rest] -> {
      assert decimal.parse(text) == Error(Nil)
      invalid_cases(rest)
    }
  }
}

pub fn float_decimal_shortest_format_test() {
  assert decimal.to_string(0.0123) == "0.0123"
  assert decimal.to_string(-0.0123) == "-0.0123"
  assert decimal.to_string(123.0) == "123.0"
  assert decimal.to_string(3.0e26) == "3.0e26"
  assert decimal.to_string(-3.0e-26) == "-3.0e-26"
  assert decimal.to_string(456.12e78) == "4.5612e80"
  assert decimal.to_string(9_007_199_254_740_992.0) == "9.007199254740992e15"
  assert decimal.to_string(5.0e-324) == "5.0e-324"
  assert decimal.to_string(1.7976931348623157e308) == "1.7976931348623157e308"
  // Independent OTP 28.5 strings cover neighbors where binary64 spacing changes
  // and both ends of the normal finite range.
  format_cases([
    #(4_503_599_627_370_495, "2.225073858507201e-308"),
    #(4_503_599_627_370_496, "2.2250738585072014e-308"),
    #(4_503_599_627_370_497, "2.225073858507202e-308"),
    #(4_607_182_418_800_017_407, "0.9999999999999999"),
    #(4_607_182_418_800_017_408, "1.0"),
    #(4_607_182_418_800_017_409, "1.0000000000000002"),
    #(4_611_686_018_427_387_903, "1.9999999999999998"),
    #(4_611_686_018_427_387_904, "2.0"),
    #(4_611_686_018_427_387_905, "2.0000000000000004"),
    #(9_218_868_437_227_405_310, "1.7976931348623155e308"),
    #(9_218_868_437_227_405_311, "1.7976931348623157e308"),
  ])
}

fn format_cases(cases: List(#(Int, String))) -> Nil {
  case cases {
    [] -> Nil
    [#(expected_bits, expected_text), ..rest] -> {
      let text = decimal.to_string(from_bits(expected_bits))
      assert text == expected_text
      let assert Ok(decoded) = decimal.parse(text)
      assert bits(decoded) == expected_bits
      format_cases(rest)
    }
  }
}

pub fn float_decimal_bitwise_roundtrip_test() {
  roundtrip(177_293_419, 128)
}

fn roundtrip(seed: Int, count: Int) -> Nil {
  case count {
    0 -> Nil
    _ -> {
      let seed =
        { seed * 6_364_136_223_846_793_005 + 1_442_695_040_888_963_407 }
        % 18_446_744_073_709_551_616
      // Exclude infinity/NaN, retaining independently chosen finite signs.
      let magnitude = seed % 9_218_868_437_227_405_312
      let sign = seed / 9_223_372_036_854_775_808 * 9_223_372_036_854_775_808
      let input = magnitude + sign
      let value = from_bits(input)
      let assert Ok(decoded) = decimal.parse(decimal.to_string(value))
      assert bits(decoded) == input
      roundtrip(seed, count - 1)
    }
  }
}
