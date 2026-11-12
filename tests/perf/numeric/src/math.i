// Diagnostic copy of de4d58d:std/src/float.i; kept for the NUM baseline.
//// Functions for `Float`, the type of finite IEEE 754 binary64 numbers.
////
//// The trigonometric functions, hypot and angle conversions round the real
//// result once to binary64, nearest with ties to even and gradual underflow.
//// Inputs are exact binary rationals; pi and tau are rounded constants, never
//// exact reduction periods. Checked results reject domains or rounded overflow,
//// including the tie above max_finite. Nonzero underflow keeps its sign.
//// A certified Float fast path falls back to growing outward fixed-point
//// intervals. Both paths have the same result on every target; no host math
//// import is used. Refinement has no fixed time or space bound. See F1–F9 in
//// docs/design/std-3.md for the bounds and exact special-value tables. Existing
//// exp/log/power retain their established contracts.
////
//// A Float operation whose result is not finite stops the program
//// (§11.3), so a Float is never infinite or NaN. `a /. 0.0` is `0.0`;
//// `divide` gives `Error(Nil)` for a zero divisor instead. Functions here
//// that could give a result that is not finite, such as `power`, give
//// `Error(Nil)` in its place, so they never stop the program.
(
	i/core
	(Option, Some, None) = std/option
	order = (Order, Less, Equal, Greater) = std/order
)

(
	fast_sin
	parse
	to_string
	compare
	loosely_compare
	loosely_equals
	min
	max
	clamp
	abs
	negate
	ceiling
	floor
	round
	truncate
	to_precision
	power
	real_power
	exponential
	logarithm
	square_root
	sum
	product
	divide
	modulo
	add
	subtract
	multiply
	is_integer
	max_finite
	min_positive
	pi
	tau
	sin
	cos
	tan
	asin
	acos
	atan
	atan2
	hypot
	radians
	degrees
	sign_bit
	copy_sign
)

/// The greatest finite Float, about `1.8e308`.
max_finite = 1.7976931348623157e308

/// The smallest Float greater than `0.0`: `5.0e-324`, a subnormal number.
min_positive = 5.0e-324

sign_mask = 9223372036854775808

fraction_unit = 4503599627370496

significand_limit = 9007199254740992

/// The text form of the Float (§11.8), the same as `core.inspect`: the
/// shortest digits that read back as the same number, with `.0` or an
/// exponent where needed.
///
/// ```i
/// float.to_string(2.5)     // "2.5"
/// float.to_string(1.0e20)  // "1.0e20"
/// ```
to_string = (x: Float) -> String { core.inspect(x) }

/// Reads a Float: an optional `-` or `+`, one or more digits, `.`, one or
/// more digits, and then an optional exponent: `e` or `E`, an optional `-`
/// or `+`, and one or more digits. The result is the nearest Float, with
/// ties to even, worked out with exact integers. Text that is not in this
/// form, or a number too big for a Float, gives `Error(Nil)`. A number too
/// small for a Float gives `0.0` with its sign.
///
/// ```i
/// float.parse("1.5")      // Ok(1.5)
/// float.parse("-2.0e-3")  // Ok(-0.002)
/// float.parse("1")        // Error(Nil)
/// ```
parse = (text: String) -> Result(Float, Nil) {
	<<text:utf8>> |>
		(<<45, rest:bytes>>) { whole(rest, True, 0, 0, 0) }
		| (<<43, rest:bytes>>) { whole(rest, False, 0, 0, 0) }
		| (bytes) { whole(bytes, False, 0, 0, 0) }
}

is_digit = (byte: Int) -> Bool { byte >= 48 && byte <= 57 }

// The digits before the point.
whole = (
	bytes: BitArray,
	negative: Bool,
	coefficient: Int,
	significant: Int,
	count: Int
) -> Result(Float, Nil) {
	bytes |>
		(<<d, rest:bytes>> ? is_digit(d)) {
			whole(
				rest,
				negative,
				coefficient * 10 + d - 48,
				count_significant(significant, d - 48),
				count + 1,
			)
		}
		| (<<46, rest:bytes>> ? count > 0) {
			fraction(rest, negative, coefficient, significant, 0)
		}
		| (_) { Error(Nil) }
}

// The digits after the point.
fraction = (
	bytes: BitArray,
	negative: Bool,
	coefficient: Int,
	significant: Int,
	digits: Int
) -> Result(Float, Nil) {
	bytes |>
		(<<d, rest:bytes>> ? is_digit(d)) {
			fraction(
				rest,
				negative,
				coefficient * 10 + d - 48,
				count_significant(significant, d - 48),
				digits + 1,
			)
		}
		| (<<>> ? digits > 0) {
			decimal(negative, coefficient, significant, -digits)
		}
		| (
			<<marker, rest:bytes>>
			? digits > 0 && { marker == 69 || marker == 101 }
		) {
			(exponent_negative, after) = rest |>
				(<<45, tail:bytes>>) { (True, tail) }
				| (<<43, tail:bytes>>) { (False, tail) }
				| (_) { (False, rest) }
			limit = significant + digits + 400
			exponent(after, 0, 0, limit) |>
				(Error(_)) { Error(Nil) }
				| (Ok(e)) {
					signed = exponent_negative |> (True) { -e } | (False) { e }
					decimal(negative, coefficient, significant, signed - digits)
				}
		}
		| (_) { Error(Nil) }
}

// The number of significant digits, not counting zeros at the start.
count_significant = (previous: Int, digit: Int) -> Int {
	{ previous == 0 && digit == 0 } |> (True) { 0 } | (False) { previous + 1 }
}

// When an exponent is greater than any digits could make up for, its exact
// size has no effect, so it is kept at `limit + 1`. Every byte is still read,
// so that bad text after it is still found.
exponent = (bytes: BitArray, value: Int, count: Int, limit: Int) -> Result(
	Int,
	Nil,
) {
	bytes |>
		(<<d, rest:bytes>> ? is_digit(d)) {
			next = { value > limit } |>
				(True) { limit + 1 }
				| (False) { value * 10 + d - 48 }
			exponent(rest, next, count + 1, limit)
		}
		| (<<>> ? count > 0) { Ok(value) }
		| (_) { Error(Nil) }
}

// The Float nearest to `coefficient` × 10^`exponent`.
decimal = (
	negative: Bool,
	coefficient: Int,
	digits: Int,
	exponent: Int
) -> Result(Float, Nil) {
	sign = negative |> (True) { sign_mask } | (False) { 0 }
	magnitude = digits + exponent - 1
	{ coefficient == 0 } |>
		(True) { Ok(core.float_from_bits(sign)) }
		| (False) {
			{ magnitude > 308 } |>
				(True) { Error(Nil) }
				| (False) {
					{ magnitude < -324 } |>
						(True) { Ok(core.float_from_bits(sign)) }
						| (False) {
							(numerator, denominator) = { exponent >= 0 } |>
								(True) {
									(coefficient * int_power(10, exponent), 1)
								}
								| (False) {
									(coefficient, int_power(10, -exponent))
								}
							ratio_bits(numerator, denominator) |>
								(Ok(bits)) {
									Ok(core.float_from_bits(sign + bits))
								}
								| (Error(_)) { Error(Nil) }
						}
				}
		}
}

// The bits of the Float nearest to `numerator / denominator`, which is more
// than 0.
ratio_bits = (numerator: Int, denominator: Int) -> Result(Int, Nil) {
	e = radix_order(numerator, denominator, 2)
	{ e > 1023 } |>
		(True) { Error(Nil) }
		| (False) {
			{ e < -1022 } |>
				(True) {
					Ok(round_ratio(numerator * int_power(2, 1074), denominator))
				}
				| (False) {
					shift = 52 - e
					significand = { shift >= 0 } |>
						(True) {
							round_ratio(
								numerator * int_power(2, shift),
								denominator,
							)
						}
						| (False) {
							round_ratio(
								numerator,
								denominator * int_power(2, -shift),
							)
						}
					{ significand == significand_limit } |>
						(True) {
							{ e == 1023 } |>
								(True) { Error(Nil) }
								| (False) { Ok({ e + 1024 } * fraction_unit) }
						}
						| (False) {
							Ok(
								{ e + 1023 } * fraction_unit +
									significand -
									fraction_unit,
							)
						}
				}
		}
}

// floor(log_radix(numerator / denominator)), with few comparisons: the
// factors radix^1, radix^2, radix^4, … are found by squaring, and then used
// from the greatest down.
radix_order = (numerator: Int, denominator: Int, radix: Int) -> Int {
	{ numerator >= denominator } |>
		(True) { order_factors(numerator, denominator, radix, 1, []).0 }
		| (False) {
			(o, scaled) = order_factors(denominator, numerator, radix, 1, [])
			{ denominator == scaled } |> (True) { -o } | (False) { -o - 1 }
		}
}

order_factors = (
	numerator: Int,
	denominator: Int,
	factor: Int,
	e: Int,
	factors: List((Int, Int))
) -> (Int, Int) {
	{ numerator >= denominator * factor } |>
		(True) {
			order_factors(
				numerator,
				denominator,
				factor * factor,
				e * 2,
				[(factor, e), ..factors],
			)
		}
		| (False) { order_search(numerator, denominator, factors, 0) }
}

order_search = (
	numerator: Int,
	denominator: Int,
	factors: List((Int, Int)),
	o: Int
) -> (Int, Int) {
	factors |>
		([]) { (o, denominator) }
		| ([(factor, e), ..rest]) {
			scaled = denominator * factor
			{ numerator >= scaled } |>
				(True) { order_search(numerator, scaled, rest, o + e) }
				| (False) { order_search(numerator, denominator, rest, o) }
		}
}

// numerator / denominator, rounded to the nearest, with ties to even.
round_ratio = (numerator: Int, denominator: Int) -> Int {
	q = numerator / denominator
	twice = { numerator % denominator } * 2
	{ twice > denominator || { twice == denominator && q % 2 == 1 } } |>
		(True) { q + 1 }
		| (False) { q }
}

int_power = (base: Int, e: Int) -> Int {
	go = (base, e, acc) {
		e |>
			(0) { acc }
			| (_) {
				e % 2 |>
					(0) { go(base * base, e / 2, acc) }
					| (_) { go(base * base, e / 2, acc * base) }
			}
	}
	go(base, e, 1)
}

/// Compares two Floats. `0.0` and `-0.0` are `Equal`.
compare = (a: Float, with: Float) -> Order {
	{ a == with } |>
		(True) { Equal }
		| (False) { { a <. with } |> (True) { Less } | (False) { Greater } }
}

/// Compares two Floats, and gives `Equal` when they are no more than
/// `tolerating` apart.
loosely_compare = (a: Float, with: Float, tolerating: Float) -> Order {
	loosely_equals(a, with, tolerating) |>
		(True) { Equal }
		| (False) { compare(a, with) }
}

/// `True` when the two Floats are no more than `tolerating` apart.
///
/// ```i
/// float.loosely_equals(0.1 +. 0.2, with: 0.3, tolerating: 1.0e-9)  // True
/// ```
loosely_equals = (a: Float, with: Float, tolerating: Float) -> Bool {
	// `a -. with` is not finite when they are far apart, so it is not used.
	{ a >=. with } |>
		(True) { a -. tolerating <=. with }
		| (False) { with -. tolerating <=. a }
}

/// The smaller of two Floats.
min = (a: Float, b: Float) -> Float {
	{ a <. b } |> (True) { a } | (False) { b }
}

/// The greater of two Floats.
max = (a: Float, b: Float) -> Float {
	{ a >. b } |> (True) { a } | (False) { b }
}

