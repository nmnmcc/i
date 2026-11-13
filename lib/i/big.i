//// Arbitrary precision Int arithmetic beyond the small range (§11.2): the
//// runtime adds, subtracts, multiplies, divides, compares and writes small
//// Ints itself, and calls these functions when an operand is large.
////
//// The runtime keeps a large Int as its sign and an array of 32-bit limbs.
//// This code reads its digits in base 2^30 in place, by index (`size`,
//// `limb`), and builds each result once from its digits (`from_top`,
//// `from_digits`). Only the text form works on lists of operand digits
//// (`digits`). Every single-digit calculation is below 2^62, where
//// the runtime does the arithmetic in 64 bits itself, so this code never
//// calls itself through the runtime: a digit product is below 2^60.
////
//// The private primitives are not part of the language (§13).

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

/// The digits of the magnitude, in base 2^30, least significant first.
@external("ilang:big", "digits")
digits: (Int) -> List(Int)

/// The Int with this sign and these digits; zero digits at the end are
/// allowed.
@external("ilang:big", "from_digits")
from_digits: (sign: Int, digits: List(Int)) -> Int

/// The number of digits of the magnitude, in base 2^30.
@external("ilang:big", "size")
size: (Int) -> Int

/// The digit of the magnitude at `index`, in base 2^30; 0 past the end.
@external("ilang:big", "limb")
limb: (x: Int, index: Int) -> Int

/// The Int with this sign and these digits in base 2^30, most significant
/// first; zero digits in front are allowed.
@external("ilang:big", "from_top")
from_top: (sign: Int, digits: List(Int)) -> Int

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
			False -> sa * magnitude_order(a, b)
		}
	}
}

// -1, 0 or 1 for the magnitudes, digit by digit from the top.
magnitude_order = (a: Int, b: Int) -> Int {
	na = size(a)
	nb = size(b)
	{ na == nb } |> {
		True -> order_from(a, b, na - 1)
		False -> { na < nb } |> {
			True  -> -1
			False -> 1
		}
	}
}

order_from = (a: Int, b: Int, i: Int) -> Int {
	{ i < 0 } |> {
		True -> 0
		False -> {
			x = limb(a, i)
			y = limb(b, i)
			{ x == y } |> {
				True -> order_from(a, b, i - 1)
				False -> { x < y } |> {
					True  -> -1
					False -> 1
				}
			}
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
			True -> { sb == sign(b) } |> {
				True  -> b
				False -> -b
			}
			False -> { sa == sb } |> {
				True -> from_top(
					sa
					add_up(a, b, 0, max(size(a), size(b)), 0, [])
				)
				False -> magnitude_order(a, b) |> {
					0 -> 0
					1 -> from_top(sa, subtract_up(a, b, 0, size(a), 0, []))
					_ -> from_top(sb, subtract_up(b, a, 0, size(b), 0, []))
				}
			}
		}
	}
}

// The digits of |a| + |b| from digit `i` up, most significant first in
// front of `acc`.
add_up = (
	a: Int
	b: Int
	i: Int
	n: Int
	carry: Int
	acc: List(Int)
) -> List(Int) {
	{ i == n } |> {
		True -> [carry, ..acc]
		False -> {
			sum = limb(a, i) + limb(b, i) + carry
			{ sum >= 1073741824 } |> {
				True  -> add_up(a, b, i + 1, n, 1, [sum - 1073741824, ..acc])
				False -> add_up(a, b, i + 1, n, 0, [sum, ..acc])
			}
		}
	}
}

// The digits of |a| - |b|, where |a| is at least |b|.
subtract_up = (
	a: Int
	b: Int
	i: Int
	n: Int
	borrow: Int
	acc: List(Int)
) -> List(Int) {
	{ i == n } |> {
		True -> acc
		False -> {
			difference = limb(a, i) - limb(b, i) - borrow
			{ difference < 0 } |> {
				True -> subtract_up(
					a
					b
					i + 1
					n
					1
					[difference + 1073741824, ..acc]
				)
				False -> subtract_up(a, b, i + 1, n, 0, [difference, ..acc])
			}
		}
	}
}

