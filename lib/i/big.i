//// Arbitrary precision Int arithmetic beyond the small range (§11.2): the
//// runtime adds, subtracts, multiplies, divides, compares and writes small
//// Ints itself, and calls these functions when an operand is large.
////
//// The runtime keeps a large Int as its sign and an array of 32-bit limbs.
//// This code reads its digits in base 2^30 in place, by index (`size`,
//// `limb`), and writes each result once into private construction storage.
//// Only the text form works on lists of operand digits
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

// Private construction storage. Each digit is written once, and the storage
// is never used again after finish; no published Int is changed.
Digits {}

@external("ilang:big", "builder")
builder: (Int) -> Digits

@external("ilang:big", "put")
put: (Digits, Int, Int) -> Digits

@external("ilang:big", "finish")
finish: (Int, Digits) -> Int

// One base-2^30 multiply-add, split into low digit and carry.
@external("ilang:big", "multiply_add")
multiply_add: (Int, Int, Int) -> (Int, Int)

// Divide high * base + low by d; high < d. Quotient and remainder.
@external("ilang:big", "divide_digit")
divide_digit: (Int, Int, Int) -> (Int, Int)

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
				True -> {
					n = max(size(a), size(b))
					finish(sa, add_up(a, b, 0, n, 0, builder(n + 1)))
				}
				False -> magnitude_order(a, b) |> {
					0 -> 0
					1 -> finish(
						sa
						subtract_up(a, b, 0, size(a), 0, builder(size(a)))
					)
					_ -> finish(
						sb
						subtract_up(b, a, 0, size(b), 0, builder(size(b)))
					)
				}
			}
		}
	}
}

// Write |a| + |b| from digit i up.
add_up = (a: Int, b: Int, i: Int, n: Int, carry: Int, out: Digits) -> Digits {
	{ i == n } |> {
		True -> put(out, i, carry)
		False -> {
			sum = limb(a, i) + limb(b, i) + carry
			{ sum >= 1073741824 } |> {
				True  -> add_up(a, b, i + 1, n, 1, put(out, i, sum - 1073741824))
				False -> add_up(a, b, i + 1, n, 0, put(out, i, sum))
			}
		}
	}
}

// Write |a| - |b|, where |a| is at least |b|.
subtract_up = (
	a: Int
	b: Int
	i: Int
	n: Int
	borrow: Int
	out: Digits
) -> Digits {
	{ i == n } |> {
		True -> out
		False -> {
			difference = limb(a, i) - limb(b, i) - borrow
			{ difference < 0 } |> {
				True -> subtract_up(
					a
					b
					i + 1
					n
					1
					put(out, i, difference + 1073741824)
				)
				False -> subtract_up(a, b, i + 1, n, 0, put(out, i, difference))
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
					finish(s, scale_up(x, d, 0, n, 0, builder(n + 1)))
				}
				False -> finish(
					s
					columns(a, b, na, nb, 0, 0, builder(na + nb))
				)
			}
		}
	}
}

// |x| × d for one digit d.
scale_up = (x: Int, d: Int, i: Int, n: Int, carry: Int, out: Digits) -> Digits {
	{ i == n } |> {
		True -> put(out, i, carry)
		False -> {
			(low, high) = multiply_add(limb(x, i), d, carry)
			scale_up(x, d, i + 1, n, high, put(out, i, low))
		}
	}
}

