//// Functions for `Float`, the type of finite IEEE 754 binary64 numbers.
////
//// A Float operation whose result is not finite stops the program
//// (§11.3), so a Float is never infinite or NaN. `a /. 0.0` is `0.0`;
//// `divide` gives `Error(Nil)` for a zero divisor instead. Functions here
//// that could give a result that is not finite, such as `power`, give
//// `Error(Nil)` in its place, so they never stop the program.
(
	i/core
	order = (Order, Less, Equal, Greater) = std/order
)

(
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
	sign_bit
	copy_sign
	asin
	acos
	atan
	atan2
	hypot
	radians
	degrees
)

/// The greatest finite Float, about `1.8e308`.
max_finite = 1.7976931348623157e308

/// The smallest Float greater than `0.0`: `5.0e-324`, a subnormal number.
min_positive = 5.0e-324

/// The nearest binary64 value to pi.
pi = 3.141592653589793

/// The nearest binary64 value to two pi.
tau = 6.283185307179586

sign_mask = 9223372036854775808

/// Whether the sign bit is set, including for negative zero.
sign_bit = (x: Float) -> Bool { core.float_to_bits(x) >= sign_mask }

/// Copy the sign bit of `sign` to `magnitude`.
copy_sign = (magnitude: Float, sign: Float) -> Float {
	core.float_from_bits(
		core.float_to_bits(magnitude) % sign_mask +
			core.float_to_bits(sign) / sign_mask * sign_mask,
	)
}

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
/// float.loosely_equals(0.1 +. 0.2, 0.3, 1.0e-9)  // True
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
/// float.power(2.0, 10)  // Ok(1024.0)
/// float.power(-.2.0, -1)  // Ok(-.0.5)
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
/// float.modulo(13.3, 3.3)  // Ok(0.10000000000000053)
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
/// float.real_power(16.0, 0.5)  // Ok(4.0)
/// float.real_power(-.8.0, 1.0 /. 3.0)  // Error(Nil)
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

// The rational approximation and argument reduction are from FreeBSD msun
// e_asin.c and e_acos.c, developed at SunPro, Sun Microsystems, Inc.
// Copyright (C) 1993 Sun Microsystems, Inc. Permission to use, copy,
// modify and distribute is freely granted provided this notice is preserved.
asin_ratio = (z: Float) -> Float {
	p = z *.
		{
			0.166666666666666657415 +.
				z *.
					{
						-.0.325565818622400915405 +.
							z *.
								{
									0.201212532134862925881 +.
										z *.
											{
												-.0.0400555345006794114027 +.
													z *.
														{
															0.000791534994289814532176 +.
																z *.
																	0.0000347933107596021167570
														}
											}
								}
					}
		}
	q = 1.0 +.
		z *.
			{
				-.2.40339491173441421878 +.
					z *.
						{
							2.02094576023350569471 +.
								z *.
									{
										-.0.688283971605453293030 +.
											z *. 0.0770381505559019352791
									}
						}
			}
	p /. q
}

/// The arcsine in radians; inputs outside [-1, 1] give Error(Nil).
asin = (x: Float) -> Result(Float, Nil) {
	a = abs(x)
	{ a >. 1.0 } |>
		(True) { Error(Nil) }
		| (False) {
			{ a == 1.0 } |>
				(True) { Ok(copy_sign(1.5707963267948966, x)) }
				| (False) {
					{ a <. 0.5 } |>
						(True) {
							{ a <. 1.4901161193847656e-8 } |>
								(True) { Ok(x) }
								| (False) { Ok(x +. x *. asin_ratio(x *. x)) }
						}
						| (False) {
							z = { 1.0 -. a } *. 0.5
							s = core.sqrt(z)
							r = asin_ratio(z)
							value = { a >=. 0.975 } |>
								(True) {
									1.5707963267948966 -.
										{
											2.0 *. { s +. s *. r } -.
												6.123233995736766e-17
										}
								}
								| (False) {
									f = core.float_from_bits(
										core.float_to_bits(s) /
											4294967296 *
											4294967296,
									)
									c = { z -. f *. f } /. { s +. f }
									0.7853981633974483 -.
										{
											2.0 *. s *. r -.
												{
													6.123233995736766e-17 -.
														2.0 *. c
												} -.
												{
													0.7853981633974483 -.
														2.0 *. f
												}
										}
								}
							Ok(copy_sign(value, x))
						}
				}
		}
}