/// The Float, but not less than `min` and not greater than `max`. When
/// `min` is greater than `max`, the two bounds are swapped.
clamp = (x: Float, min: Float, max: Float) -> Float {
	(low, high) = { min <=. max } |>
		(True) { (min, max) }
		| (False) { (max, min) }
	{ x <. low } |>
		(True) { low }
		| (False) { { x >. high } |> (True) { high } | (False) { x } }
}

/// The Float without its sign.
abs = (x: Float) -> Float { { x <. 0.0 } |> (True) { -.x } | (False) { x } }

/// The Float with the other sign.
negate = (x: Float) -> Float { -.x }

/// The smallest whole number that is not less than `x`.
ceiling = (x: Float) -> Float { core.ceiling(x) }

/// The greatest whole number that is not greater than `x`.
floor = (x: Float) -> Float { core.floor(x) }

/// The nearest whole number, as an Int. A half goes away from zero.
///
/// ```i
/// float.round(2.5)    // 3
/// float.round(-2.5)   // -3
/// ```
round = (x: Float) -> Int {
	{ x <. 0.0 } |>
		(True) { -round_positive(-.x) }
		| (False) { round_positive(x) }
}

round_positive = (x: Float) -> Int {
	f = core.floor(x)
	// `x -. f` is exact for `x >= 0`.
	{ x -. f >=. 0.5 } |>
		(True) { core.truncate(f) + 1 }
		| (False) { core.truncate(f) }
}

/// The Float without its fraction, as an Int, rounded toward zero.
truncate = (x: Float) -> Int { core.truncate(x) }

/// `True` when the Float is a whole number.
is_integer = (x: Float) -> Bool { core.floor(x) == x }

/// The Float rounded to `precision` digits after the point. A negative
/// precision rounds to tens, hundreds, and so on. The result is the
/// nearest Float to the rounded number, and so it may not be exact.
///
/// ```i
/// float.to_precision(2.43434348473, 2)  // 2.43
/// float.to_precision(547890.453444, -3)  // 548000.0
/// ```
to_precision = (x: Float, precision: Int) -> Float {
	{ precision <= 0 } |>
		(True) {
			factor = core.to_float(int_power(10, -precision))
			core.to_float(round(x /. factor)) *. factor
		}
		| (False) {
			factor = core.to_float(int_power(10, precision))
			core.to_float(round(x *. factor)) /. factor
		}
}

/// `base` to the power of a whole number. A negative exponent gives the
/// inverse: `power(2.0, -1)` is `0.5`. `0.0` with a negative exponent, and a
/// result too big for a Float, give `Error(Nil)`. The result is within one
/// unit in the last place of the exact power, and exact when the exact power
/// is a Float.
///
/// ```i
/// float.power(2.0, of: 10)  // Ok(1024.0)
/// float.power(-.2.0, of: -1)  // Ok(-.0.5)
/// ```
power = (base: Float, of: Int) -> Result(Float, Nil) {
	{ base == 0.0 && of < 0 } |>
		(True) { Error(Nil) }
		| (False) {
			// Beyond 2^62 only the sign and the parity of the exponent matter.
			limit = 4611686018427387904
			e = { of > limit } |>
				(True) { limit + of % 2 }
				| (False) {
					{ of < -limit } |>
						(True) { -limit + of % 2 }
						| (False) { of }
				}
			{ base == -.1.0 } |>
				(True) {
					{ e % 2 == 0 } |> (True) { Ok(1.0) } | (False) { Ok(-.1.0) }
				}
				| (False) { pow(base, core.to_float(e)) }
		}
}

/// The square root. A negative number gives `Error(Nil)`.
///
/// ```i
/// float.square_root(4.0)   // Ok(2.0)
/// float.square_root(-1.0)  // Error(Nil)
/// ```
square_root = (x: Float) -> Result(Float, Nil) {
	{ x <. 0.0 } |> (True) { Error(Nil) } | (False) { Ok(core.sqrt(x)) }
}

/// The sum of the Floats. The sum of no Floats is `0.0`. A sum that is not
/// finite stops the program.
sum = (numbers: List(Float)) -> Float {
	go = (rest, acc) { rest |> ([]) { acc } | ([x, ..xs]) { go(xs, acc +. x) } }
	go(numbers, 0.0)
}

/// The product of the Floats. The product of no Floats is `1.0`. A product
/// that is not finite stops the program.
product = (numbers: List(Float)) -> Float {
	go = (rest, acc) { rest |> ([]) { acc } | ([x, ..xs]) { go(xs, acc *. x) } }
	go(numbers, 1.0)
}

/// `a /. by`, or `Error(Nil)` when `by` is `0.0`.
divide = (a: Float, by: Float) -> Result(Float, Nil) {
	{ by == 0.0 } |> (True) { Error(Nil) } | (False) { Ok(a /. by) }
}

/// The remainder of `a` divided by `by`, rounded down, with the sign of
/// `by`, or `Error(Nil)` when `by` is `0.0`.
///
/// ```i
/// float.modulo(13.3, by: 3.3)  // Ok(0.10000000000000053)
/// ```
modulo = (a: Float, by: Float) -> Result(Float, Nil) {
	{ by == 0.0 } |>
		(True) { Error(Nil) }
		| (False) { Ok(a -. core.floor(a /. by) *. by) }
}

/// `a +. b`, as a function.
add = (a: Float, b: Float) -> Float { a +. b }

/// `a -. b`, as a function.
subtract = (a: Float, b: Float) -> Float { a -. b }

/// `a *. b`, as a function.
multiply = (a: Float, b: Float) -> Float { a *. b }

/// `e` to the power `x`. A result too big for a Float gives `Error(Nil)`;
/// that is for `x` greater than about 709.78.
///
/// ```i
/// float.exponential(1.0)  // Ok(2.718281828459045)
/// ```
exponential = (x: Float) -> Result(Float, Nil) {
	{ x >. 7.09782712893383973096e2 } |>
		(True) { Error(Nil) }
		| (False) {
			{ x <. -.7.4513321910194110842e2 } |>
				(True) { Ok(0.0) }
				| (False) {
					{ x == 1.0 } |>
						(True) { Ok(2.71828182845904523536) }
						| (False) { Ok(exp_finite(x)) }
				}
		}
}

/// The natural logarithm (base `e`). `0.0` and negative numbers give
/// `Error(Nil)`.
///
/// ```i
/// float.logarithm(1.0)  // Ok(0.0)
/// float.logarithm(0.0)  // Error(Nil)
/// ```
logarithm = (x: Float) -> Result(Float, Nil) {
	{ x <=. 0.0 } |> (True) { Error(Nil) } | (False) { Ok(log_positive(x)) }
}

/// `base` to the power `of`, for any Float exponent. A negative base with
/// an exponent that is not a whole number, `0.0` with a negative exponent,
/// and a result too big for a Float give `Error(Nil)`. The result is within
/// one unit in the last place of the exact power.
///
/// ```i
/// float.real_power(16.0, of: 0.5)  // Ok(4.0)
/// float.real_power(-.8.0, of: 1.0 /. 3.0)  // Error(Nil)
/// ```
real_power = (base: Float, of: Float) -> Result(Float, Nil) {
	fractional = core.ceiling(of) -. of >. 0.0
	{ { base <. 0.0 && fractional } || { base == 0.0 && of <. 0.0 } } |>
		(True) { Error(Nil) }
		| (False) { pow(base, of) }
}

// The algorithms of `exponential`, `logarithm`, and `real_power` are those
// of OpenLibm's e_exp.c, e_log.c, and e_pow.c (fdlibm), with their range
// reductions and polynomial coefficients:
//
// Copyright (C) 1993, 2004 by Sun Microsystems, Inc. All rights reserved.
// Developed at SunSoft, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this software is freely
// granted, provided that this notice is preserved.
word = 4294967296

mantissa_unit = 4503599627370496

ln2_hi = 6.9314718036912381649e-1

ln2_lo = 1.90821492927058770002e-10

inverse_ln2 = 1.442695040888963387

bits_of = (x: Float) -> Int { core.float_to_bits(x) }

from_bits = (x: Int) -> Float { core.float_from_bits(x) }

abs_bits = (x: Float) -> Float { from_bits(bits_of(x) % sign_mask) }

high = (x: Float) -> Int { bits_of(x) / word }

positive_high = (x: Float) -> Int { high(x) % 0x8000_0000 }

with_high = (x: Float, high_word: Int) -> Float {
	from_bits(high_word * word + bits_of(x) % word)
}

without_low = (x: Float) -> Float { from_bits(bits_of(x) / word * word) }

two_power = (e: Int) -> Float { from_bits({ e + 1023 } * mantissa_unit) }

// `value` × 2^`e`, for -1075 <= e <= 1024. The scale for subnormal results
// is split, so that it is not rounded to zero before the multiplication.
scale = (value: Float, e: Int) -> Float {
	{ e > 1023 } |>
		(True) { value *. 2.0 *. two_power(e - 1) }
		| (False) {
			{ e < -1021 } |>
				(True) {
					value *. two_power(e + 1000) *. 9.3326361850321887899e-302
				}
				| (False) { value *. two_power(e) }
		}
}

exp_polynomial = (x: Float) -> Float {
	square = x *. x
	x -.
		square *.
			{
				1.66666666666666019037e-1 +.
					square *.
						{
							-.2.77777777770155933842e-3 +.
								square *.
									{
										6.61375632143793436117e-5 +.
											square *.
												{
													-.1.6533902205465251539e-6 +.
														square *.
															4.13813679705723846039e-8
												}
									}
						}
			}
}

exp_finite = (value: Float) -> Float {
	hx = positive_high(value)
	{ hx < 0x3e30_0000 } |>
		(True) { 1.0 +. value }
		| (False) {
			(e, hi, lo) = { hx > 0x3fd6_2e42 } |>
				(False) { (0, value, 0.0) }
				| (True) {
					direction = { value <. 0.0 } |>
						(True) { -.1.0 }
						| (False) { 1.0 }
					{ hx < 0x3ff0_a2b2 } |>
						(True) {
							(
								core.truncate(direction),
								value -. direction *. ln2_hi,
								direction *. ln2_lo,
							)
						}
						| (False) {
							k = core.truncate(
								inverse_ln2 *. value +. direction *. 0.5,
							)
							kf = core.to_float(k)
							(k, value -. kf *. ln2_hi, kf *. ln2_lo)
						}
				}
			x = hi -. lo
			c = exp_polynomial(x)
			e |>
				(0) { 1.0 -. { x *. c /. { c -. 2.0 } -. x } }
				| (k) {
					scale(1.0 -. { { lo -. x *. c /. { 2.0 -. c } } -. hi }, k)
				}
		}
}

log_positive = (input: Float) -> Float {
	(value, correction) = { positive_high(input) < 0x0010_0000 } |>
		(True) { (input *. 1.8014398509481984e16, -54) }
		| (False) { (input, 0) }
	high_word = high(value)
	fraction = high_word % 0x0010_0000
	half = { fraction + 0x0009_5f64 >= 0x0010_0000 } |>
		(True) { 1 }
		| (False) { 0 }
	e = correction + high_word / 0x0010_0000 - 1023 + half
	x = with_high(value, fraction + 0x3ff0_0000 - half * 0x0010_0000)
	f = x -. 1.0
	k = core.to_float(e)
	{ { fraction + 2 } % 0x0010_0000 < 3 } |>
		(True) {
			{ f == 0.0 } |>
				(True) { k *. ln2_hi +. k *. ln2_lo }
				| (False) {
					r = f *. f *. { 0.5 -. 0.33333333333333333 *. f }
					e |>
						(0) { f -. r }
						| (_) { k *. ln2_hi -. { { r -. k *. ln2_lo } -. f } }
				}
		}
		| (False) {
			s = f /. { 2.0 +. f }
			z = s *. s
			w = z *. z
			t1 = w *.
				{
					3.999999999940941908e-1 +.
						w *.
							{
								2.222219843214978396e-1 +.
									w *. 1.531383769920937332e-1
							}
				}
			t2 = z *.
				{
					6.66666666666673513e-1 +.
						w *.
							{
								2.857142874366239149e-1 +.
									w *.
										{
											1.818357216161805012e-1 +.
												w *. 1.479819860511658591e-1
										}
							}
				}
			r = t1 +. t2
			{ fraction >= 0x0006_147a && fraction <= 0x0006_b851 } |>
				(True) {
					half_square = 0.5 *. f *. f
					e |>
						(0) {
							f -. { half_square -. s *. { half_square +. r } }
						}
						| (_) {
							k *. ln2_hi -.
								{
									{
										half_square -.
											{
												s *. { half_square +. r } +.
													k *. ln2_lo
											}
									} -.
										f
								}
						}
				}
				| (False) {
					e |>
						(0) { f -. s *. { f -. r } }
						| (_) {
							k *. ln2_hi -.
								{ { s *. { f -. r } -. k *. ln2_lo } -. f }
						}
				}
		}
}

