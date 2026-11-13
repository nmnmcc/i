//// The text form of a Float (§11.8): the shortest digits that read back as
//// the same Float, correctly rounded. `core.inspect` and every Float in an
//// inspected value use this function, so it uses no `core.inspect` itself.
////
//// The digit loop is the Dragon interval construction (Steele & White,
//// "How to Print Floating-Point Numbers Accurately", 1990; Burger & Dybvig,
//// "Printing Floating-Point Numbers Quickly and Accurately", 1996): the
//// exact value and the two rounding midpoints are exact integers, so every
//// decision is an integer comparison, also for subnormals and the largest
//// finite value. No floating-point logarithm is used.

(
	i/core
)

(
	to_string
)

/// The shortest text that reads back as `x`.
///
/// ```i
/// float_text.to_string(0.1)      // "0.1"
/// float_text.to_string(1.0e20)   // "1.0e20"
/// float_text.to_string(-0.0025)  // "-0.0025"
/// ```
to_string = (x: Float) -> String {
	bits = core.float_to_bits(x)
	// The fields of the bits are taken apart with shifts and masks, so that
	// the numbers below stay small for ordinary Floats. Bit 63 is the sign; the
	// numbers are written out in place, because a top-level value is
	// worked out at start, in the frame of every program.
	sign = { core.shift_right(bits, 63) == 1 } |> {
		True  -> "-"
		False -> ""
	}
	encoded = core.bit_and(core.shift_right(bits, 52), 2047)
	fraction = core.bit_and(bits, 4503599627370495)
	{ encoded == 0 && fraction == 0 } |> {
		True  -> core.concat([sign, "0.0"])
		False -> core.concat([sign, ..positive(encoded, fraction)])
	}
}

// The pieces of the text of the positive Float with this exponent field and
// fraction.
positive = (encoded: Int, fraction: Int) -> List(String) {
	// v = r / s × 2^e, with the midpoint distances minus / s and plus / s.
	// At an exact normal power of two the binade below has half the spacing.
	(mantissa, r, minus, plus, e) = { encoded == 0 } |> {
		True -> (fraction, fraction * 2, 1, 1, -1075)
		False -> {
			m = fraction + 4503599627370496
			low = { fraction == 0 && encoded > 1 } |> {
				True  -> 1
				False -> 2
			}
			(m, m * 4, low, 2, encoded - 1077)
		}
	}
	inclusive = mantissa % 2 == 0
	(r, s, minus, plus) = { e < 0 } |> {
		True  -> (r, core.shift_left(1, -e), minus, plus)
		False -> (core.shift_left(r, e), 1, core.shift_left(minus, e), core.shift_left(plus, e))
	}
	// Scale by a power of ten first, from an estimate of the decimal
	// exponent that is never too high: j <= floor(log10(v)). 78913 / 2^18 is
	// just below log10(2); the binary exponent of v is at least `low`.
	low = { encoded == 0 } |> {
		True  -> -1075
		False -> encoded - 1023
	}
	j = floor_divide(low * 78913, 262144) - 1
	(r, s, minus, plus) = { j >= 0 } |> {
		True -> (r, s * power_of_ten(j), minus, plus)
		False -> {
			t = power_of_ten(-j)
			(r * t, s, minus * t, plus * t)
		}
	}
	(s, k) = grow(r, s, j + 1)
	(r, minus, plus, k) = shrink(r, s, minus, plus, k)
	(reversed, k) = generate(r, s, minus, plus, inclusive, [], k)
	digits = reverse_onto(trim(reversed), [])
	// 1076 is the exponent field of 2^53, the first Float whose whole
	// digits imply more precision than a Float has.
	layout(digits, length(digits, 0), k, encoded >= 1076)
}

// floor(a / b) for b > 0.
floor_divide = (a: Int, b: Int) -> Int {
	{ a < 0 } |> {
		True  -> 0 - { b - 1 - a } / b
		False -> a / b
	}
}

// 10^n, by squaring.
// 10^n for n from 0 to 340: a power of 10^16 from a table, times a power
// below 10^16, which the runtime multiplies in 64 bits.
power_of_ten = (n: Int) -> Int { sixteens(n / 16) * small_power(n % 16, 1) }

small_power = (n: Int, acc: Int) -> Int {
	{ n == 0 } |> {
		True  -> acc
		False -> small_power(n - 1, acc * 10)
	}
}

// 10^(16 k).
sixteens = (k: Int) -> Int {
	k |> {
		0 -> 1
		1 -> 10000000000000000
		2 -> 100000000000000000000000000000000
		3 -> 1000000000000000000000000000000000000000000000000
		4 -> 10000000000000000000000000000000000000000000000000000000000000000
		5 -> 100000000000000000000000000000000000000000000000000000000000000000000000000000000
		6 -> 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		7 -> 10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		8 -> 100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		9 -> 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		10 -> 10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		11 -> 100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		12 -> 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		13 -> 10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		14 -> 100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		15 -> 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		16 -> 10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		17 -> 100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		18 -> 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		19 -> 10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		20 -> 100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
		_ -> 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
	}
}

// Exact normalization, part one: the smallest `s × 10^j` above `r`, with
// `k` counting the steps.
grow = (r: Int, s: Int, k: Int) -> (Int, Int) {
	next = s * 10
	{ r < next } |> {
		True  -> (s, k)
		False -> grow(r, next, k + 1)
	}
}

// Exact normalization, part two: `1 <= r/s`. Multiplying the numerator and
// both margins together keeps the round-to-nearest interval.
shrink = (r: Int, s: Int, minus: Int, plus: Int, k: Int) -> (Int, Int, Int, Int) {
	{ r >= s } |> {
		True  -> (r, minus, plus, k)
		False -> shrink(r * 10, s, minus * 10, plus * 10, k - 1)
	}
}