/// The arccosine in radians; inputs outside [-1, 1] give Error(Nil).
acos = (x: Float) -> Result(Float, Nil) {
	a = abs(x)
	{ a >. 1.0 } |>
		(True) { Error(Nil) }
		| (False) {
			{ a == 1.0 } |>
				(True) {
					{ x <. 0.0 } |> (True) { Ok(pi) } | (False) { Ok(0.0) }
				}
				| (False) {
					{ a <. 0.5 } |>
						(True) {
							{ a <. 6.938893903907228e-18 } |>
								(True) { Ok(1.5707963267948966) }
								| (False) {
									Ok(
										1.5707963267948966 -.
											{
												x -.
													{
														6.123233995736766e-17 -.
															x *.
																asin_ratio(
																	x *. x,
																)
													}
											},
									)
								}
						}
						| (False) {
							{ x <. 0.0 } |>
								(True) {
									z = { 1.0 +. x } *. 0.5
									s = core.sqrt(z)
									Ok(
										2.0 *.
											{
												1.5707963267948966 -.
													{
														s +.
															{
																asin_ratio(z) *.
																	s -.
																	6.123233995736766e-17
															}
													}
											},
									)
								}
								| (False) {
									z = { 1.0 -. x } *. 0.5
									s = core.sqrt(z)
									f = core.float_from_bits(
										core.float_to_bits(s) /
											4294967296 *
											4294967296,
									)
									c = { z -. f *. f } /. { s +. f }
									Ok(
										2.0 *.
											{
												f +. { asin_ratio(z) *. s +. c }
											},
									)
								}
						}
				}
		}
}

// FreeBSD msun s_atan.c and e_atan2.c, with the SunPro notice above.
atan_kernel = (x: Float, hi: Float, lo: Float, reduced: Bool) -> Float {
	z = x *. x
	w = z *. z
	s1 = z *.
		{
			0.333333333333329318027 +.
				w *.
					{
						0.142857142725034663711 +.
							w *.
								{
									0.0909088713343650656196 +.
										w *.
											{
												0.0666107313738753120669 +.
													w *.
														{
															0.0497687799461593236017 +.
																w *.
																	0.0162858201153657823623
														}
											}
								}
					}
		}
	s2 = w *.
		{
			-.0.199999999998764832476 +.
				w *.
					{
						-.0.111111104054623557880 +.
							w *.
								{
									-.0.0769187620504482999495 +.
										w *.
											{
												-.0.0583357013379057348645 +.
													w *.
														-.0.0365315727442169155270
											}
								}
					}
		}
	{ reduced } |>
		(True) { hi -. { x *. { s1 +. s2 } -. lo -. x } }
		| (False) { x -. x *. { s1 +. s2 } }
}

/// Arctangent in radians.
atan = (x: Float) -> Float {
	a = abs(x)
	{ a <. 7.450580596923828e-9 } |>
		(True) { x }
		| (False) {
			{ a <. 0.4375 } |>
				(True) { atan_kernel(x, 0.0, 0.0, False) }
				| (False) {
					value = { a <. 0.6875 } |>
						(True) {
							atan_kernel(
								{ 2.0 *. a -. 1.0 } /. { 2.0 +. a },
								0.4636476090008061,
								2.2698777452961687e-17,
								True,
							)
						}
						| (False) {
							{ a <. 1.1875 } |>
								(True) {
									atan_kernel(
										{ a -. 1.0 } /. { a +. 1.0 },
										0.7853981633974483,
										3.061616997868383e-17,
										True,
									)
								}
								| (False) {
									{ a <. 2.4375 } |>
										(True) {
											atan_kernel(
												{ a -. 1.5 } /.
													{ 1.0 +. 1.5 *. a },
												0.982793723247329,
												1.3903311031230998e-17,
												True,
											)
										}
										| (False) {
											{ a >=. 73786976294838206464.0 } |>
												(True) { 1.5707963267948966 }
												| (False) {
													atan_kernel(
														-.1.0 /. a,
														1.5707963267948966,
														6.123233995736766e-17,
														True,
													)
												}
										}
								}
						}
					copy_sign(value, x)
				}
		}
}