// The high and low halves of log2(x), which keep their precision when they
// are multiplied by a large exponent. This matters for bases near one.
log2_split = (input: Float) -> (Float, Float) {
	(value, correction) = { high(input) < 0x0010_0000 } |>
		(True) { (input *. 9007199254740992.0, -53) }
		| (False) { (input, 0) }
	hx0 = high(value)
	fraction = hx0 % 0x0010_0000
	n0 = correction + hx0 / 0x0010_0000 - 1023
	(interval, n, hx) = { fraction <= 0x0003_988e } |>
		(True) { (0, n0, fraction + 0x3ff0_0000) }
		| (False) {
			{ fraction < 0x000b_b67a } |>
				(True) { (1, n0, fraction + 0x3ff0_0000) }
				| (False) { (0, n0 + 1, fraction + 0x3fe0_0000) }
		}
	x = with_high(value, hx)
	(bp, dp_hi, dp_lo) = interval |>
		(0) { (1.0, 0.0, 0.0) }
		| (_) { (1.5, 5.84962487220764160156e-1, 1.35003920212974897128e-8) }
	u = x -. bp
	v = 1.0 /. { x +. bp }
	ss = u *. v
	s_hi = without_low(ss)
	t_hi = from_bits(
		{ hx / 2 + 0x2000_0000 + 0x0008_0000 + interval * 0x0004_0000 } * word,
	)
	t_lo = x -. { t_hi -. bp }
	s_lo = v *. { { u -. s_hi *. t_hi } -. s_hi *. t_lo }
	s2 = ss *. ss
	r0 = s2 *.
		s2 *.
		{
			5.99999999999994648725e-1 +.
				s2 *.
					{
						4.28571428578550184252e-1 +.
							s2 *.
								{
									3.33333329818377432918e-1 +.
										s2 *.
											{
												2.72728123808534006489e-1 +.
													s2 *.
														{
															2.30660745775561754067e-1 +.
																s2 *.
																	2.06975017800338417784e-1
														}
											}
								}
					}
		}
	r = r0 +. s_lo *. { s_hi +. ss }
	s2h = s_hi *. s_hi
	t_hi2 = without_low(3.0 +. s2h +. r)
	t_lo2 = r -. { { t_hi2 -. 3.0 } -. s2h }
	u2 = s_hi *. t_hi2
	v2 = s_lo *. t_hi2 +. t_lo2 *. ss
	p_hi = without_low(u2 +. v2)
	p_lo = v2 -. { p_hi -. u2 }
	z_hi = 9.61796700954437255859e-1 *. p_hi
	z_lo = -.7.02846165095275826516e-9 *. p_hi +.
		p_lo *. 9.61796693925975554329e-1 +.
		dp_lo
	nf = core.to_float(n)
	hi = without_low({ { z_hi +. z_lo } +. dp_hi } +. nf)
	lo = z_lo -. { { { hi -. nf } -. dp_hi } -. z_hi }
	(hi, lo)
}

log2_close_to_one = (x: Float) -> (Float, Float) {
	t = x -. 1.0
	w = t *. t *. { 0.5 -. t *. { 0.3333333333333333333333 -. t *. 0.25 } }
	u = 1.44269502162933349609 *. t
	v = t *. 1.92596299112661746887e-8 -. w *. inverse_ln2
	hi = without_low(u +. v)
	(hi, v -. { hi -. u })
}

// `base` to the power `exponent`. A negative base has a whole exponent.
pow = (base: Float, exponent: Float) -> Result(Float, Nil) {
	exponent |>
		(0.0) { Ok(1.0) }
		| (1.0) { Ok(base) }
		| (_) {
			negative = bits_of(base) >= sign_mask
			odd = negative && is_odd_whole(exponent)
			direction = odd |> (True) { -.1.0 } | (False) { 1.0 }
			x = abs_bits(base)
			ix = high(x)
			iy = positive_high(exponent)
			{ x == 0.0 } |>
				(True) { Ok(direction *. 0.0) }
				| (False) {
					{ x == 1.0 } |>
						(True) { Ok(direction) }
						| (False) {
							{ iy > 0x41e0_0000 } |>
								(True) {
									{
										iy > 0x43f0_0000 ||
											ix < 0x3fef_ffff ||
											ix > 0x3ff0_0000
									} |>
										(True) {
											pow_extreme(x, exponent, direction)
										}
										| (False) {
											(hi, lo) = log2_close_to_one(x)
											pow_exp2(
												exponent,
												hi,
												lo,
												direction,
											)
										}
								}
								| (False) {
									(hi, lo) = log2_split(x)
									pow_exp2(exponent, hi, lo, direction)
								}
						}
				}
		}
}

// `True` when a whole Float is odd. Floats of 2^53 or more are even.
is_odd_whole = (x: Float) -> Bool {
	y = abs(x)
	{ y >=. 9007199254740992.0 } |>
		(True) { False }
		| (False) { core.truncate(y) % 2 != 0 }
}

pow_extreme = (base: Float, exponent: Float, direction: Float) -> Result(
	Float,
	Nil,
) {
	{
		{ base >. 1.0 && exponent >. 0.0 } || { base <. 1.0 && exponent <. 0.0 }
	} |>
		(True) { Error(Nil) }
		| (False) { Ok(direction *. 0.0) }
}

pow_exp2 = (
	exponent: Float,
	log_hi: Float,
	log_lo: Float,
	direction: Float
) -> Result(Float, Nil) {
	y_hi = without_low(exponent)
	p_lo = { exponent -. y_hi } *. log_hi +. exponent *. log_lo
	p_hi0 = y_hi *. log_hi
	z0 = p_hi0 +. p_lo
	{
		z0 >. 1024.0 ||
			{ z0 == 1024.0 && p_lo +. 8.0085662595372944372e-17 >. z0 -. p_hi0 }
	} |>
		(True) { Error(Nil) }
		| (False) {
			{ z0 <. -.1075.0 || { z0 == -.1075.0 && p_lo <=. z0 -. p_hi0 } } |>
				(True) { Ok(direction *. 0.0) }
				| (False) {
					n = { z0 >. 0.5 } |>
						(True) { core.truncate(z0 +. 0.5) }
						| (False) {
							{ z0 <. -.0.5 } |>
								(True) { core.truncate(z0 -. 0.5) }
								| (False) { 0 }
						}
					p_hi = p_hi0 -. core.to_float(n)
					t = without_low(p_lo +. p_hi)
					u = t *. 6.93147182464599609375e-1
					v = { p_lo -. { t -. p_hi } } *.
						6.93147180559945286227e-1 +.
						t *. -.1.90465429995776804525e-9
					z = u +. v
					w = v -. { z -. u }
					t1 = exp_polynomial(z)
					r = z *. t1 /. { t1 -. 2.0 } -. { w +. z *. w }
					Ok(direction *. scale(1.0 -. { r -. z }, n))
				}
		}
}

/// Correctly rounded real pi; not an exact reduction period.
pi = 3.141592653589793

/// Correctly rounded real 2*pi.
tau = 6.283185307179586

/// Tests the sign bit, distinguishing the two zeros.
sign_bit = (x: Float) -> Bool { core.float_to_bits(x) >= sign_mask }

/// Copies only the sign bit, preserving all other magnitude bits.
copy_sign = (magnitude: Float, sign: Float) -> Float {
	core.float_from_bits(
		core.bit_and(core.float_to_bits(magnitude), sign_mask - 1) +
			{ sign_bit(sign) |> (True) { sign_mask } | (False) { 0 } },
	)
}

// All finite inputs are integer multiples of 2^-1074. This common scale avoids
// rational normalization, including for extreme atan2 ratios and exact norms.
dyadic = (x: Float) -> Int {
	bits = core.float_to_bits(x)
	fraction = core.bit_and(bits, fraction_unit - 1)
	exponent = core.bit_and(core.shift_right(bits, 52), 2047)
	magnitude = { exponent == 0 } |>
		(True) { fraction }
		| (False) { core.shift_left(fraction_unit + fraction, exponent - 1) }
	sign_bit(x) |> (True) { -magnitude } | (False) { magnitude }
}

int_abs = (n: Int) -> Int { { n < 0 } |> (True) { -n } | (False) { n } }

int_min = (a: Int, b: Int) -> Int { { a < b } |> (True) { a } | (False) { b } }

int_max = (a: Int, b: Int) -> Int { { a > b } |> (True) { a } | (False) { b } }

// Denominator is positive. Int / truncates; directed division must not.
down = (n: Int, d: Int) -> Int {
	n / d - { { n < 0 && n % d != 0 } |> (True) { 1 } | (False) { 0 } }
}

up = (n: Int, d: Int) -> Int { -down(-n, d) }

Fixed = (lo = Int, hi = Int)

fixed_ratio = (n: Int, d: Int, p: Int) -> Fixed {
	scaled = core.shift_left(n, p)
	Fixed(lo = down(scaled, d), hi = up(scaled, d))
}

fixed_float = (x: Float, p: Int) -> Fixed {
	n = dyadic(x)
	{ p >= 1074 } |>
		(True) {
			v = core.shift_left(n, p - 1074)
			Fixed(lo = v, hi = v)
		}
		| (False) {
			d = core.shift_left(1, 1074 - p)
			Fixed(lo = down(n, d), hi = up(n, d))
		}
}

fixed_add = (a: Fixed, b: Fixed) -> Fixed {
	Fixed(lo = a.lo + b.lo, hi = a.hi + b.hi)
}

fixed_neg = (a: Fixed) -> Fixed { Fixed(lo = -a.hi, hi = -a.lo) }

fixed_sub = (a: Fixed, b: Fixed) -> Fixed { fixed_add(a, fixed_neg(b)) }

fixed_scale = (a: Fixed, k: Int) -> Fixed {
	{ k >= 0 } |>
		(True) { Fixed(lo = a.lo * k, hi = a.hi * k) }
		| (False) { Fixed(lo = a.hi * k, hi = a.lo * k) }
}

fixed_div_int = (a: Fixed, d: Int) -> Fixed {
	Fixed(lo = down(a.lo, d), hi = up(a.hi, d))
}

fixed_mul = (a: Fixed, b: Fixed, p: Int) -> Fixed {
	aa = a.lo * b.lo
	ab = a.lo * b.hi
	ba = a.hi * b.lo
	bb = a.hi * b.hi
	low = int_min(int_min(aa, ab), int_min(ba, bb))
	high = int_max(int_max(aa, ab), int_max(ba, bb))
	Fixed(lo = core.shift_right(low, p), hi = -core.shift_right(-high, p))
}

