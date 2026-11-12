//// Arbitrary precision Int arithmetic beyond the small range (§11.2): the
//// runtime adds, subtracts, multiplies, divides, compares and writes small
//// Ints itself, and calls these functions when an operand is large.
////
//// A magnitude here is a list of digits in base 2^14, least significant
//// first, with no zero digit at the end. Every number this code computes
//// with is below 2^30, so it never makes a large Int itself and never calls
//// itself through the runtime: a digit product is below 2^28.
////
//// The three private primitives convert between an Int and its sign and
//// digits. They are not part of the language (§13).

(
	i/core
)

(
	compare
	add
	subtract
	multiply
	divide
	remainder
	to_text
)

/// -1, 0 or 1.
@external("ilang:big", "sign")
sign: (Int) -> Int

/// The digits of the magnitude, in base 2^14, least significant first.
@external("ilang:big", "digits")
digits: (Int) -> List(Int)

/// The Int with this sign and these digits; zero digits at the end are
/// allowed.
@external("ilang:big", "from_digits")
from_digits: (sign: Int, digits: List(Int)) -> Int

/// -1, 0 or 1, as `a` is less than, equal to or greater than `b`.
compare = (a: Int, b: Int) -> Int {
	sa = sign(a)
	sb = sign(b)
	{ sa == sb } |> {
		False -> { sa < sb } |> {
			True  -> -1
			False -> 1
		}
		True -> { sa == 0 } |> {
			True  -> 0
			False -> sa * magnitude_compare(digits(a), digits(b))
		}
	}
}

/// `a + b`.
add = (a: Int, b: Int) -> Int { signed_add(a, sign(a), b, sign(b)) }

/// `a - b`.
subtract = (a: Int, b: Int) -> Int { signed_add(a, sign(a), b, -sign(b)) }

// `a` with sign `sa` plus the magnitude of `b` with sign `sb`.
signed_add = (a: Int, sa: Int, b: Int, sb: Int) -> Int {
	{ sb == 0 } |> {
		True -> a
		False -> { sa == 0 } |> {
			True -> from_digits(sb, digits(b))
			False -> {
				da = digits(a)
				db = digits(b)
				{ sa == sb } |> {
					True -> from_digits(sa, magnitude_add(da, db, 0, []))
					False -> magnitude_compare(da, db) |> {
						0 -> 0
						1 -> from_digits(sa, magnitude_subtract(da, db, 0, []))
						_ -> from_digits(sb, magnitude_subtract(db, da, 0, []))
					}
				}
			}
		}
	}
}

/// `a * b`.
multiply = (a: Int, b: Int) -> Int {
	s = sign(a) * sign(b)
	{ s == 0 } |> {
		True  -> 0
		False -> {
			da = digits(a)
			db = digits(b)
			// One row for each digit of the shorter one.
			{ length(da, 0) <= length(db, 0) } |> {
				True  -> from_digits(s, magnitude_multiply(da, db, [], []))
				False -> from_digits(s, magnitude_multiply(db, da, [], []))
			}
		}
	}
}

/// `a / b`, rounded toward zero; 0 when `b` is 0 (§11.2).
divide = (a: Int, b: Int) -> Int {
	s = sign(a) * sign(b)
	{ s == 0 } |> {
		True -> 0
		False -> {
			(q, _) = magnitude_divide(digits(a), digits(b))
			from_digits(s, q)
		}
	}
}

/// `a % b`, with the sign of `a`; 0 when `b` is 0 (§11.2).
remainder = (a: Int, b: Int) -> Int {
	sa = sign(a)
	{ sa * sign(b) == 0 } |> {
		True -> 0
		False -> {
			(_, r) = magnitude_divide(digits(a), digits(b))
			from_digits(sa, r)
		}
	}
}

/// The digits of `value` in `radix` (2 to 36), with `-` when it is
/// negative; digits above 9 are capital letters.
to_text = (value: Int, radix: Int) -> String {
	s = sign(value)
	{ s == 0 } |> {
		True -> "0"
		False -> {
			(chunk, width) = chunk_of(radix, radix, 1)
			pieces = text_chunks(digits(value), chunk, radix, width, [])
			{ s < 0 } |> {
				True  -> core.concat(["-", ..pieces])
				False -> core.concat(pieces)
			}
		}
	}
}

// ---- Magnitudes ----

length = (x: List(Int), n: Int) -> Int {
	x |> {
		[]        -> n
		[_, ..xs] -> length(xs, n + 1)
	}
}

reverse = (x: List(Int), acc: List(Int)) -> List(Int) {
	x |> {
		[]        -> acc
		[d, ..ds] -> reverse(ds, [d, ..acc])
	}
}

// Without zero digits at the most significant end.
trim = (x: List(Int)) -> List(Int) { reverse(drop_zeros(reverse(x, [])), []) }

drop_zeros = (x: List(Int)) -> List(Int) {
	x |> {
		[0, ..rest] -> drop_zeros(rest)
		_           -> x
	}
}

