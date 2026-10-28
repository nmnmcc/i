//// Internal, host-independent binary64 transcendental functions.
////
//// The range reductions and polynomial coefficients are adapted from
//// OpenLibm's e_exp.c, e_log.c and e_pow.c (the fdlibm algorithms).
//// https://github.com/JuliaMath/openlibm/tree/master/src
////
//// Copyright (C) 1993, 2004 by Sun Microsystems, Inc. All rights reserved.
//// Developed at SunSoft, a Sun Microsystems, Inc. business.
//// Permission to use, copy, modify, and distribute this software is freely
//// granted, provided that this notice is preserved.
////
//// Inputs are finite. Domain checks are performed by gleam/float. The
//// explicit overflow paths preserve Gleam's finite-float semantics.

const word = 4_294_967_296

const mantissa = 4_503_599_627_370_496

const sign_bit = 9_223_372_036_854_775_808

const ln2_hi = 6.9314718036912381649e-1

const ln2_lo = 1.90821492927058770002e-10

const inverse_ln2 = 1.442695040888963387

const p1 = 1.66666666666666019037e-1

const p2 = -2.77777777770155933842e-3

const p3 = 6.61375632143793436117e-5

const p4 = -1.6533902205465251539e-6

const p5 = 4.13813679705723846039e-8

@external(wasm, "wleam:runtime/v1", "float_to_bits")
fn bits(x: Float) -> Int

@external(wasm, "wleam:runtime/v1", "float_from_bits")
fn from_bits(x: Int) -> Float

@external(wasm, "wleam:runtime/v1", "float_truncate")
fn truncate(x: Float) -> Int

@external(wasm, "wleam:runtime/v1", "int_to_float")
fn to_float(x: Int) -> Float

@external(wasm, "wleam:runtime/v1", "float_sqrt")
fn square_root(x: Float) -> Float

fn absolute(x: Float) -> Float {
  from_bits(bits(x) % sign_bit)
}

fn high(x: Float) -> Int {
  bits(x) / word
}

fn positive_high(x: Float) -> Int {
  high(x) % 0x8000_0000
}

fn with_high(x: Float, high_word: Int) -> Float {
  from_bits(high_word * word + bits(x) % word)
}

fn without_low(x: Float) -> Float {
  from_bits(bits(x) / word * word)
}

fn two_power(exponent: Int) -> Float {
  from_bits({ exponent + 1023 } * mantissa)
}

// Called with a reduced significand and -1075 <= exponent <= 1024.
// Splitting the subnormal scale avoids rounding to zero before multiplication.
fn scale(value: Float, exponent: Int) -> Float {
  case exponent {
    n if n > 1023 -> value *. 2.0 *. two_power(n - 1)
    n if n < -1021 -> value *. two_power(n + 1000) *. 9.3326361850321887899e-302
    n -> value *. two_power(n)
  }
}

fn exp_polynomial(x: Float) -> Float {
  let square = x *. x
  x
  -. square
  *. {
    p1
    +. square
    *. { p2 +. square *. { p3 +. square *. { p4 +. square *. p5 } } }
  }
}

pub fn exponential(value: Float) -> Float {
  case value {
    x if x >. 7.09782712893383973096e2 -> panic as "Float overflow in exponential"
    x if x <. -7.4513321910194110842e2 -> 0.0
    1.0 -> 2.71828182845904523536
    x -> exp_finite(x)
  }
}

fn exp_finite(value: Float) -> Float {
  let hx = positive_high(value)
  case hx < 0x3e30_0000 {
    True -> 1.0 +. value
    False -> {
      let #(exponent, hi, lo) = case hx > 0x3fd6_2e42 {
        False -> #(0, value, 0.0)
        True -> {
          let direction = case value <. 0.0 {
            True -> -1.0
            False -> 1.0
          }
          case hx < 0x3ff0_a2b2 {
            True -> #(
              truncate(direction),
              value -. direction *. ln2_hi,
              direction *. ln2_lo,
            )
            False -> {
              let k = truncate(inverse_ln2 *. value +. direction *. 0.5)
              let kf = to_float(k)
              #(k, value -. kf *. ln2_hi, kf *. ln2_lo)
            }
          }
        }
      }
      let x = hi -. lo
      let c = exp_polynomial(x)
      case exponent {
        0 -> 1.0 -. { x *. c /. { c -. 2.0 } -. x }
        k -> {
          let y = 1.0 -. { { lo -. x *. c /. { 2.0 -. c } } -. hi }
          scale(y, k)
        }
      }
    }
  }
}