fixed_div = (a: Fixed, b: Fixed, p: Int) -> Option(Fixed) {
	{ b.lo <= 0 && b.hi >= 0 } |>
		(True) { None }
		| (False) {
			{ b.hi < 0 } |>
				(True) { fixed_div(fixed_neg(a), fixed_neg(b), p) }
				| (False) {
					aa = fixed_ratio(a.lo, b.lo, p)
					ab = fixed_ratio(a.lo, b.hi, p)
					ba = fixed_ratio(a.hi, b.lo, p)
					bb = fixed_ratio(a.hi, b.hi, p)
					Some(
						Fixed(
							lo = int_min(
								int_min(aa.lo, ab.lo),
								int_min(ba.lo, bb.lo),
							),
							hi = int_max(
								int_max(aa.hi, ab.hi),
								int_max(ba.hi, bb.hi),
							),
						),
					)
				}
		}
}

fixed_radius = (a: Fixed) -> Int { int_max(int_abs(a.lo), int_abs(a.hi)) }

fixed_widen = (a: Fixed, radius: Int) -> Fixed {
	Fixed(lo = a.lo - radius, hi = a.hi + radius)
}

// Machin terms are divided individually with directed rounding. The first
// omitted term bounds each alternating remainder. No denominators accumulate.
reciprocal_atan = (k: Int, p: Int) -> Fixed {
	scale = core.shift_left(1, p)
	go = (power: Int, index: Int, sum: Fixed, positive: Bool) -> Fixed {
		denominator = power * { 2 * index + 1 }
		{ denominator > scale } |>
			(True) { fixed_widen(sum, 1) }
			| (False) {
				term = Fixed(
					lo = scale / denominator,
					hi = up(scale, denominator),
				)
				next = positive |>
					(True) { fixed_add(sum, term) }
					| (False) { fixed_sub(sum, term) }
				go(power * k * k, index + 1, next, core.not(positive))
			}
	}
	go(k, 0, Fixed(lo = 0, hi = 0), True)
}

fixed_pi = (p: Int) -> Fixed {
	fixed_sub(
		fixed_scale(reciprocal_atan(5, p), 16),
		fixed_scale(reciprocal_atan(239, p), 4),
	)
}

// |z| <= 1/2. All recurrences enclose their exact terms; at the first next
// term whose power bound is <= one unit, the remaining alternating tail is
// <= one unit too. Width from arithmetic is retained, never guessed away.
small_atan = (z: Fixed, p: Int) -> Fixed {
	square = fixed_mul(z, z, p)
	go = (power: Fixed, index: Int, sum: Fixed) -> Fixed {
		term = fixed_div_int(power, 2 * index + 1)
		next_sum = fixed_add(sum, term)
		next = fixed_neg(fixed_mul(power, square, p))
		{ fixed_radius(next) <= 2 } |>
			(True) { fixed_widen(next_sum, 2) }
			| (False) { go(next, index + 1, next_sum) }
	}
	go(z, 0, Fixed(lo = 0, hi = 0))
}

// z is nonnegative; interval extension of exact inverse/addition identities.
positive_atan = (z: Fixed, pi_bounds: () -> Fixed, p: Int) -> Fixed {
	scale = core.shift_left(1, p)
	one = Fixed(lo = scale, hi = scale)
	{ z.lo >= scale } |>
		(True) {
			fixed_div(one, z, p) |>
				(Some(inverse)) {
					fixed_sub(
						fixed_div_int(pi_bounds(), 2),
						positive_atan_small(inverse, p),
					)
				}
				| (None) { Fixed(lo = 0, hi = pi_bounds().hi) }
		}
		| (False) { positive_atan_small(z, p) }
}

positive_atan_small = (z: Fixed, p: Int) -> Fixed {
	scale = core.shift_left(1, p)
	{ z.hi <= scale / 2 } |>
		(True) { small_atan(z, p) }
		| (False) {
			half = Fixed(lo = scale / 2, hi = scale / 2)
			denominator = fixed_add(
				Fixed(lo = scale, hi = scale),
				fixed_div_int(z, 2),
			)
			fixed_div(fixed_sub(z, half), denominator, p) |>
				(Some(reduced)) {
					fixed_add(small_atan(half, p), small_atan(reduced, p))
				}
				| (None) { Fixed(lo = -scale, hi = scale) }
		}
}

// Reduction gets p+exponent+guard bits, then outward rescaling. If an integer
// quadrant cannot yet be selected uniquely, double the reduction precision.
reduce_angle = (x: Float, p: Int, work: Int) -> (Int, Fixed) {
	pi_bounds = fixed_pi(work)
	half_pi = fixed_div_int(pi_bounds, 2)
	fixed_div(fixed_float(x, work), half_pi, work) |>
		(None) { reduce_angle(x, p, work * 2) }
		| (Some(quotient)) {
			scale = core.shift_left(1, work)
			qlo = down(quotient.lo + scale / 2, scale)
			qhi = down(quotient.hi + scale / 2, scale)
			r = fixed_sub(fixed_float(x, work), fixed_scale(half_pi, qlo))
			{
				qlo == qhi &&
					r.hi - r.lo <= core.shift_left(1, work - p) &&
					fixed_radius(r) < scale
			} |>
				(False) { reduce_angle(x, p, work * 2) }
				| (True) {
					shift = work - p
					(
						qlo,
						Fixed(
							lo = core.shift_right(r.lo, shift),
							hi = -core.shift_right(-r.hi, shift),
						),
					)
				}
		}
}

// On |r|<1 terms alternate and decrease. Interval recurrence encloses the
// exact Taylor sum; two units bound the first omitted term and its tail.
fixed_sincos = (r: Fixed, cosine: Bool, p: Int) -> Fixed {
	square = fixed_mul(r, r, p)
	scale = core.shift_left(1, p)
	initial = cosine |> (True) { Fixed(lo = scale, hi = scale) } | (False) { r }
	go = (term: Fixed, k: Int, sum: Fixed) -> Fixed {
		next_sum = fixed_add(sum, term)
		d = cosine |>
			(True) { { 2 * k + 1 } * { 2 * k + 2 } }
			| (False) { { 2 * k + 2 } * { 2 * k + 3 } }
		next = fixed_div_int(
			fixed_neg(fixed_mul(term, square, p)),
			d,
		)
		{ fixed_radius(next) <= 2 } |>
			(True) { fixed_widen(next_sum, 2) }
			| (False) { go(next, k + 1, next_sum) }
	}
	go(initial, 0, Fixed(lo = 0, hi = 0))
}

trig_bounds = (x: Float, p: Int) -> (Fixed, Fixed) {
	magnitude = core.bit_and(core.float_to_bits(x), sign_mask - 1)
	exponent = core.shift_right(magnitude, 52) - 1023
	(q, r) = reduce_angle(x, p, p + int_max(0, exponent) + 32)
	s = fixed_sincos(r, False, p)
	c = fixed_sincos(r, True, p)
	{ { q % 4 } + 4 } % 4 |>
		(0) { (s, c) }
		| (1) { (c, fixed_neg(s)) }
		| (2) { (fixed_neg(s), fixed_neg(c)) }
		| (_) { (fixed_neg(c), s) }
}

integer_sqrt = (n: Int) -> Int {
	{ n == 0 } |>
		(True) { 0 }
		| (False) {
			initial = core.shift_left(1, { radix_order(n, 1, 2) + 2 } / 2)
			go = (x: Int) -> Int {
				next = { x + n / x } / 2
				{ next >= x } |> (True) { x } | (False) { go(next) }
			}
			go(initial)
		}
}

fixed_sqrt_ratio = (n: Int, d: Int, p: Int) -> Fixed {
	scaled = core.shift_left(n, 2 * p)
	k = integer_sqrt(scaled / d)
	Fixed(
		lo = k,
		hi = { k * k * d == scaled } |> (True) { k } | (False) { k + 1 },
	)
}

signed_bits = (n: Int, p: Int) -> Result(Int, Nil) {
	{ n == 0 } |>
		(True) { Ok(0) }
		| (False) {
			ratio_bits(int_abs(n), core.shift_left(1, p)) |>
				(Error(_)) { Error(Nil) }
				| (Ok(bits)) {
					Ok(
						bits +
							{
								{ n < 0 } |>
									(True) { sign_mask }
									| (False) { 0 }
							},
					)
				}
		}
}

certify = (bounds: Fixed, p: Int) -> Option(Result(Float, Nil)) {
	(signed_bits(bounds.lo, p), signed_bits(bounds.hi, p)) |>
		((Ok(a), Ok(b)) ? a == b) { Some(Ok(core.float_from_bits(a))) }
		| ((Error(_), Error(_)) ? bounds.lo > 0 || bounds.hi < 0) {
			Some(Error(Nil))
		}
		| (_) { None }
}

// Error-free sum/product on the bounded normal operands used below. No fused
// operations; the explicit evaluation order is the same in every Wasm engine.
Double = (hi = Float, lo = Float)

two_sum = (a: Float, b: Float) -> Double {
	s = a +. b
	v = s -. a
	Double(hi = s, lo = { a -. { s -. v } } +. { b -. v })
}

two_product = (a: Float, b: Float) -> Double {
	product = a *. b
	split_a = splitter *. a
	ah = split_a -. { split_a -. a }
	al = a -. ah
	split_b = splitter *. b
	bh = split_b -. { split_b -. b }
	bl = b -. bh
	error = { { { ah *. bh } -. product } +. { ah *. bl } +. { al *. bh } } +.
		{ al *. bl }
	Double(hi = product, lo = error)
}

double_add = (a: Double, b: Double) -> Double {
	high = two_sum(a.hi, b.hi)
	two_sum(high.hi, high.lo +. a.lo +. b.lo)
}

double_neg = (a: Double) -> Double { Double(hi = -.a.hi, lo = -.a.lo) }

double_mul = (a: Double, b: Double) -> Double {
	high = two_product(a.hi, b.hi)
	two_sum(high.hi, high.lo +. { a.hi *. b.lo } +. { a.lo *. b.hi })
}

double_div = (a: Double, d: Float) -> Double {
	q = a.hi /. d
	product = two_product(q, d)
	remainder = { { a.hi -. product.hi } -. product.lo } +. a.lo
	two_sum(q, remainder /. d)
}

// Fast arithmetic is restricted to bounded normal regions. Each returned pair
// encloses its real target within the documented error allowance; ambiguity
// always falls back to growing fixed-point intervals.
splitter = 134217729.0

rounding_radius = 2.6469779601696886e-23

reduction_radius = 1.0097419586828951e-28

fast_zero = 0.0

fast_one = 1.0

fast_two = 2.0

fast_four = 4.0

fast_sixteen = 16.0

fast_half = 0.5

fast_three = 3.0

fast_inverse_half_pi = 0.6366197723675814

fast_angle_limit = 1048576.0

fast_reduced_limit = 0.8

fast_pole_limit = 0.00000095367431640625

fast_magnitude_max = 1.6069380442589903e60

fast_magnitude_min = 6.223015277861142e-61

fast_unit_min = 3.8725919148493183e-121

fast_unit_max = 2.5822498780869086e120

quick_sum = (a: Float, b: Float) -> Double {
	s = a +. b
	Double(hi = s, lo = b -. { s -. a })
}

double_mul_fast = (a: Double, b: Double) -> Double {
	a_high = a.hi
	a_low = a.lo
	b_high = b.hi
	b_low = b.lo
	product = a_high *. b_high
	split_a = splitter *. a_high
	ah = split_a -. { split_a -. a_high }
	al = a_high -. ah
	split_b = splitter *. b_high
	bh = split_b -. { split_b -. b_high }
	bl = b_high -. bh
	error = { { { ah *. bh } -. product } +. { ah *. bl } +. { al *. bh } } +.
		{ al *. bl }
	quick_sum(product, error +. { a_high *. b_low } +. { a_low *. b_high })
}