/// `a * b`.
multiply = (a: Int, b: Int) -> Int {
	s = sign(a) * sign(b)
	{ s == 0 } |> {
		True -> 0
		False -> {
			na = size(a)
			nb = size(b)
			{ na == 1 || nb == 1 } |> {
				True -> {
					(x, d, n) = { na == 1 } |> {
						True  -> (b, limb(a, 0), nb)
						False -> (a, limb(b, 0), na)
					}
					from_top(s, scale_up(x, d, 0, n, 0, []))
				}
				False -> from_top(s, columns(a, b, na, nb, 0, 0, []))
			}
		}
	}
}

// |x| × d for one digit d.
scale_up = (
	x: Int
	d: Int
	i: Int
	n: Int
	carry: Int
	acc: List(Int)
) -> List(Int) {
	{ i == n } |> {
		True -> [carry, ..acc]
		False -> {
			p = limb(x, i) * d + carry
			scale_up(x, d, i + 1, n, p / 1073741824, [p % 1073741824, ..acc])
		}
	}
}

// Product scanning: digit k of |a| × |b| is the sum of the products of the
// digits i and k - i, with the carry of the digits below. Each product is
// split at 2^30, so the two sums stay below 2^62 for any size.
columns = (
	a: Int
	b: Int
	na: Int
	nb: Int
	k: Int
	carry: Int
	acc: List(Int)
) -> List(Int) {
	{ k == na + nb - 1 } |> {
		True -> spill(carry, acc)
		False -> {
			first = max(0, k - nb + 1)
			(low, high) = column(a, b, first, min(k, na - 1), k, 0, 0)
			total = low + carry
			next = total / 1073741824 + high
			columns(a, b, na, nb, k + 1, next, [total % 1073741824, ..acc])
		}
	}
}

column = (
	a: Int
	b: Int
	i: Int
	last: Int
	k: Int
	low: Int
	high: Int
) -> (Int, Int) {
	{ i > last } |> {
		True -> (low, high)
		False -> {
			p = limb(a, i) * limb(b, k - i)
			column(
				a
				b
				i + 1
				last
				k
				low + p % 1073741824
				high + p / 1073741824
			)
		}
	}
}

// The digits of the last carry in front of `acc`.
spill = (carry: Int, acc: List(Int)) -> List(Int) {
	{ carry == 0 } |> {
		True  -> acc
		False -> spill(carry / 1073741824, [carry % 1073741824, ..acc])
	}
}

max = (a: Int, b: Int) -> Int {
	{ a > b } |> {
		True  -> a
		False -> b
	}
}

min = (a: Int, b: Int) -> Int {
	{ a < b } |> {
		True  -> a
		False -> b
	}
}