/// The angle of (x, y), in radians, with the sign of y.
atan2 = (y: Float, x: Float) -> Float {
	{ y == 0.0 } |>
		(True) {
			{ sign_bit(x) } |> (True) { copy_sign(pi, y) } | (False) { y }
		}
		| (False) {
			{ x == 0.0 } |>
				(True) { copy_sign(1.5707963267948966, y) }
				| (False) {
					ax = abs(x)
					ay = abs(y)
					ex = core.float_to_bits(ax) / 4503599627370496
					ey = core.float_to_bits(ay) / 4503599627370496
					z = { ey > ex + 64 } |>
						(True) { 1.5707963267948966 }
						| (False) {
							{ sign_bit(x) && ex > ey + 64 } |>
								(True) { 0.0 }
								| (False) { atan(ay /. ax) }
						}
					result = { sign_bit(x) } |>
						(True) { pi -. { z -. 1.2246467991473532e-16 } }
						| (False) { z }
					copy_sign(result, y)
				}
		}
}

/// Convert degrees to radians.
radians = (degrees: Float) -> Float { degrees *. 0.017453292519943295 }

/// Convert radians to degrees, returning an error if the result overflows.
degrees = (radians: Float) -> Result(Float, Nil) {
	{ abs(radians) >. 3.1375664143845866e306 } |>
		(True) { Error(Nil) }
		| (False) { Ok(radians *. 57.29577951308232) }
}

// Compensated squares and scaling from FreeBSD msun hypot.c.
hypot_square = (x: Float) -> (Float, Float) {
	xc = x *. 134217729.0
	xh = x -. xc +. xc
	xl = x -. xh
	hi = x *. x
	lo = xh *. xh -. hi +. 2.0 *. xh *. xl +. xl *. xl
	(hi, lo)
}

/// The Euclidean length of (x, y), returning an error on overflow.
hypot = (x: Float, y: Float) -> Result(Float, Nil) {
	a = max(abs(x), abs(y))
	b = min(abs(x), abs(y))
	ex = core.float_to_bits(a) / mantissa_unit
	ey = core.float_to_bits(b) / mantissa_unit
	{ b == 0.0 || ex - ey > 64 } |>
		(True) { Ok(a) }
		| (False) {
			(sx, sy, factor) = { ex > 1533 } |>
				(True) {
					(
						a *. 1.90109156629516e-211,
						b *. 1.90109156629516e-211,
						5.260135901548374e210,
					)
				}
				| (False) {
					{ ey < 573 } |>
						(True) {
							(
								a *. 5.260135901548374e210,
								b *. 5.260135901548374e210,
								1.90109156629516e-211,
							)
						}
						| (False) { (a, b, 1.0) }
				}
			(hx, lx) = hypot_square(sx)
			(hy, ly) = hypot_square(sy)
			root = core.sqrt(ly +. lx +. hy +. hx)
			{ factor >. 1.0 && root >. max_finite /. factor } |>
				(True) { Error(Nil) }
				| (False) { Ok(factor *. root) }
		}
}