// Coefficients dominate products by at least two. Load immutable fields once;
// generic record field access otherwise repeats casts and checked array reads.
double_add_coefficient = (value: Double, coefficient: Double) -> Double {
	coefficient_high = coefficient.hi
	value_high = value.hi
	high = coefficient_high +. value_high
	low = value_high -. { high -. coefficient_high }
	quick_sum(high, low +. coefficient.lo +. value.lo)
}

double_quotient = (a: Double, b: Double) -> Double {
	q = a.hi /. b.hi
	residual = double_add(
		a,
		double_neg(double_mul_fast(Double(hi = q, lo = fast_zero), b)),
	)
	two_sum(q, { residual.hi +. residual.lo } /. b.hi)
}

double_root = (x: Double) -> Double {
	root = core.sqrt(x.hi)
	residual = double_add(
		x,
		double_neg(two_product(root, root)),
	)
	two_sum(root, { residual.hi +. residual.lo } /. { fast_two *. root })
}

// In the fast regions the endpoints are normal and nonzero, so IEEE equality
// implies identical bits. This avoids constructing arbitrary-precision bit
// integers merely to compare two normal Float values. Zero ambiguity refines.
fast_round = (value: Double, absolute_radius: Float) -> Option(Float) {
	radius = abs(value.hi) *. rounding_radius +. absolute_radius
	low = value.hi +. { value.lo -. radius }
	high = value.hi +. { value.lo +. radius }
	{ low == high && low != fast_zero } |>
		(True) { Some(low) }
		| (False) { None }
}

Reduced = (q = Float, quadrant = Float, argument = Double)

fast_reduce = (x: Float) -> Option(Reduced) {
	{ abs(x) >. fast_angle_limit } |>
		(True) { None }
		| (False) {
			q = core.floor(x *. fast_inverse_half_pi +. fast_half)
			argument = double_add(
				Double(hi = x, lo = fast_zero),
				double_neg(
					double_mul_fast(
						Double(hi = q, lo = fast_zero),
						half_pi_pair,
					),
				),
			)
			{ abs(argument.hi) >. fast_reduced_limit } |>
				(True) { None }
				| (False) {
					Some(
						Reduced(
							q = q,
							quadrant = q -.
								fast_four *. core.floor(q /. fast_four),
							argument = argument,
						),
					)
				}
		}
}

reduced_value = (reduced: Reduced, cosine: Bool) -> Double {
	r = reduced.argument
	square = double_mul_fast(r, r)
	k = reduced.quadrant
	use_cosine = cosine |>
		(True) { k == fast_zero || k == fast_two }
		| (False) { k == fast_one || k == fast_three }
	value = use_cosine |>
		(True) { cosine_polynomial(square) }
		| (False) { double_mul_fast(r, sine_polynomial(square)) }
	negative = cosine |>
		(True) { k == fast_one || k == fast_two }
		| (False) { k == fast_two || k == fast_three }
	negative |> (True) { double_neg(value) } | (False) { value }
}

fast_sincos = (x: Float, cosine: Bool) -> Option(Float) {
	fast_reduce(x) |>
		(None) { None }
		| (Some(reduced)) {
			fast_round(
				reduced_value(reduced, cosine),
				abs(reduced.q) *. reduction_radius,
			)
		}
}

fast_tangent = (x: Float) -> Option(Float) {
	fast_reduce(x) |>
		(None) { None }
		| (Some(reduced)) {
			s = reduced_value(reduced, False)
			c = reduced_value(reduced, True)
			{ abs(c.hi) <. fast_pole_limit } |>
				(True) { None }
				| (False) {
					fast_round(
						double_quotient(s, c),
						{ abs(reduced.q) *. reduction_radius } /.
							{ fast_half *. c.hi *. c.hi },
					)
				}
		}
}

// The table center is an exactly represented multiple of 1/16. The reduced
// argument has magnitude <=1/32 (plus rounding slack), so nine atan terms
// suffice; the first omitted term is <=|r|^19/19.
fast_atan_unit = (z: Double) -> Double {
	index = core.floor(z.hi *. fast_sixteen +. fast_half)
	center = index /. fast_sixteen
	base = index |>
		(0.0) { atan_point_0 }
		| (1.0) { atan_point_1 }
		| (2.0) { atan_point_2 }
		| (3.0) { atan_point_3 }
		| (4.0) { atan_point_4 }
		| (5.0) { atan_point_5 }
		| (6.0) { atan_point_6 }
		| (7.0) { atan_point_7 }
		| (8.0) { atan_point_8 }
		| (9.0) { atan_point_9 }
		| (10.0) { atan_point_10 }
		| (11.0) { atan_point_11 }
		| (12.0) { atan_point_12 }
		| (13.0) { atan_point_13 }
		| (14.0) { atan_point_14 }
		| (15.0) { atan_point_15 }
		| (_) { atan_point_16 }
	numerator = double_add(z, Double(hi = -.center, lo = fast_zero))
	denominator = double_add(
		Double(hi = fast_one, lo = fast_zero),
		double_mul_fast(Double(hi = center, lo = fast_zero), z),
	)
	r = double_quotient(numerator, denominator)
	correction = double_mul_fast(
		r,
		atan_polynomial(double_mul_fast(r, r)),
	)
	double_add(base, correction)
}

positive_angle = (y: Double, x: Double) -> Double {
	{ y.hi >. x.hi } |>
		(True) {
			double_add(
				half_pi_pair,
				double_neg(
					fast_atan_unit(double_quotient(x, y)),
				),
			)
		}
		| (False) { fast_atan_unit(double_quotient(y, x)) }
}

fast_arctangent = (x: Float) -> Option(Float) {
	a = positive_angle(
		Double(hi = abs(x), lo = fast_zero),
		Double(hi = fast_one, lo = fast_zero),
	)
	signed = { x <. fast_zero } |> (True) { double_neg(a) } | (False) { a }
	fast_round(signed, fast_zero)
}

fast_angle = (y: Float, x: Float) -> Option(Float) {
	largest = max(abs(x), abs(y))
	smallest = min(abs(x), abs(y))
	{ largest >. fast_magnitude_max || smallest <. fast_magnitude_min } |>
		(True) { None }
		| (False) {
			angle = positive_angle(
				Double(hi = abs(y), lo = fast_zero),
				Double(hi = abs(x), lo = fast_zero),
			)
			quadrant = { x <. fast_zero } |>
				(True) { double_add(pi_pair, double_neg(angle)) }
				| (False) { angle }
			signed = { y <. fast_zero } |>
				(True) { double_neg(quadrant) }
				| (False) { quadrant }
			fast_round(signed, fast_zero)
		}
}

fast_inverse = (x: Float, cosine: Bool) -> Option(Float) {
	lower = two_sum(fast_one, -.x)
	upper = two_sum(fast_one, x)
	root = double_root(double_mul_fast(lower, upper))
	magnitude = Double(hi = abs(x), lo = fast_zero)
	angle = cosine |>
		(True) { positive_angle(root, magnitude) }
		| (False) { positive_angle(magnitude, root) }
	signed = { x <. fast_zero } |>
		(True) {
			cosine |>
				(True) { double_add(pi_pair, double_neg(angle)) }
				| (False) { double_neg(angle) }
		}
		| (False) { angle }
	fast_round(signed, fast_zero)
}

fast_norm = (x: Float, y: Float) -> Option(Float) {
	largest = max(abs(x), abs(y))
	{ largest <. fast_magnitude_min || largest >. fast_magnitude_max } |>
		(True) { None }
		| (False) {
			fast_round(
				double_root(
					double_add(two_product(x, x), two_product(y, y)),
				),
				fast_zero,
			)
		}
}

fast_units = (x: Float, factor: Double) -> Option(Float) {
	{ abs(x) <. fast_unit_min || abs(x) >. fast_unit_max } |>
		(True) { None }
		| (False) {
			fast_round(
				double_mul_fast(Double(hi = x, lo = fast_zero), factor),
				fast_zero,
			)
		}
}

Sine = ()
Cosine = ()
Tangent = ()
Arcsine = ()
Arccosine = ()
Arctangent = ()
Angle = ()
UnitsIn = ()
UnitsOut = ()
// Operation codes are private ordinary values, shared by the refinement loop.
Math = Sine
	| Cosine
	| Tangent
	| Arcsine
	| Arccosine
	| Arctangent
	| Angle
	| UnitsIn
	| UnitsOut

math_bounds = (operation: Math, x: Float, y: Float, p: Int) -> Option(Fixed) {
	scale = core.shift_left(1, p)
	common = core.shift_left(1, 1074)
	operation |>
		(Sine) { Some(trig_bounds(x, p).0) }
		| (Cosine) { Some(trig_bounds(x, p).1) }
		| (Tangent) {
			(s, c) = trig_bounds(x, p)
			fixed_div(s, c, p)
		}
		| (_) {
			pi_bounds = () { fixed_pi(p) }
			operation |>
				(Arctangent) {
					a = positive_atan(
						fixed_ratio(
							int_abs(dyadic(x)),
							common,
							p,
						),
						pi_bounds,
						p,
					)
					Some(sign_bit(x) |> (True) { fixed_neg(a) } | (False) { a })
				}
				| (Angle) {
					a = positive_atan(
						fixed_ratio(
							int_abs(dyadic(y)),
							int_abs(dyadic(x)),
							p,
						),
						pi_bounds,
						p,
					)
					quadrant = { x <. 0.0 } |>
						(True) { fixed_sub(pi_bounds(), a) }
						| (False) { a }
					Some(
						sign_bit(y) |>
							(True) { fixed_neg(quadrant) }
							| (False) { quadrant },
					)
				}
				| (Arcsine) {
					dx = dyadic(x)
					root = fixed_sqrt_ratio(
						common * common - dx * dx,
						common * common,
						p,
					)
					fixed_div(
						fixed_float(abs(x), p),
						root,
						p,
					) |>
						(None) { None }
						| (Some(ratio)) {
							a = positive_atan(ratio, pi_bounds, p)
							Some(
								sign_bit(x) |>
									(True) { fixed_neg(a) }
									| (False) { a },
							)
						}
				}
				| (Arccosine) {
					dx = dyadic(x)
					root = fixed_sqrt_ratio(
						common * common - dx * dx,
						common * common,
						p,
					)
					fixed_div(
						fixed_float(abs(x), p),
						root,
						p,
					) |>
						(None) { None }
						| (Some(ratio)) {
							a = positive_atan(ratio, pi_bounds, p)
							half = fixed_div_int(pi_bounds(), 2)
							Some(
								sign_bit(x) |>
									(True) { fixed_add(half, a) }
									| (False) { fixed_sub(half, a) },
							)
						}
				}
				| (UnitsIn) {
					Some(
						fixed_div_int(
							fixed_mul(fixed_float(x, p), pi_bounds(), p),
							180,
						),
					)
				}
				| (UnitsOut) {
					fixed_div(
						fixed_scale(fixed_float(x, p), 180),
						pi_bounds(),
						p,
					)
				}
				| (_) { Some(Fixed(lo = -scale, hi = scale)) }
		}
}

refine_math = (operation: Math, x: Float, y: Float, p: Int) -> Result(
	Float,
	Nil,
) {
	math_bounds(operation, x, y, p) |>
		(None) { refine_math(operation, x, y, p * 2) }
		| (Some(bounds)) {
			certify(bounds, p) |>
				(Some(result)) { result }
				| (None) { refine_math(operation, x, y, p * 2) }
		}
}