pub fn logarithm(value: Float) -> Float {
  let #(value, correction) = case positive_high(value) < 0x0010_0000 {
    True -> #(value *. 1.8014398509481984e16, -54)
    False -> #(value, 0)
  }
  let high_word = high(value)
  let fraction = high_word % 0x0010_0000
  let half = case fraction + 0x0009_5f64 >= 0x0010_0000 {
    True -> 1
    False -> 0
  }
  let exponent = correction + high_word / 0x0010_0000 - 1023 + half
  let x = with_high(value, fraction + 0x3ff0_0000 - half * 0x0010_0000)
  let f = x -. 1.0
  let k = to_float(exponent)
  case { fraction + 2 } % 0x0010_0000 < 3 {
    True -> {
      case f {
        0.0 -> k *. ln2_hi +. k *. ln2_lo
        _ -> {
          let r = f *. f *. { 0.5 -. 0.33333333333333333 *. f }
          case exponent {
            0 -> f -. r
            _ -> k *. ln2_hi -. { { r -. k *. ln2_lo } -. f }
          }
        }
      }
    }
    False -> {
      let s = f /. { 2.0 +. f }
      let z = s *. s
      let w = z *. z
      let t1 =
        w
        *. {
          3.999999999940941908e-1
          +. w
          *. { 2.222219843214978396e-1 +. w *. 1.531383769920937332e-1 }
        }
      let t2 =
        z
        *. {
          6.66666666666673513e-1
          +. w
          *. {
            2.857142874366239149e-1
            +. w
            *. { 1.818357216161805012e-1 +. w *. 1.479819860511658591e-1 }
          }
        }
      let r = t1 +. t2
      case fraction >= 0x0006_147a && fraction <= 0x0006_b851 {
        True -> {
          let half_square = 0.5 *. f *. f
          case exponent {
            0 -> f -. { half_square -. s *. { half_square +. r } }
            _ ->
              k
              *. ln2_hi
              -. {
                { half_square -. { s *. { half_square +. r } +. k *. ln2_lo } }
                -. f
              }
          }
        }
        False -> {
          case exponent {
            0 -> f -. s *. { f -. r }
            _ -> k *. ln2_hi -. { { s *. { f -. r } -. k *. ln2_lo } -. f }
          }
        }
      }
    }
  }
}

// The high and low halves of log2(x) retain precision when multiplied by a
// large exponent. The split is essential for bases very close to one.
fn log2_split(value: Float) -> #(Float, Float) {
  let #(value, correction) = case high(value) < 0x0010_0000 {
    True -> #(value *. 9_007_199_254_740_992.0, -53)
    False -> #(value, 0)
  }
  let hx = high(value)
  let fraction = hx % 0x0010_0000
  let n = correction + hx / 0x0010_0000 - 1023
  let #(interval, n, hx) = case fraction {
    j if j <= 0x0003_988e -> #(0, n, j + 0x3ff0_0000)
    j if j < 0x000b_b67a -> #(1, n, j + 0x3ff0_0000)
    j -> #(0, n + 1, j + 0x3fe0_0000)
  }
  let x = with_high(value, hx)
  let #(bp, dp_hi, dp_lo) = case interval {
    0 -> #(1.0, 0.0, 0.0)
    _ -> #(1.5, 5.84962487220764160156e-1, 1.35003920212974897128e-8)
  }
  let u = x -. bp
  let v = 1.0 /. { x +. bp }
  let ss = u *. v
  let s_hi = without_low(ss)
  let t_hi =
    from_bits(
      { hx / 2 + 0x2000_0000 + 0x0008_0000 + interval * 0x0004_0000 } * word,
    )
  let t_lo = x -. { t_hi -. bp }
  let s_lo = v *. { { u -. s_hi *. t_hi } -. s_hi *. t_lo }
  let s2 = ss *. ss
  let r =
    s2
    *. s2
    *. {
      5.99999999999994648725e-1
      +. s2
      *. {
        4.28571428578550184252e-1
        +. s2
        *. {
          3.33333329818377432918e-1
          +. s2
          *. {
            2.72728123808534006489e-1
            +. s2
            *. { 2.30660745775561754067e-1 +. s2 *. 2.06975017800338417784e-1 }
          }
        }
      }
    }
  let r = r +. s_lo *. { s_hi +. ss }
  let s2 = s_hi *. s_hi
  let t_hi = without_low(3.0 +. s2 +. r)
  let t_lo = r -. { { t_hi -. 3.0 } -. s2 }
  let u = s_hi *. t_hi
  let v = s_lo *. t_hi +. t_lo *. ss
  let p_hi = without_low(u +. v)
  let p_lo = v -. { p_hi -. u }
  let z_hi = 9.61796700954437255859e-1 *. p_hi
  let z_lo =
    -7.02846165095275826516e-9
    *. p_hi
    +. p_lo
    *. 9.61796693925975554329e-1
    +. dp_lo
  let nf = to_float(n)
  let hi = without_low({ { z_hi +. z_lo } +. dp_hi } +. nf)
  let lo = z_lo -. { { { hi -. nf } -. dp_hi } -. z_hi }
  #(hi, lo)
}