// The digit at `index`, or 0 past the end.
at = (x: List(Int), index: Int) -> Int {
	x |> {
		[]        -> 0
		[d, ..ds] -> { index == 0 } |> {
			True  -> d
			False -> at(ds, index - 1)
		}
	}
}

magnitude_compare = (x: List(Int), y: List(Int)) -> Int {
	nx = length(x, 0)
	ny = length(y, 0)
	{ nx == ny } |> {
		True -> compare_from_top(reverse(x, []), reverse(y, []))
		False -> { nx < ny } |> {
			True  -> -1
			False -> 1
		}
	}
}

compare_from_top = (x: List(Int), y: List(Int)) -> Int {
	(x, y) |> {
		([a, ..xs], [b, ..ys]) -> { a == b } |> {
			True -> compare_from_top(xs, ys)
			False -> { a < b } |> {
				True  -> -1
				False -> 1
			}
		}
		_ -> 0
	}
}

// `x + y + carry`, the result reversed onto `acc` and turned round.
magnitude_add = (x: List(Int), y: List(Int), carry: Int, acc: List(Int)) -> List(Int) {
	(x, y) |> {
		([], []) -> reverse([carry, ..acc], [])
		([a, ..xs], []) -> add_digit(a + carry, xs, [], acc)
		([], [b, ..ys]) -> add_digit(b + carry, [], ys, acc)
		([a, ..xs], [b, ..ys]) -> add_digit(a + b + carry, xs, ys, acc)
	}
}

add_digit = (sum: Int, x: List(Int), y: List(Int), acc: List(Int)) -> List(Int) {
	{ sum >= 16384 } |> {
		True  -> magnitude_add(x, y, 1, [sum - 16384, ..acc])
		False -> magnitude_add(x, y, 0, [sum, ..acc])
	}
}

// `x - y - borrow`, where `x` is at least `y`.
magnitude_subtract = (x: List(Int), y: List(Int), borrow: Int, acc: List(Int)) -> List(Int) {
	(x, y) |> {
		([], _) -> trim(reverse(acc, []))
		([a, ..xs], []) -> subtract_digit(a - borrow, xs, [], acc)
		([a, ..xs], [b, ..ys]) -> subtract_digit(a - b - borrow, xs, ys, acc)
	}
}

subtract_digit = (difference: Int, x: List(Int), y: List(Int), acc: List(Int)) -> List(Int) {
	{ difference < 0 } |> {
		True  -> magnitude_subtract(x, y, 1, [difference + 16384, ..acc])
		False -> magnitude_subtract(x, y, 0, [difference, ..acc])
	}
}

// `x × d` for one digit `d`, with the carry `carry`.
multiply_digit = (x: List(Int), d: Int, carry: Int, acc: List(Int)) -> List(Int) {
	x |> {
		[] -> reverse([carry, ..acc], [])
		[a, ..xs] -> {
			p = a * d + carry
			multiply_digit(xs, d, p / 16384, [p % 16384, ..acc])
		}
	}
}

// Schoolbook multiplication: `shift` is the zero digits before each row.
magnitude_multiply = (
	x: List(Int)
	y: List(Int)
	shift: List(Int)
	acc: List(Int)
) -> List(Int) {
	x |> {
		[] -> acc
		[d, ..ds] -> {
			row = { d == 0 } |> {
				True  -> []
				False -> prepend(shift, multiply_digit(y, d, 0, []))
			}
			magnitude_multiply(ds, y, [0, ..shift], magnitude_add(acc, row, 0, []))
		}
	}
}

prepend = (front: List(Int), back: List(Int)) -> List(Int) {
	front |> {
		[]        -> back
		[d, ..ds] -> [d, ..prepend(ds, back)]
	}
}

// Quotient and remainder of magnitudes; `y` is not zero.
magnitude_divide = (x: List(Int), y: List(Int)) -> (List(Int), List(Int)) {
	{ magnitude_compare(x, y) < 0 } |> {
		True -> ([], x)
		False -> y |> {
			[d] -> {
				(q, r) = short_divide(reverse(x, []), d, 0, [])
				(q, [r])
			}
			_ -> {
				// Normalize so that the top digit of the divisor is at
				// least base / 2; then each estimate is at most a few short.
				s = normal_shift(at(y, length(y, 0) - 1), 0)
				ny = shift_left(y, s, 0, [])
				nx = shift_left(x, s, 0, [])
				m = length(ny, 0)
				top = at(ny, m - 1) + 1
				// The first m - 1 digits are below the divisor, so they
				// start the remainder and give no quotient digits.
				(start, rest) = split_top(reverse(nx, []), m - 1, [])
				(q, r) = long_divide(rest, ny, m, top, trim(start), [])
				(q, shift_right(reverse(r, []), s, 0, []))
			}
		}
	}
}