// The real functions here are bounded; Error is unreachable by F3/F4. Keep
// this exhaustive rather than adding an Assert effect or constructing infinity.
unbounded_result = (result: Result(Float, Nil)) -> Float {
	result |>
		(Ok(value)) { value }
		| (Error(_)) { panic("bounded real function overflowed") }
}

/// Real sine in radians; preserves both zero signs. Correctly rounded.
sin = (x: Float) -> Float {
	{ abs(x) <=. 7.450580596923828e-9 } |>
		(True) { x }
		| (False) {
			fast_sincos(x, False) |>
				(Some(value)) { value }
				| (None) { unbounded_result(refine_math(Sine, x, 0.0, 128)) }
		}
}

/// Real cosine in radians; either zero gives +1. Correctly rounded.
cos = (x: Float) -> Float {
	{ abs(x) <=. 7.450580596923828e-9 } |>
		(True) { 1.0 }
		| (False) {
			fast_sincos(x, True) |>
				(Some(value)) { value }
				| (None) { unbounded_result(refine_math(Cosine, x, 0.0, 128)) }
		}
}

/// Real tangent, rounded once, rather than a quotient of rounded sin/cos.
/// Binary-rational angles are never exact poles; rounded overflow gives Error.
tan = (x: Float) -> Result(Float, Nil) {
	{ abs(x) <=. 7.450580596923828e-9 } |>
		(True) { Ok(x) }
		| (False) {
			fast_tangent(x) |>
				(Some(value)) { Ok(value) }
				| (None) { refine_math(Tangent, x, 0.0, 128) }
		}
}

/// Real arcsine; outside [-1,1] gives Error before arithmetic. Keeps zero sign.
asin = (x: Float) -> Result(Float, Nil) {
	{ abs(x) >. 1.0 } |>
		(True) { Error(Nil) }
		| (False) {
			{ abs(x) == 1.0 } |>
				(True) { Ok(copy_sign(pi /. 2.0, x)) }
				| (False) {
					{ abs(x) <=. 7.450580596923828e-9 } |>
						(True) { Ok(x) }
						| (False) {
							fast_inverse(x, False) |>
								(Some(value)) { Ok(value) }
								| (None) { refine_math(Arcsine, x, 0.0, 128) }
						}
				}
		}
}

/// Real arccosine; outside [-1,1] gives Error; +1 gives +0 and -1 gives pi.
acos = (x: Float) -> Result(Float, Nil) {
	{ abs(x) >. 1.0 } |>
		(True) { Error(Nil) }
		| (False) {
			x |>
				(1.0) { Ok(0.0) }
				| (-.1.0) { Ok(pi) }
				| (0.0) { Ok(pi /. 2.0) }
				| (_) {
					fast_inverse(x, True) |>
						(Some(value)) { Ok(value) }
						| (None) { refine_math(Arccosine, x, 0.0, 128) }
				}
		}
}

/// Real arctangent; preserves zero signs. Correctly rounded.
atan = (x: Float) -> Float {
	{ abs(x) <=. 7.450580596923828e-9 } |>
		(True) { x }
		| (False) {
			{ abs(x) >=. 18014398509481984.0 } |>
				(True) { copy_sign(pi /. 2.0, x) }
				| (False) {
					fast_arctangent(x) |>
						(Some(value)) { value }
						| (None) {
							unbounded_result(
								refine_math(Arctangent, x, 0.0, 128),
							)
						}
				}
		}
}

/// Argument of x+i*y, in [-pi,pi], sign from y. Signed zero axes precede
/// comparisons; nonzero ratios are exact, including extreme exponent gaps.
atan2 = (y: Float, x: Float) -> Float {
	{ y == 0.0 } |>
		(True) { sign_bit(x) |> (True) { copy_sign(pi, y) } | (False) { y } }
		| (False) {
			{ x == 0.0 } |>
				(True) { copy_sign(pi /. 2.0, y) }
				| (False) {
					fast_angle(y, x) |>
						(Some(value)) { value }
						| (None) {
							unbounded_result(refine_math(Angle, x, y, 128))
						}
				}
		}
}

/// Rounded sqrt(x*x+y*y), without intermediate Float overflow. Exact square
/// comparisons resolve midpoint ties. Both zeros give +0, independently of sign.
hypot = (x: Float, y: Float) -> Result(Float, Nil) {
	fast_norm(x, y) |> (Some(value)) { Ok(value) } | (None) { exact_norm(x, y) }
}

exact_norm = (x: Float, y: Float) -> Result(Float, Nil) {
	a = dyadic(x)
	b = dyadic(y)
	sum = a * a + b * b
	{ sum == 0 } |>
		(True) { Ok(0.0) }
		| (False) {
			common = core.shift_left(1, 1074)
			maximum = dyadic(max_finite)
			threshold = maximum + core.shift_left(1, 2044)
			{ sum >= threshold * threshold } |>
				(True) { Error(Nil) }
				| (False) {
					{ sum >= maximum * maximum } |>
						(True) { Ok(max_finite) }
						| (False) {
							k = integer_sqrt(sum)
							shift = int_max(0, radix_order(k, 1, 2) - 52)
							lower = core.shift_left(
								core.shift_right(k, shift),
								shift,
							)
							upper = lower + core.shift_left(1, shift)
							bits = ratio_bits_positive_zero(lower, common) |>
								(Ok(value)) { value }
								| (Error(_)) {
									panic("finite lower norm overflowed")
								}
							middle_square = { lower + upper } *
								{ lower + upper }
							increment = {
								4 * sum > middle_square ||
									{
										4 * sum == middle_square &&
											bits % 2 == 1
									}
							}
							Ok(
								core.float_from_bits(
									bits +
										{
											increment |>
												(True) { 1 }
												| (False) { 0 }
										},
								),
							)
						}
				}
		}
}

ratio_bits_positive_zero = (n: Int, d: Int) -> Result(Int, Nil) {
	{ n == 0 } |> (True) { Ok(0) } | (False) { ratio_bits(n, d) }
}

/// Rounded real degrees*pi/180, using real pi. Preserves the input zero sign.
radians = (degrees: Float) -> Float {
	{ degrees == 0.0 } |>
		(True) { degrees }
		| (False) {
			fast_units(degrees, radian_pair) |>
				(Some(value)) { value }
				| (None) {
					unbounded_result(refine_math(UnitsIn, degrees, 0.0, 128))
				}
		}
}

/// Rounded real radians*180/pi. Preserves zero signs; rounded overflow is Error.
degrees = (radians: Float) -> Result(Float, Nil) {
	{ radians == 0.0 } |>
		(True) { Ok(radians) }
		| (False) {
			fast_units(radians, degree_pair) |>
				(Some(value)) { Ok(value) }
				| (None) { refine_math(UnitsOut, radians, 0.0, 128) }
		}
}

// BEGIN GENERATED FAST COEFFICIENTS
// Generated by std/test/float_coefficients.py; MPFR MPFR 4.2.2.
half_pi_pair = Double(hi = 1.5707963267948966, lo = 6.123233995736766e-17)

pi_pair = Double(hi = 3.141592653589793, lo = 1.2246467991473532e-16)

radian_pair = Double(hi = 0.017453292519943295, lo = 2.9486522708701687e-19)

degree_pair = Double(hi = 57.29577951308232, lo = -.1.9878495670576283e-15)

atan_point_0 = Double(hi = 0.0, lo = -.0.0)

atan_point_1 = Double(hi = 0.06241880999595735, lo = -.1.5490756308295046e-18)

atan_point_2 = Double(hi = 0.12435499454676144, lo = -.3.1253241424539383e-18)

atan_point_3 = Double(hi = 0.18534794999569476, lo = 4.180692268843079e-18)

atan_point_4 = Double(hi = 0.24497866312686414, lo = 1.0698755618734451e-17)

atan_point_5 = Double(hi = 0.3028848683749714, lo = -.1.1010827903001369e-17)

atan_point_6 = Double(hi = 0.35877067027057225, lo = -.2.4623815582638635e-17)

atan_point_7 = Double(hi = 0.4124104415973873, lo = -.1.587652227770689e-17)

atan_point_8 = Double(hi = 0.4636476090008061, lo = 2.2698777452961687e-17)

atan_point_9 = Double(hi = 0.5123894603107377, lo = -.2.5462781472855804e-17)

atan_point_10 = Double(hi = 0.5585993153435624, lo = -.5.4556305485916264e-18)

atan_point_11 = Double(hi = 0.6022873461349642, lo = 2.950430737228402e-17)

atan_point_12 = Double(hi = 0.6435011087932844, lo = 1.5834785051444286e-17)

atan_point_13 = Double(hi = 0.6823165548747481, lo = 6.943223671560008e-18)

atan_point_14 = Double(hi = 0.7188299996216245, lo = -.2.1478388444456983e-17)

atan_point_15 = Double(hi = 0.7531512809621944, lo = -.2.4256934659182068e-17)

atan_point_16 = Double(hi = 0.7853981633974483, lo = 3.061616997868383e-17)

sine_coefficient_0 = Double(hi = 1.0, lo = 0.0)

sine_coefficient_1 = Double(
	hi = -.0.16666666666666666,
	lo = -.9.25185853854297e-18,
)

sine_coefficient_2 = Double(
	hi = 0.008333333333333333,
	lo = 1.1564823173178714e-19,
)

sine_coefficient_3 = Double(
	hi = -.0.0001984126984126984,
	lo = -.1.7209558293420705e-22,
)

sine_coefficient_4 = Double(
	hi = 2.7557319223985893e-06,
	lo = -.1.858393274046472e-22,
)

sine_coefficient_5 = Double(
	hi = -.2.505210838544172e-08,
	lo = 1.448814070935912e-24,
)

sine_coefficient_6 = Double(
	hi = 1.6059043836821613e-10,
	lo = 1.2585294588752098e-26,
)

sine_coefficient_7 = Double(
	hi = -.7.647163731819816e-13,
	lo = -.7.03872877733453e-30,
)

sine_coefficient_8 = Double(
	hi = 2.8114572543455206e-15,
	lo = 1.6508842730861433e-31,
)

sine_coefficient_9 = Double(
	hi = -.8.22063524662433e-18,
	lo = -.2.2141894119604265e-34,
)

sine_coefficient_10 = Double(
	hi = 1.9572941063391263e-20,
	lo = -.1.3643503830087908e-36,
)

sine_coefficient_11 = Double(
	hi = -.3.868170170630684e-23,
	lo = 8.843177655482344e-40,
)

sine_coefficient_12 = Double(
	hi = 6.446950284384474e-26,
	lo = -.1.9330404233703465e-42,
)

sine_coefficient_13 = Double(
	hi = -.9.183689863795546e-29,
	lo = -.1.4303150396787322e-45,
)

cosine_coefficient_0 = Double(hi = 1.0, lo = 0.0)

cosine_coefficient_1 = Double(hi = -.0.5, lo = 0.0)

cosine_coefficient_2 = Double(
	hi = 0.041666666666666664,
	lo = 2.3129646346357427e-18,
)

cosine_coefficient_3 = Double(
	hi = -.0.001388888888888889,
	lo = 5.300543954373577e-20,
)

cosine_coefficient_4 = Double(
	hi = 2.48015873015873e-05,
	lo = 2.1511947866775882e-23,
)

cosine_coefficient_5 = Double(
	hi = -.2.755731922398589e-07,
	lo = -.2.3767714622250297e-23,
)

cosine_coefficient_6 = Double(
	hi = 2.08767569878681e-09,
	lo = -.1.20734505911326e-25,
)

cosine_coefficient_7 = Double(
	hi = -.1.1470745597729725e-11,
	lo = -.2.0655512752830745e-28,
)

cosine_coefficient_8 = Double(
	hi = 4.779477332387385e-14,
	lo = 4.399205485834081e-31,
)