// Product scanning: accumulate one column at a time. The low accumulator
// stays one digit; each multiply-add contributes its carry to the high sum.
columns = (
	a: Int
	b: Int
	na: Int
	nb: Int
	k: Int
	carry: Int
	out: Digits
) -> Digits {
	{ k == na + nb - 1 } |> {
		True -> put(out, k, carry)
		False -> {
			first = max(0, k - nb + 1)
			(low, high) = column(a, b, first, min(k, na - 1), k, 0, 0)
			total = low + carry
			next = total / 1073741824 + high
			columns(a, b, na, nb, k + 1, next, put(out, k, total % 1073741824))
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
			(next, carry) = multiply_add(limb(a, i), limb(b, k - i), low)
			column(a, b, i + 1, last, k, next, high + carry)
		}
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
			{ s < 0 } |> {
				True  -> -q
				False -> q
			}
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

// Positive quotient and remainder. Normalize each operand once, then read
// both numbers directly by offset, with no operand lists.
magnitude_divide = (a: Int, b: Int) -> (Int, Int) {
	{ magnitude_order(a, b) < 0 } |> {
		True -> (0, { sign(a) < 0 } |> {
			True  -> -a
			False -> a
		})
		False -> {
			m = size(b)
			{ m == 1 } |> {
				True -> short_divide_number(
					a
					limb(b, 0)
					size(a) - 1
					0
					builder(size(a))
				)
				False -> {
					factor = power_of_two(normal_shift(limb(b, m - 1), 0), 1)
					x = normalized(a, factor)
					y = normalized(b, factor)
					n = size(x)
					start = finish(
						1
						initial_remainder(
							x
							n - m + 1
							0
							m - 1
							builder(m - 1)
						)
					)
					(q, r) = long_divide(
						x
						y
						n - m
						m
						limb(y, m - 1) + 1
						start
						builder(n - m + 1)
					)
					{ factor == 1 } |> {
						True -> (q, r)
						False -> {
							(rest, _) = short_divide_number(
								r
								factor
								size(r) - 1
								0
								builder(size(r))
							)
							(q, rest)
						}
					}
				}
			}
		}
	}
}

normalized = (x: Int, factor: Int) -> Int {
	{ factor == 1 } |> {
		True -> x
		False -> finish(
			1
			scale_up(x, factor, 0, size(x), 0, builder(size(x) + 1))
		)
	}
}

// The top m - 1 dividend digits are below the normalized divisor.
initial_remainder = (
	x: Int
	offset: Int
	i: Int
	n: Int
	out: Digits
) -> Digits {
	{ i == n } |> {
		True -> out
		False -> initial_remainder(
			x
			offset
			i + 1
			n
			put(out, i, limb(x, offset + i))
		)
	}
}

short_divide_number = (
	x: Int
	d: Int
	i: Int
	r: Int
	out: Digits
) -> (Int, Int) {
	{ i < 0 } |> {
		True -> (finish(1, out), r)
		False -> {
			(q, next) = divide_digit(r, limb(x, i), d)
			short_divide_number(x, d, i - 1, next, put(out, i, q))
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
	q: Digits
) -> (Int, Int) {
	{ i < 0 } |> {
		True -> (finish(1, q), r)
		False -> {
			a = limb(x, i)
			(estimate, _) = divide_digit(limb(r, m - 1), limb(r, m - 2), top)
			reduced = finish(
				1
				reduce_digit(r, a, y, estimate, 0, m + 1, 0, 0, builder(m + 1))
			)
			(digit, left) = correct(reduced, y, estimate)
			long_divide(x, y, i - 1, m, top, left, put(q, i, digit))
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
	out: Digits
) -> Digits {
	{ i == n } |> {
		True -> out
		False -> {
			(low, high) = multiply_add(limb(y, i), d, carry)
			x = { i == 0 } |> {
				True  -> a
				False -> limb(r, i - 1)
			}
			difference = x - low - borrow
			{ difference < 0 } |> {
				True -> reduce_digit(
					r
					a
					y
					d
					i + 1
					n
					high
					1
					put(out, i, difference + 1073741824)
				)
				False -> reduce_digit(
					r
					a
					y
					d
					i + 1
					n
					high
					0
					put(out, i, difference)
				)
			}
		}
	}
}

correct = (r: Int, y: Int, digit: Int) -> (Int, Int) {
	{ magnitude_order(r, y) >= 0 } |> {
		True -> correct(
			finish(1, subtract_up(r, y, 0, size(r), 0, builder(size(r))))
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
			(q, next) = divide_digit(r, a, d)
			short_divide(rest, d, next, [q, ..acc])
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