// The digits, last first, until the rest is inside the rounding interval,
// and the decimal exponent `k` after rounding.
generate = (
	r: Int
	s: Int
	minus: Int
	plus: Int
	inclusive: Bool
	acc: List(Int)
	k: Int
) -> (List(Int), Int) {
	digit = r / s
	rest = r - digit * s
	digits = [digit, ..acc]
	low = rest - minus
	high = rest + plus - s
	down = low < 0 || { inclusive && low == 0 }
	up = high > 0 || { inclusive && high == 0 }
	{ down || up } |> {
		False -> generate(rest * 10, s, minus * 10, plus * 10, inclusive, digits, k)
		True -> {
			// Where both shortest candidates are valid, the closer one; an
			// exact halfway tie goes towards an even last digit.
			half = rest * 2 - s
			round_up = up && {
				core.not(down) || half > 0 || { half == 0 && digit % 2 == 1 }
			}
			round_up |> {
				False -> (digits, k)
				True  -> carry(digits) |> {
					(more, True)  -> (more, k + 1)
					(more, False) -> (more, k)
				}
			}
		}
	}
}

// Adds one to the last digit; `True` when the first digit carries out,
// and then the digits are a single 1.
carry = (reversed: List(Int)) -> (List(Int), Bool) {
	reversed |> {
		[d, ..rest] ? d < 9 -> ([d + 1, ..rest], False)
		[_]                 -> ([1], True)
		[_, ..rest]         -> carry(rest) |> {
			(_, True)     -> ([1], True)
			(more, False) -> ([0, ..more], False)
		}
		[] -> ([1], True)
	}
}

// Without zeros at the end, keeping at least one digit.
trim = (reversed: List(Int)) -> List(Int) {
	reversed |> {
		[0, ..rest] ? rest != [] -> trim(rest)
		_                        -> reversed
	}
}

reverse_onto = (list: List(Int), acc: List(Int)) -> List(Int) {
	list |> {
		[]        -> acc
		[x, ..xs] -> reverse_onto(xs, [x, ..acc])
	}
}

length = (list: List(Int), n: Int) -> Int {
	list |> {
		[]        -> n
		[_, ..xs] -> length(xs, n + 1)
	}
}

// §11.8: the shortest digits, in scientific notation once whole decimal
// output would imply precision above 2^53, or would need many zeros.
layout = (digits: List(Int), n: Int, k: Int, imprecise: Bool) -> List(String) {
	point = k - n
	lower = { n == 1 } |> {
		True  -> -4
		False -> -2 - n
	}
	upper = { n == 1 || k >= 11 } |> {
		True  -> 2
		False -> 1
	}
	scientific = point < lower || point > upper || { point >= 0 && imprecise }
	scientific |> {
		True -> digits |> {
			[first, ..rest] -> {
				fraction = { rest == [] } |> {
					True  -> ["0"]
					False -> texts(rest, [])
				}
				append([digit_text(first), ".", ..fraction], ["e", ..int_text(k - 1)])
			}
			[] -> ["0.0"]
		}
		False -> { k <= 0 } |> {
			True  -> ["0.", ..zeros(-k, texts(digits, []))]
			False -> { k >= n } |> {
				True  -> texts(digits, zeros(k - n, [".0"]))
				False -> split(digits, k, [])
			}
		}
	}
}

// The digits with a point after the first `k` of them.
split = (digits: List(Int), k: Int, acc: List(String)) -> List(String) {
	{ k == 0 } |> {
		True -> reverse_texts(acc, [".", ..texts(digits, [])])
		False -> digits |> {
			[d, ..rest] -> split(rest, k - 1, [digit_text(d), ..acc])
			[]          -> reverse_texts(acc, [])
		}
	}
}

reverse_texts = (list: List(String), acc: List(String)) -> List(String) {
	list |> {
		[]        -> acc
		[x, ..xs] -> reverse_texts(xs, [x, ..acc])
	}
}

append = (front: List(String), back: List(String)) -> List(String) {
	reverse_texts(reverse_texts(front, []), back)
}

// `count` zeros before `rest`.
zeros = (count: Int, rest: List(String)) -> List(String) {
	{ count <= 0 } |> {
		True  -> rest
		False -> zeros(count - 1, ["0", ..rest])
	}
}

// The text of each digit, before `rest`.
texts = (digits: List(Int), rest: List(String)) -> List(String) {
	reverse_texts(digit_list(digits, []), rest)
}

digit_list = (digits: List(Int), acc: List(String)) -> List(String) {
	digits |> {
		[]        -> acc
		[d, ..ds] -> digit_list(ds, [digit_text(d), ..acc])
	}
}

// The decimal digits of a small exponent, with `-` when it is negative.
int_text = (e: Int) -> List(String) {
	{ e < 0 } |> {
		True  -> ["-", ..int_digits(-e, [])]
		False -> int_digits(e, [])
	}
}

int_digits = (e: Int, acc: List(String)) -> List(String) {
	next = [digit_text(e % 10), ..acc]
	{ e < 10 } |> {
		True  -> next
		False -> int_digits(e / 10, next)
	}
}

digit_text = (d: Int) -> String {
	d |> {
		0 -> "0"
		1 -> "1"
		2 -> "2"
		3 -> "3"
		4 -> "4"
		5 -> "5"
		6 -> "6"
		7 -> "7"
		8 -> "8"
		_ -> "9"
	}
}