cosine_coefficient_9 = Double(
	hi = -.1.5619206968586225e-16,
	lo = -.1.1910679660273754e-32,
)

cosine_coefficient_10 = Double(
	hi = 4.110317623312165e-19,
	lo = 1.4412973378659527e-36,
)

cosine_coefficient_11 = Double(
	hi = -.8.896791392450574e-22,
	lo = 7.911402614872376e-38,
)

cosine_coefficient_12 = Double(
	hi = 1.6117375710961184e-24,
	lo = -.3.6846573564509766e-41,
)

cosine_coefficient_13 = Double(
	hi = -.2.4795962632247976e-27,
	lo = 1.2953730964765229e-43,
)

atan_coefficient_0 = Double(hi = 1.0, lo = 0.0)

atan_coefficient_1 = Double(
	hi = -.0.3333333333333333,
	lo = -.1.850371707708594e-17,
)

atan_coefficient_2 = Double(hi = 0.2, lo = -.1.1102230246251566e-17)

atan_coefficient_3 = Double(
	hi = -.0.14285714285714285,
	lo = -.7.93016446160826e-18,
)

atan_coefficient_4 = Double(
	hi = 0.1111111111111111,
	lo = 6.1679056923619804e-18,
)

atan_coefficient_5 = Double(
	hi = -.0.09090909090909091,
	lo = 2.523234146875356e-18,
)

atan_coefficient_6 = Double(
	hi = 0.07692307692307693,
	lo = -.4.270088556250602e-18,
)

atan_coefficient_7 = Double(
	hi = -.0.06666666666666667,
	lo = -.9.251858538542971e-19,
)

atan_coefficient_8 = Double(
	hi = 0.058823529411764705,
	lo = 8.163404592832033e-19,
)

sine_high_0 = sine_coefficient_0.hi

sine_low_0 = sine_coefficient_0.lo

sine_high_1 = sine_coefficient_1.hi

sine_low_1 = sine_coefficient_1.lo

sine_high_2 = sine_coefficient_2.hi

sine_low_2 = sine_coefficient_2.lo

sine_high_3 = sine_coefficient_3.hi

sine_low_3 = sine_coefficient_3.lo

sine_high_4 = sine_coefficient_4.hi

sine_low_4 = sine_coefficient_4.lo

sine_high_5 = sine_coefficient_5.hi

sine_low_5 = sine_coefficient_5.lo

sine_high_6 = sine_coefficient_6.hi

sine_high_7 = sine_coefficient_7.hi

sine_high_8 = sine_coefficient_8.hi

sine_high_9 = sine_coefficient_9.hi

sine_high_10 = sine_coefficient_10.hi

sine_high_11 = sine_coefficient_11.hi

sine_high_12 = sine_coefficient_12.hi

sine_high_13 = sine_coefficient_13.hi

sine_polynomial = (square: Double) -> Double {
	w = square.hi
	w_low = square.lo
	split_w = splitter *. w
	w_high = split_w -. { split_w -. w }
	w_tail = w -. w_high
	t13 = sine_high_13
	t12 = t13 *. w +. sine_high_12
	t11 = t12 *. w +. sine_high_11
	t10 = t11 *. w +. sine_high_10
	t9 = t10 *. w +. sine_high_9
	t8 = t9 *. w +. sine_high_8
	t7 = t8 *. w +. sine_high_7
	t6 = t7 *. w +. sine_high_6
	p6_hi = t6
	p6_lo = fast_zero
	s5_product = p6_hi *. w
	s5_split = splitter *. p6_hi
	s5_high = s5_split -. { s5_split -. p6_hi }
	s5_low = p6_hi -. s5_high
	s5_error = {
		{ { s5_high *. w_high } -. s5_product } +.
			{ s5_high *. w_tail } +.
			{ s5_low *. w_high }
	} +.
		{ s5_low *. w_tail }
	s5_correction = s5_error +. { p6_hi *. w_low } +. { p6_lo *. w }
	s5_mul_hi = s5_product +. s5_correction
	s5_mul_lo = s5_correction -. { s5_mul_hi -. s5_product }
	s5_sum = sine_high_5 +. s5_mul_hi
	s5_residual = s5_mul_hi -. { s5_sum -. sine_high_5 }
	s5_sum_low = s5_residual +. sine_low_5 +. s5_mul_lo
	p5_hi = s5_sum +. s5_sum_low
	p5_lo = s5_sum_low -. { p5_hi -. s5_sum }
	s4_product = p5_hi *. w
	s4_split = splitter *. p5_hi
	s4_high = s4_split -. { s4_split -. p5_hi }
	s4_low = p5_hi -. s4_high
	s4_error = {
		{ { s4_high *. w_high } -. s4_product } +.
			{ s4_high *. w_tail } +.
			{ s4_low *. w_high }
	} +.
		{ s4_low *. w_tail }
	s4_correction = s4_error +. { p5_hi *. w_low } +. { p5_lo *. w }
	s4_mul_hi = s4_product +. s4_correction
	s4_mul_lo = s4_correction -. { s4_mul_hi -. s4_product }
	s4_sum = sine_high_4 +. s4_mul_hi
	s4_residual = s4_mul_hi -. { s4_sum -. sine_high_4 }
	s4_sum_low = s4_residual +. sine_low_4 +. s4_mul_lo
	p4_hi = s4_sum +. s4_sum_low
	p4_lo = s4_sum_low -. { p4_hi -. s4_sum }
	s3_product = p4_hi *. w
	s3_split = splitter *. p4_hi
	s3_high = s3_split -. { s3_split -. p4_hi }
	s3_low = p4_hi -. s3_high
	s3_error = {
		{ { s3_high *. w_high } -. s3_product } +.
			{ s3_high *. w_tail } +.
			{ s3_low *. w_high }
	} +.
		{ s3_low *. w_tail }
	s3_correction = s3_error +. { p4_hi *. w_low } +. { p4_lo *. w }
	s3_mul_hi = s3_product +. s3_correction
	s3_mul_lo = s3_correction -. { s3_mul_hi -. s3_product }
	s3_sum = sine_high_3 +. s3_mul_hi
	s3_residual = s3_mul_hi -. { s3_sum -. sine_high_3 }
	s3_sum_low = s3_residual +. sine_low_3 +. s3_mul_lo
	p3_hi = s3_sum +. s3_sum_low
	p3_lo = s3_sum_low -. { p3_hi -. s3_sum }
	s2_product = p3_hi *. w
	s2_split = splitter *. p3_hi
	s2_high = s2_split -. { s2_split -. p3_hi }
	s2_low = p3_hi -. s2_high
	s2_error = {
		{ { s2_high *. w_high } -. s2_product } +.
			{ s2_high *. w_tail } +.
			{ s2_low *. w_high }
	} +.
		{ s2_low *. w_tail }
	s2_correction = s2_error +. { p3_hi *. w_low } +. { p3_lo *. w }
	s2_mul_hi = s2_product +. s2_correction
	s2_mul_lo = s2_correction -. { s2_mul_hi -. s2_product }
	s2_sum = sine_high_2 +. s2_mul_hi
	s2_residual = s2_mul_hi -. { s2_sum -. sine_high_2 }
	s2_sum_low = s2_residual +. sine_low_2 +. s2_mul_lo
	p2_hi = s2_sum +. s2_sum_low
	p2_lo = s2_sum_low -. { p2_hi -. s2_sum }
	s1_product = p2_hi *. w
	s1_split = splitter *. p2_hi
	s1_high = s1_split -. { s1_split -. p2_hi }
	s1_low = p2_hi -. s1_high
	s1_error = {
		{ { s1_high *. w_high } -. s1_product } +.
			{ s1_high *. w_tail } +.
			{ s1_low *. w_high }
	} +.
		{ s1_low *. w_tail }
	s1_correction = s1_error +. { p2_hi *. w_low } +. { p2_lo *. w }
	s1_mul_hi = s1_product +. s1_correction
	s1_mul_lo = s1_correction -. { s1_mul_hi -. s1_product }
	s1_sum = sine_high_1 +. s1_mul_hi
	s1_residual = s1_mul_hi -. { s1_sum -. sine_high_1 }
	s1_sum_low = s1_residual +. sine_low_1 +. s1_mul_lo
	p1_hi = s1_sum +. s1_sum_low
	p1_lo = s1_sum_low -. { p1_hi -. s1_sum }
	s0_product = p1_hi *. w
	s0_split = splitter *. p1_hi
	s0_high = s0_split -. { s0_split -. p1_hi }
	s0_low = p1_hi -. s0_high
	s0_error = {
		{ { s0_high *. w_high } -. s0_product } +.
			{ s0_high *. w_tail } +.
			{ s0_low *. w_high }
	} +.
		{ s0_low *. w_tail }
	s0_correction = s0_error +. { p1_hi *. w_low } +. { p1_lo *. w }
	s0_mul_hi = s0_product +. s0_correction
	s0_mul_lo = s0_correction -. { s0_mul_hi -. s0_product }
	s0_sum = sine_high_0 +. s0_mul_hi
	s0_residual = s0_mul_hi -. { s0_sum -. sine_high_0 }
	s0_sum_low = s0_residual +. sine_low_0 +. s0_mul_lo
	p0_hi = s0_sum +. s0_sum_low
	p0_lo = s0_sum_low -. { p0_hi -. s0_sum }
	Double(hi = p0_hi, lo = p0_lo)
}

cosine_high_0 = cosine_coefficient_0.hi

cosine_low_0 = cosine_coefficient_0.lo

cosine_high_1 = cosine_coefficient_1.hi

cosine_low_1 = cosine_coefficient_1.lo

cosine_high_2 = cosine_coefficient_2.hi

cosine_low_2 = cosine_coefficient_2.lo

cosine_high_3 = cosine_coefficient_3.hi

cosine_low_3 = cosine_coefficient_3.lo

cosine_high_4 = cosine_coefficient_4.hi

cosine_low_4 = cosine_coefficient_4.lo

cosine_high_5 = cosine_coefficient_5.hi

cosine_low_5 = cosine_coefficient_5.lo

cosine_high_6 = cosine_coefficient_6.hi

cosine_low_6 = cosine_coefficient_6.lo

cosine_high_7 = cosine_coefficient_7.hi

cosine_high_8 = cosine_coefficient_8.hi

cosine_high_9 = cosine_coefficient_9.hi

cosine_high_10 = cosine_coefficient_10.hi

cosine_high_11 = cosine_coefficient_11.hi

cosine_high_12 = cosine_coefficient_12.hi

cosine_high_13 = cosine_coefficient_13.hi