// Payne–Hanek reduction using the 4/pi digits from Go math/trig_reduce.go.
// Copyright 2018 The Go Authors. All rights reserved. BSD-style license.
// The arbitrary precision product retains the bits needed at every Float exponent.
trig_reduce = (x: Float) -> (Int, Float, Float) {
	bits = core.float_to_bits(x)
	exponent = bits / mantissa_unit - 1023 - 52
	mantissa = bits % mantissa_unit + mantissa_unit
	product = mantissa *
		1436762042799249464599471190770549237238954220229867879941249734202507677396818572841661258712197472977584340426583215490169760957443689211057306349163373844600972405389011770843781920761928525465569652634283684085946426272060692158419916259561059652236058246048249386799806649121873585408072943491027126850699742857762370245662594349139533413873096177094268384892759
	denominator = int_power(2, 1217 - exponent)
	n = { product + denominator / 2 } / denominator
	rem = product - n * denominator
	negative = rem < 0
	magnitude = { negative } |> (True) { -rem } | (False) { rem }
	fbits = ratio_bits(magnitude, denominator) |>
		(Ok(b)) { b }
		| (Error(_)) { panic("trig: fractional reduction overflow") }
	f = core.float_from_bits(fbits)
	// Recover the signed tail of the fraction after rounding the leading word.
	fb = core.float_to_bits(f)
	fe = fb / mantissa_unit - 1023 - 52
	fm = fb % mantissa_unit + mantissa_unit
	shift = 1217 - exponent + fe
	represented = { shift >= 0 } |>
		(True) { fm * int_power(2, shift) }
		| (False) { fm / int_power(2, -shift) }
	tail = magnitude - represented
	tail_negative = tail < 0
	tail_magnitude = { tail_negative } |> (True) { -tail } | (False) { tail }
	ft = { tail_magnitude == 0 } |>
		(True) { 0.0 }
		| (False) {
			tb = ratio_bits(tail_magnitude, denominator) |>
				(Ok(b)) { b }
				| (Error(_)) { panic("trig: tail overflow") }
			core.float_from_bits(tb)
		}
	ft = { tail_negative } |> (True) { -.ft } | (False) { ft }
	f = { negative } |> (True) { -.f } | (False) { f }
	ft = { negative } |> (True) { -.ft } | (False) { ft }
	hi = f *. 1.5707963267948966
	fh = without_low(f)
	fl = f -. fh
	lo = { fh *. 1.5707963267341256 -. hi } +.
		fl *. 1.5707963267341256 +.
		f *. 6.077100506506192e-11 +.
		ft *. 1.5707963267948966
	(n % 4, hi, lo)
}

// SunPro fdlibm kernels, via musl __sin.c and __cos.c.
// Copyright (C) 1993 Sun Microsystems, Inc. All rights reserved.
// Developed at SunSoft, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this software is freely
// granted, provided that this notice is preserved.
trig_sin = (x: Float, y: Float) -> Float {
	z = x *. x
	w = z *. z
	r = 8.33333333332248946124e-3 +.
		z *.
			{ -.1.98412698298579493134e-4 +. z *. 2.75573137070700676789e-6 } +.
		z *.
			w *.
			{ -.2.50507602534068634195e-8 +. z *. 1.58969099521155010221e-10 }
	v = z *. x
	x -.
		{ z *. { 0.5 *. y -. v *. r } -. y -. v *. -.1.66666666666666324348e-1 }
}

trig_cos = (x: Float, y: Float) -> Float {
	z = x *. x
	w = z *. z
	r = z *.
		{
			4.16666666666666019037e-2 +.
				z *.
					{
						-.1.38888888888741095749e-3 +.
							z *. 2.48015872894767294178e-5
					}
		} +.
		w *.
			w *.
			{
				-.2.75573143513906633035e-7 +.
					z *.
						{
							2.08757232129817482790e-9 +.
								z *. -.1.13596475577881948265e-11
						}
			}
	hz = 0.5 *. z
	a = 1.0 -. hz
	a +. { { 1.0 -. a -. hz } +. { z *. r -. x *. y } }
}