/// `a / b`, rounded toward zero; 0 when `b` is 0 (§11.2).
divide = (a: Int, b: Int) -> Int {
	s = sign(a) * sign(b)
	{ s == 0 } |> {
		True -> 0
		False -> {
			(q, _) = magnitude_divide(a, b)
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
			(_, r) = magnitude_divide(a, b)
			{ sa < 0 } |> {
				True  -> -r
				False -> r
			}
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

// ---- Division ----

// Quotient digits, least significant first, and the positive remainder.
// Normalize each operand once, then read both numbers directly by offset.
magnitude_divide = (a: Int, b: Int) -> (List(Int), Int) {
	{ magnitude_order(a, b) < 0 } |> {
		True -> ([], { sign(a) < 0 } |> {
			True  -> -a
			False -> a
		})
		False -> {
			m = size(b)
			{ m == 1 } |> {
				True -> short_divide_number(a, limb(b, 0), size(a) - 1, 0, [])
				False -> {
					factor = power_of_two(normal_shift(limb(b, m - 1), 0), 1)
					x = normalized(a, factor)
					y = normalized(b, factor)
					n = size(x)
					start = from_digits(
						1
						initial_remainder(x, n - 1, n - m, [])
					)
					(q, r) = long_divide(
						x
						y
						n - m
						m
						limb(y, m - 1) + 1
						start
						[]
					)
					{ factor == 1 } |> {
						True -> (q, r)
						False -> {
							(rest, _) = short_divide_number(
								r
								factor
								size(r) - 1
								0
								[]
							)
							(q, from_digits(1, rest))
						}
					}
				}
			}
		}
	}
}

normalized = (x: Int, factor: Int) -> Int {
	{ factor == 1 } |> {
		True  -> x
		False -> from_top(1, scale_up(x, factor, 0, size(x), 0, []))
	}
}

// The top m - 1 dividend digits are below the normalized divisor.
initial_remainder = (x: Int, i: Int, stop: Int, acc: List(Int)) -> List(Int) {
	{ i == stop } |> {
		True  -> acc
		False -> initial_remainder(x, i - 1, stop, [limb(x, i), ..acc])
	}
}

short_divide_number = (
	x: Int
	d: Int
	i: Int
	r: Int
	acc: List(Int)
) -> (List(Int), Int) {
	{ i < 0 } |> {
		True -> (acc, r)
		False -> {
			v = r * 1073741824 + limb(x, i)
			short_divide_number(x, d, i - 1, v % d, [v / d, ..acc])
		}
	}
}

// Bring down one dividend digit. Estimate from the top two digits of the
// shifted remainder, dividing by divisor_top + 1 so it cannot be too big.
long_divide = (
	x: Int
	y: Int
	i: Int
	m: Int
	top: Int
	r: Int
	q: List(Int)
) -> (List(Int), Int) {
	{ i < 0 } |> {
		True -> (q, r)
		False -> {
			a = limb(x, i)
			estimate = { limb(r, m - 1) * 1073741824 + limb(r, m - 2) } / top
			reduced = from_top(
				1
				reduce_digit(r, a, y, estimate, 0, m + 1, 0, 0, [])
			)
			(digit, left) = correct(reduced, y, estimate)
			long_divide(x, y, i - 1, m, top, left, [digit, ..q])
		}
	}
}

// r * base + a - y * d. Fuse the one-digit product and subtraction so
// there is no temporary product or list copy of either operand.
reduce_digit = (
	r: Int
	a: Int
	y: Int
	d: Int
	i: Int
	n: Int
	carry: Int
	borrow: Int
	acc: List(Int)
) -> List(Int) {
	{ i == n } |> {
		True -> acc
		False -> {
			p = limb(y, i) * d + carry
			x = { i == 0 } |> {
				True  -> a
				False -> limb(r, i - 1)
			}
			difference = x - p % 1073741824 - borrow
			{ difference < 0 } |> {
				True -> reduce_digit(
					r
					a
					y
					d
					i + 1
					n
					p / 1073741824
					1
					[difference + 1073741824, ..acc]
				)
				False -> reduce_digit(
					r
					a
					y
					d
					i + 1
					n
					p / 1073741824
					0
					[difference, ..acc]
				)
			}
		}
	}
}

correct = (r: Int, y: Int, digit: Int) -> (Int, Int) {
	{ magnitude_order(r, y) >= 0 } |> {
		True -> correct(
			from_top(1, subtract_up(r, y, 0, size(r), 0, []))
			y
			digit + 1
		)
		False -> (digit, r)
	}
}

normal_shift = (top: Int, s: Int) -> Int {
	{ top >= 536870912 } |> {
		True  -> s
		False -> normal_shift(top * 2, s + 1)
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
short_divide = (
	top: List(Int)
	d: Int
	r: Int
	acc: List(Int)
) -> (List(Int), Int) {
	top |> {
		[] -> (trim(acc), r)
		[a, ..rest] -> {
			v = r * 1073741824 + a
			short_divide(rest, d, v % d, [v / d, ..acc])
		}
	}
}

// Text conversion alone uses lists of digits.
reverse = (x: List(Int), acc: List(Int)) -> List(Int) {
	x |> {
		[]        -> acc
		[d, ..ds] -> reverse(ds, [d, ..acc])
	}
}

trim = (x: List(Int)) -> List(Int) { reverse(drop_zeros(reverse(x, [])), []) }

drop_zeros = (x: List(Int)) -> List(Int) {
	x |> {
		[0, ..rest] -> drop_zeros(rest)
		_           -> x
	}
}

// ---- Text ----

// The largest power of `radix` that is below 2^30, and its exponent.
chunk_of = (radix: Int, chunk: Int, width: Int) -> (Int, Int) {
	next = chunk * radix
	{ next >= 1073741824 } |> {
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
		True -> chunk_text(r, radix, 0, False, acc)
		False -> text_chunks(
			q
			chunk
			radix
			width
			chunk_text(r, radix, width, True, acc)
		)
	}
}

// The digits of `v` in `radix` before `acc`, at least `width` of them
// when `padded`.
chunk_text = (
	v: Int
	radix: Int
	width: Int
	padded: Bool
	acc: List(String)
) -> List(String) {
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