cosine_polynomial = (square: Double) -> Double {
	w = square.hi
	w_low = square.lo
	split_w = splitter *. w
	w_high = split_w -. { split_w -. w }
	w_tail = w -. w_high
	t13 = cosine_high_13
	t12 = t13 *. w +. cosine_high_12
	t11 = t12 *. w +. cosine_high_11
	t10 = t11 *. w +. cosine_high_10
	t9 = t10 *. w +. cosine_high_9
	t8 = t9 *. w +. cosine_high_8
	t7 = t8 *. w +. cosine_high_7
	p7_hi = t7
	p7_lo = fast_zero
	s6_product = p7_hi *. w
	s6_split = splitter *. p7_hi
	s6_high = s6_split -. { s6_split -. p7_hi }
	s6_low = p7_hi -. s6_high
	s6_error = {
		{ { s6_high *. w_high } -. s6_product } +.
			{ s6_high *. w_tail } +.
			{ s6_low *. w_high }
	} +.
		{ s6_low *. w_tail }
	s6_correction = s6_error +. { p7_hi *. w_low } +. { p7_lo *. w }
	s6_mul_hi = s6_product +. s6_correction
	s6_mul_lo = s6_correction -. { s6_mul_hi -. s6_product }
	s6_sum = cosine_high_6 +. s6_mul_hi
	s6_residual = s6_mul_hi -. { s6_sum -. cosine_high_6 }
	s6_sum_low = s6_residual +. cosine_low_6 +. s6_mul_lo
	p6_hi = s6_sum +. s6_sum_low
	p6_lo = s6_sum_low -. { p6_hi -. s6_sum }
	s5_product = p6_hi *. w
	s5_split = splitter *. p6_hi
	s5_high = s5_split -. { s5_split -. p6_hi }
	s5_low = p6_hi -. s5_high
	s5_error = {
		{ { s5_high *. w_high } -. s5_product } +.
			{ s5_high *. w_tail } +.
			{ s5_low *. w_high }
	} +.
		{ s5_low *. w_tail }
	s5_correction = s5_error +. { p6_hi *. w_low } +. { p6_lo *. w }
	s5_mul_hi = s5_product +. s5_correction
	s5_mul_lo = s5_correction -. { s5_mul_hi -. s5_product }
	s5_sum = cosine_high_5 +. s5_mul_hi
	s5_residual = s5_mul_hi -. { s5_sum -. cosine_high_5 }
	s5_sum_low = s5_residual +. cosine_low_5 +. s5_mul_lo
	p5_hi = s5_sum +. s5_sum_low
	p5_lo = s5_sum_low -. { p5_hi -. s5_sum }
	s4_product = p5_hi *. w
	s4_split = splitter *. p5_hi
	s4_high = s4_split -. { s4_split -. p5_hi }
	s4_low = p5_hi -. s4_high
	s4_error = {
		{ { s4_high *. w_high } -. s4_product } +.
			{ s4_high *. w_tail } +.
			{ s4_low *. w_high }
	} +.
		{ s4_low *. w_tail }
	s4_correction = s4_error +. { p5_hi *. w_low } +. { p5_lo *. w }
	s4_mul_hi = s4_product +. s4_correction
	s4_mul_lo = s4_correction -. { s4_mul_hi -. s4_product }
	s4_sum = cosine_high_4 +. s4_mul_hi
	s4_residual = s4_mul_hi -. { s4_sum -. cosine_high_4 }
	s4_sum_low = s4_residual +. cosine_low_4 +. s4_mul_lo
	p4_hi = s4_sum +. s4_sum_low
	p4_lo = s4_sum_low -. { p4_hi -. s4_sum }
	s3_product = p4_hi *. w
	s3_split = splitter *. p4_hi
	s3_high = s3_split -. { s3_split -. p4_hi }
	s3_low = p4_hi -. s3_high
	s3_error = {
		{ { s3_high *. w_high } -. s3_product } +.
			{ s3_high *. w_tail } +.
			{ s3_low *. w_high }
	} +.
		{ s3_low *. w_tail }
	s3_correction = s3_error +. { p4_hi *. w_low } +. { p4_lo *. w }
	s3_mul_hi = s3_product +. s3_correction
	s3_mul_lo = s3_correction -. { s3_mul_hi -. s3_product }
	s3_sum = cosine_high_3 +. s3_mul_hi
	s3_residual = s3_mul_hi -. { s3_sum -. cosine_high_3 }
	s3_sum_low = s3_residual +. cosine_low_3 +. s3_mul_lo
	p3_hi = s3_sum +. s3_sum_low
	p3_lo = s3_sum_low -. { p3_hi -. s3_sum }
	s2_product = p3_hi *. w
	s2_split = splitter *. p3_hi
	s2_high = s2_split -. { s2_split -. p3_hi }
	s2_low = p3_hi -. s2_high
	s2_error = {
		{ { s2_high *. w_high } -. s2_product } +.
			{ s2_high *. w_tail } +.
			{ s2_low *. w_high }
	} +.
		{ s2_low *. w_tail }
	s2_correction = s2_error +. { p3_hi *. w_low } +. { p3_lo *. w }
	s2_mul_hi = s2_product +. s2_correction
	s2_mul_lo = s2_correction -. { s2_mul_hi -. s2_product }
	s2_sum = cosine_high_2 +. s2_mul_hi
	s2_residual = s2_mul_hi -. { s2_sum -. cosine_high_2 }
	s2_sum_low = s2_residual +. cosine_low_2 +. s2_mul_lo
	p2_hi = s2_sum +. s2_sum_low
	p2_lo = s2_sum_low -. { p2_hi -. s2_sum }
	s1_product = p2_hi *. w
	s1_split = splitter *. p2_hi
	s1_high = s1_split -. { s1_split -. p2_hi }
	s1_low = p2_hi -. s1_high
	s1_error = {
		{ { s1_high *. w_high } -. s1_product } +.
			{ s1_high *. w_tail } +.
			{ s1_low *. w_high }
	} +.
		{ s1_low *. w_tail }
	s1_correction = s1_error +. { p2_hi *. w_low } +. { p2_lo *. w }
	s1_mul_hi = s1_product +. s1_correction
	s1_mul_lo = s1_correction -. { s1_mul_hi -. s1_product }
	s1_sum = cosine_high_1 +. s1_mul_hi
	s1_residual = s1_mul_hi -. { s1_sum -. cosine_high_1 }
	s1_sum_low = s1_residual +. cosine_low_1 +. s1_mul_lo
	p1_hi = s1_sum +. s1_sum_low
	p1_lo = s1_sum_low -. { p1_hi -. s1_sum }
	s0_product = p1_hi *. w
	s0_split = splitter *. p1_hi
	s0_high = s0_split -. { s0_split -. p1_hi }
	s0_low = p1_hi -. s0_high
	s0_error = {
		{ { s0_high *. w_high } -. s0_product } +.
			{ s0_high *. w_tail } +.
			{ s0_low *. w_high }
	} +.
		{ s0_low *. w_tail }
	s0_correction = s0_error +. { p1_hi *. w_low } +. { p1_lo *. w }
	s0_mul_hi = s0_product +. s0_correction
	s0_mul_lo = s0_correction -. { s0_mul_hi -. s0_product }
	s0_sum = cosine_high_0 +. s0_mul_hi
	s0_residual = s0_mul_hi -. { s0_sum -. cosine_high_0 }
	s0_sum_low = s0_residual +. cosine_low_0 +. s0_mul_lo
	p0_hi = s0_sum +. s0_sum_low
	p0_lo = s0_sum_low -. { p0_hi -. s0_sum }
	Double(hi = p0_hi, lo = p0_lo)
}

atan_high_0 = atan_coefficient_0.hi

atan_low_0 = atan_coefficient_0.lo

atan_high_1 = atan_coefficient_1.hi

atan_low_1 = atan_coefficient_1.lo

atan_high_2 = atan_coefficient_2.hi

atan_low_2 = atan_coefficient_2.lo

atan_high_3 = atan_coefficient_3.hi

atan_low_3 = atan_coefficient_3.lo

atan_high_4 = atan_coefficient_4.hi

atan_high_5 = atan_coefficient_5.hi

atan_high_6 = atan_coefficient_6.hi

atan_high_7 = atan_coefficient_7.hi

atan_high_8 = atan_coefficient_8.hi

atan_polynomial = (square: Double) -> Double {
	w = square.hi
	w_low = square.lo
	split_w = splitter *. w
	w_high = split_w -. { split_w -. w }
	w_tail = w -. w_high
	t8 = atan_high_8
	t7 = t8 *. w +. atan_high_7
	t6 = t7 *. w +. atan_high_6
	t5 = t6 *. w +. atan_high_5
	t4 = t5 *. w +. atan_high_4
	p4_hi = t4
	p4_lo = fast_zero
	s3_product = p4_hi *. w
	s3_split = splitter *. p4_hi
	s3_high = s3_split -. { s3_split -. p4_hi }
	s3_low = p4_hi -. s3_high
	s3_error = {
		{ { s3_high *. w_high } -. s3_product } +.
			{ s3_high *. w_tail } +.
			{ s3_low *. w_high }
	} +.
		{ s3_low *. w_tail }
	s3_correction = s3_error +. { p4_hi *. w_low } +. { p4_lo *. w }
	s3_mul_hi = s3_product +. s3_correction
	s3_mul_lo = s3_correction -. { s3_mul_hi -. s3_product }
	s3_sum = atan_high_3 +. s3_mul_hi
	s3_residual = s3_mul_hi -. { s3_sum -. atan_high_3 }
	s3_sum_low = s3_residual +. atan_low_3 +. s3_mul_lo
	p3_hi = s3_sum +. s3_sum_low
	p3_lo = s3_sum_low -. { p3_hi -. s3_sum }
	s2_product = p3_hi *. w
	s2_split = splitter *. p3_hi
	s2_high = s2_split -. { s2_split -. p3_hi }
	s2_low = p3_hi -. s2_high
	s2_error = {
		{ { s2_high *. w_high } -. s2_product } +.
			{ s2_high *. w_tail } +.
			{ s2_low *. w_high }
	} +.
		{ s2_low *. w_tail }
	s2_correction = s2_error +. { p3_hi *. w_low } +. { p3_lo *. w }
	s2_mul_hi = s2_product +. s2_correction
	s2_mul_lo = s2_correction -. { s2_mul_hi -. s2_product }
	s2_sum = atan_high_2 +. s2_mul_hi
	s2_residual = s2_mul_hi -. { s2_sum -. atan_high_2 }
	s2_sum_low = s2_residual +. atan_low_2 +. s2_mul_lo
	p2_hi = s2_sum +. s2_sum_low
	p2_lo = s2_sum_low -. { p2_hi -. s2_sum }
	s1_product = p2_hi *. w
	s1_split = splitter *. p2_hi
	s1_high = s1_split -. { s1_split -. p2_hi }
	s1_low = p2_hi -. s1_high
	s1_error = {
		{ { s1_high *. w_high } -. s1_product } +.
			{ s1_high *. w_tail } +.
			{ s1_low *. w_high }
	} +.
		{ s1_low *. w_tail }
	s1_correction = s1_error +. { p2_hi *. w_low } +. { p2_lo *. w }
	s1_mul_hi = s1_product +. s1_correction
	s1_mul_lo = s1_correction -. { s1_mul_hi -. s1_product }
	s1_sum = atan_high_1 +. s1_mul_hi
	s1_residual = s1_mul_hi -. { s1_sum -. atan_high_1 }
	s1_sum_low = s1_residual +. atan_low_1 +. s1_mul_lo
	p1_hi = s1_sum +. s1_sum_low
	p1_lo = s1_sum_low -. { p1_hi -. s1_sum }
	s0_product = p1_hi *. w
	s0_split = splitter *. p1_hi
	s0_high = s0_split -. { s0_split -. p1_hi }
	s0_low = p1_hi -. s0_high
	s0_error = {
		{ { s0_high *. w_high } -. s0_product } +.
			{ s0_high *. w_tail } +.
			{ s0_low *. w_high }
	} +.
		{ s0_low *. w_tail }
	s0_correction = s0_error +. { p1_hi *. w_low } +. { p1_lo *. w }
	s0_mul_hi = s0_product +. s0_correction
	s0_mul_lo = s0_correction -. { s0_mul_hi -. s0_product }
	s0_sum = atan_high_0 +. s0_mul_hi
	s0_residual = s0_mul_hi -. { s0_sum -. atan_high_0 }
	s0_sum_low = s0_residual +. atan_low_0 +. s0_mul_lo
	p0_hi = s0_sum +. s0_sum_low
	p0_lo = s0_sum_low -. { p0_hi -. s0_sum }
	Double(hi = p0_hi, lo = p0_lo)
}

// END GENERATED FAST COEFFICIENTS
fast_sin = (x: Float) -> Float {
	fast_sincos(x, False) |> (Some(v)) { v } | (None) { x }
}
