import i/float

@external(wasm, "ilang:runtime/v1", "float_to_bits")
fn bits(value: Float) -> Int

@external(wasm, "ilang:runtime/v1", "float_from_bits")
fn from_bits(value: Int) -> Float

// Independent libm reference values, with a one-ULP allowance for the
// portable fdlibm algorithms. Include subnormals and range-reduction edges.
pub fn float_exponential_boundaries_test() {
  unary_cases(float.exponential, [
    #(-745.0, 1),
    #(-744.0, 2),
    #(-710.0, 906_010_418_525_123),
    #(-709.0, 2_462_791_657_071_416),
    #(-1.0, 4_600_298_746_774_613_816),
    #(-0.5, 4_603_638_362_051_417_610),
    #(-1.0e-20, 4_607_182_418_800_017_408),
    #(0.0, 4_607_182_418_800_017_408),
    #(1.0e-20, 4_607_182_418_800_017_408),
    #(0.5, 4_610_103_999_673_009_820),
    #(1.0, 4_613_303_445_314_885_481),
    #(700.0, 9_155_136_748_776_909_966),
    #(709.7827128933839, 9_218_868_437_227_404_074),
  ])
  assert float.exponential(-1000.0) == 0.0
}

pub fn float_logarithm_boundaries_test() {
  unary_cases(
    fn(value) {
      let assert Ok(result) = float.logarithm(value)
      result
    },
    [
      #(5.0e-324, 13_873_131_416_795_836_867),
      #(2.2250738585072014e-308, 13_872_814_373_467_765_970),
      #(1.0e-100, 13_865_677_554_486_826_226),
      #(0.5, 13_827_790_571_168_217_583),
      #(0.9999999999999999, 13_591_863_675_404_156_928),
      #(1.0, 0),
      #(1.0000000000000002, 4_372_995_238_176_751_615),
      #(2.0, 4_604_418_534_313_441_775),
      #(2.718281828459045, 4_607_182_418_800_017_408),
      #(1.7976931348623157e308, 4_649_454_530_587_146_735),
    ],
  )
  assert float.logarithm(0.0) == Error(Nil)
  assert float.logarithm(-1.0) == Error(Nil)
}

fn unary_cases(
  function: fn(Float) -> Float,
  cases: List(#(Float, Int)),
) -> Nil {
  case cases {
    [] -> Nil
    [#(input, expected), ..rest] -> {
      close_bits(function(input), expected)
      unary_cases(function, rest)
    }
  }
}

fn close_bits(value: Float, expected: Int) -> Nil {
  let distance = bits(value) - expected
  assert distance >= -1 && distance <= 1
}

pub fn float_power_boundaries_test() {
  power_cases([
    #(2.0, -1074.0, 1),
    #(2.0, -1075.0, 0),
    #(2.0, 1023.0, 9_214_364_837_600_034_816),
    #(8.0, 1.5, 4_627_062_269_354_654_669),
    #(1.0000000000000002, 1.0e16, 4_621_375_197_044_677_778),
    #(0.9999999999999999, 1.0e16, 4_599_607_102_468_687_849),
    #(0.5, -1000.0, 9_110_782_046_170_513_408),
    #(2.0, 0.5, 4_609_047_870_845_172_685),
    #(3.0, 0.3, 4_608_940_575_321_981_614),
    #(1.0e-5, 15.2, 3_469_786_628_572_181_841),
    #(12_345.6789, -3.125, 4_415_508_367_817_633_704),
  ])
  assert float.power(-2.0, 3.0) == Ok(-8.0)
  assert float.power(-2.0, -3.0) == Ok(-0.125)
  assert float.power(-2.0, 0.5) == Error(Nil)
  assert float.power(0.0, -1.0) == Error(Nil)
  assert float.power(0.0, 0.0) == Ok(1.0)
  let zero = from_bits(9_223_372_036_854_775_808)
  let assert Ok(square_root) = float.square_root(zero)
  assert bits(square_root) == 0
  let assert Ok(odd_power) = float.power(zero, 3.0)
  assert bits(odd_power) == 9_223_372_036_854_775_808
}

fn power_cases(cases: List(#(Float, Float, Int))) -> Nil {
  case cases {
    [] -> Nil
    [#(base, exponent, expected), ..rest] -> {
      let assert Ok(result) = float.power(base, exponent)
      close_bits(result, expected)
      power_cases(rest)
    }
  }
}

pub fn float_round_and_truncate_large_integers_test() {
  assert float.round(0.49999999999999994) == 0
  assert float.round(-0.49999999999999994) == 0
  assert float.round(0.5) == 1
  assert float.round(-0.5) == -1
  assert float.round(4_503_599_627_370_495.5) == 4_503_599_627_370_496
  assert float.round(4_503_599_627_370_497.0) == 4_503_599_627_370_497
  assert float.truncate(18_446_744_073_709_551_616.0)
    == 18_446_744_073_709_551_616
  assert float.truncate(-18_446_744_073_709_551_616.0)
    == -18_446_744_073_709_551_616
  assert float.round(18_446_744_073_709_551_616.0) == 18_446_744_073_709_551_616
}