/// Sine of an angle in radians.
sin = (x: Float) -> Float {
	a = abs(x)
	{ a <. 0.7853981633974483 } |>
		(True) {
			{ a <. 1.4901161193847656e-8 } |>
				(True) { x }
				| (False) { trig_sin(x, 0.0) }
		}
		| (False) {
			(n, hi, lo) = trig_reduce(a)
			v = n |>
				(0) { trig_sin(hi, lo) }
				| (1) { trig_cos(hi, lo) }
				| (2) { -.trig_sin(hi, lo) }
				| (_) { -.trig_cos(hi, lo) }
			{ sign_bit(x) } |> (True) { -.v } | (False) { v }
		}
}

/// Cosine of an angle in radians.
cos = (x: Float) -> Float {
	a = abs(x)
	{ a <. 0.7853981633974483 } |>
		(True) { trig_cos(a, 0.0) }
		| (False) {
			(n, hi, lo) = trig_reduce(a)
			n |>
				(0) { trig_cos(hi, lo) }
				| (1) { -.trig_sin(hi, lo) }
				| (2) { -.trig_cos(hi, lo) }
				| (_) { trig_sin(hi, lo) }
		}
}

// SunPro fdlibm tangent kernel, via musl __tan.c.
// Copyright 2004 Sun Microsystems, Inc. All rights reserved.
// Permission to use, copy, modify, and distribute this software is freely
// granted, provided that this notice is preserved.
trig_tan = (input: Float, tail: Float, odd: Bool) -> Float {
	big = abs(input) >=. 0.6744
	negative = sign_bit(input)
	(x, y) = { big } |>
		(True) {
			a = abs(input)
			b = { negative } |> (True) { -.tail } | (False) { tail }
			(0.7853981633974483 -. a +. { 3.061616997868383e-17 -. b }, 0.0)
		}
		| (False) { (input, tail) }
	z = x *. x
	w = z *. z
	r = 1.33333333333201242699e-1 +.
		w *.
			{
				2.18694882948595424599e-2 +.
					w *.
						{
							3.59207910759131235356e-3 +.
								w *.
									{
										5.88041240820264096874e-4 +.
											w *.
												{
													7.81794442939557092300e-5 +.
														w *.
															-.1.85586374855275456654e-5
												}
									}
						}
			}
	v = z *.
		{
			5.39682539762260521377e-2 +.
				w *.
					{
						8.86323982359930005737e-3 +.
							w *.
								{
									1.45620945432529025516e-3 +.
										w *.
											{
												2.46463134818469906812e-4 +.
													w *.
														{
															7.14072491382608190305e-5 +.
																w *.
																	2.59073051863633712884e-5
														}
											}
								}
					}
		}
	s = z *. x
	r = y +. z *. { s *. { r +. v } +. y } +. s *. 3.33333333333334091986e-1
	w = x +. r
	{ big } |>
		(True) {
			s = { odd } |> (True) { -.1.0 } | (False) { 1.0 }
			v = s -. 2.0 *. { x +. { r -. w *. w /. { w +. s } } }
			{ negative } |> (True) { -.v } | (False) { v }
		}
		| (False) {
			{ odd } |>
				(False) { w }
				| (True) {
					w0 = without_low(w)
					v = r -. { w0 -. x }
					a = -.1.0 /. w
					a0 = without_low(a)
					a0 +. a *. { 1.0 +. a0 *. w0 +. a0 *. v }
				}
		}
}

/// Tangent of an angle in radians.
tan = (x: Float) -> Result(Float, Nil) {
	a = abs(x)
	{ a <. 0.7853981633974483 } |>
		(True) {
			{ a <. 1.4901161193847656e-8 } |>
				(True) { Ok(x) }
				| (False) { Ok(trig_tan(x, 0.0, False)) }
		}
		| (False) {
			(n, hi, lo) = trig_reduce(a)
			odd = n % 2 == 1
			{ odd && abs(hi +. lo) <=. 5.562684646268003e-309 } |>
				(True) { Error(Nil) }
				| (False) {
					v = trig_tan(hi, lo, odd)
					Ok({ sign_bit(x) } |> (True) { -.v } | (False) { v })
				}
		}
}