fn log2_close_to_one(x: Float) -> #(Float, Float) {
  let t = x -. 1.0
  let w = t *. t *. { 0.5 -. t *. { 0.3333333333333333333333 -. t *. 0.25 } }
  let u = 1.44269502162933349609 *. t
  let v = t *. 1.92596299112661746887e-8 -. w *. inverse_ln2
  let hi = without_low(u +. v)
  #(hi, v -. { hi -. u })
}

pub fn power(base: Float, exponent: Float) -> Float {
  case exponent {
    0.0 -> 1.0
    1.0 -> base
    -1.0 -> 1.0 /. base
    2.0 -> base *. base
    3.0 -> base *. base *. base
    4.0 -> {
      let squared = base *. base
      squared *. squared
    }
    0.5 if base >=. 0.0 -> square_root(absolute(base))
    _ -> power_finite(base, exponent)
  }
}

fn power_finite(base: Float, exponent: Float) -> Float {
  let negative = bits(base) >= sign_bit
  let odd = case negative {
    True -> truncate(exponent) % 2 != 0
    False -> False
  }
  let direction = case odd {
    True -> -1.0
    False -> 1.0
  }
  let x = absolute(base)
  let ix = high(x)
  let iy = positive_high(exponent)
  case x {
    0.0 -> direction *. 0.0
    1.0 -> direction
    _ -> {
      case iy > 0x41e0_0000 {
        True -> {
          case True {
            _ if iy > 0x43f0_0000 -> power_extreme(x, exponent, direction)
            _ if ix < 0x3fef_ffff || ix > 0x3ff0_0000 ->
              power_extreme(x, exponent, direction)
            _ -> {
              let #(hi, lo) = log2_close_to_one(x)
              power_exp2(exponent, hi, lo, direction)
            }
          }
        }
        False -> {
          let #(hi, lo) = log2_split(x)
          power_exp2(exponent, hi, lo, direction)
        }
      }
    }
  }
}

fn power_extreme(base: Float, exponent: Float, direction: Float) -> Float {
  case base >. 1.0 && exponent >. 0.0 || base <. 1.0 && exponent <. 0.0 {
    True -> panic as "Float overflow in power"
    False -> direction *. 0.0
  }
}

fn power_exp2(
  exponent: Float,
  log_hi: Float,
  log_lo: Float,
  direction: Float,
) -> Float {
  let y_hi = without_low(exponent)
  let p_lo = { exponent -. y_hi } *. log_hi +. exponent *. log_lo
  let p_hi = y_hi *. log_hi
  let z = p_hi +. p_lo
  case True {
    _ if z >. 1024.0 -> panic as "Float overflow in power"
    _ if z == 1024.0 && p_lo +. 8.0085662595372944372e-17 >. z -. p_hi ->
      panic as "Float overflow in power"
    _ if z <. -1075.0 -> direction *. 0.0
    _ if z == -1075.0 && p_lo <=. z -. p_hi -> direction *. 0.0
    _ -> {
      let n = case z {
        x if x >. 0.5 -> truncate(x +. 0.5)
        x if x <. -0.5 -> truncate(x -. 0.5)
        _ -> 0
      }
      let p_hi = p_hi -. to_float(n)
      let t = without_low(p_lo +. p_hi)
      let u = t *. 6.93147182464599609375e-1
      let v =
        { p_lo -. { t -. p_hi } }
        *. 6.93147180559945286227e-1
        +. t
        *. -1.90465429995776804525e-9
      let z = u +. v
      let w = v -. { z -. u }
      let t1 = exp_polynomial(z)
      let r = z *. t1 /. { t1 -. 2.0 } -. { w +. z *. w }
      direction *. scale(1.0 -. { r -. z }, n)
    }
  }
}