// The first `n` digits of `top`, least significant first, and the rest.
split_top = (top: List(Int), n: Int, acc: List(Int)) -> (List(Int), List(Int)) {
	{ n == 0 } |> {
		True -> (acc, top)
		False -> top |> {
			[]          -> (acc, [])
			[d, ..rest] -> split_top(rest, n - 1, [d, ..acc])
		}
	}
}

normal_shift = (top: Int, s: Int) -> Int {
	{ top >= 8192 } |> {
		True  -> s
		False -> normal_shift(top * 2, s + 1)
	}
}

// `x × 2^s`, for `s` below 14.
shift_left = (x: List(Int), s: Int, carry: Int, acc: List(Int)) -> List(Int) {
	x |> {
		[] -> trim(reverse([carry, ..acc], []))
		[d, ..ds] -> {
			v = d * power_of_two(s, 1) + carry
			shift_left(ds, s, v / 16384, [v % 16384, ..acc])
		}
	}
}

// `x / 2^s`, for `s` below 14, from the most significant digit `top`.
shift_right = (top: List(Int), s: Int, carry: Int, acc: List(Int)) -> List(Int) {
	top |> {
		[] -> trim(acc)
		[d, ..ds] -> {
			p = power_of_two(s, 1)
			v = carry * 16384 + d
			shift_right(ds, s, v % p, [v / p, ..acc])
		}
	}
}

power_of_two = (s: Int, acc: Int) -> Int {
	{ s == 0 } |> {
		True  -> acc
		False -> power_of_two(s - 1, acc * 2)
	}
}

// Division by one digit, from the most significant digit `top`; the
// quotient least significant first, and the remainder.
short_divide = (top: List(Int), d: Int, r: Int, acc: List(Int)) -> (List(Int), Int) {
	top |> {
		[] -> (trim(acc), r)
		[a, ..rest] -> {
			v = r * 16384 + a
			short_divide(rest, d, v % d, [v / d, ..acc])
		}
	}
}

// Long division, one digit of the dividend at a time from the most
// significant one (`top`). The remainder `r` stays below `y × base`, so
// each quotient digit is below `base`. The estimate divides the top two
// digits of `r` by the top digit of `y` plus one, so it is never too big.
long_divide = (
	top: List(Int)
	y: List(Int)
	m: Int
	divisor_top: Int
	r: List(Int)
	q: List(Int)
) -> (List(Int), List(Int)) {
	top |> {
		[] -> (trim(q), r)
		[a, ..rest] -> {
			shifted = { r == [] && a == 0 } |> {
				True  -> []
				False -> [a, ..r]
			}
			estimate = { at(shifted, m) * 16384 + at(shifted, m - 1) } / divisor_top
			reduced = { estimate == 0 } |> {
				True  -> shifted
				False -> magnitude_subtract(shifted, trim(multiply_digit(y, estimate, 0, [])), 0, [])
			}
			(digit, left) = correct(reduced, y, estimate)
			long_divide(rest, y, m, divisor_top, left, [digit, ..q])
		}
	}
}

correct = (r: List(Int), y: List(Int), digit: Int) -> (Int, List(Int)) {
	{ magnitude_compare(r, y) >= 0 } |> {
		True  -> correct(magnitude_subtract(r, y, 0, []), y, digit + 1)
		False -> (digit, r)
	}
}

// ---- Text ----

// The largest power of `radix` that is at most 2^16, and its exponent.
chunk_of = (radix: Int, chunk: Int, width: Int) -> (Int, Int) {
	next = chunk * radix
	{ next > 65536 } |> {
		True  -> (chunk, width)
		False -> chunk_of(radix, next, width + 1)
	}
}

// Divides by `chunk` until nothing is left; each remainder is `width`
// digits of the text, and the last one has no zeros in front.
text_chunks = (
	x: List(Int)
	chunk: Int
	radix: Int
	width: Int
	acc: List(String)
) -> List(String) {
	(q, r) = short_divide(reverse(x, []), chunk, 0, [])
	{ q == [] } |> {
		True  -> chunk_text(r, radix, 0, False, acc)
		False -> text_chunks(q, chunk, radix, width, chunk_text(r, radix, width, True, acc))
	}
}

// The digits of `v` in `radix` before `acc`, at least `width` of them
// when `padded`.
chunk_text = (v: Int, radix: Int, width: Int, padded: Bool, acc: List(String)) -> List(String) {
	next = [digit_text(v % radix), ..acc]
	rest = v / radix
	{ rest == 0 && { core.not(padded) || width <= 1 } } |> {
		True  -> next
		False -> chunk_text(rest, radix, width - 1, padded, next)
	}
}

digit_text = (d: Int) -> String {
	d |> {
		0  -> "0"
		1  -> "1"
		2  -> "2"
		3  -> "3"
		4  -> "4"
		5  -> "5"
		6  -> "6"
		7  -> "7"
		8  -> "8"
		9  -> "9"
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
		_  -> "Z"
	}
}
