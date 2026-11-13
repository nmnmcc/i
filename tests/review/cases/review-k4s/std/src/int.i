//// Functions for `Int`, the type of whole numbers of any size.
////
//// Ints never overflow (§11.2). `a / b` rounds toward zero, `a % b` has the
//// sign of `a`, and both are `0` when `b` is `0`. The functions `divide`,
//// `remainder`, `modulo`, and `floor_divide` give `Error(Nil)` for `0`
//// instead.
(
	i/core
	order = (Order, Less, Equal, Greater) = std/order
)

(
	to_string
	parse
	parse_base
	to_base
	to_base2
	to_base8
	to_base16
	to_base36
	to_float
	abs
	negate
	min
	max
	clamp
	compare
	is_even
	is_odd
	power
	square_root
	sum
	product
	digits
	undigits
	divide
	remainder
	modulo
	floor_divide
	add
	subtract
	multiply
	bitwise_and
	bitwise_or
	bitwise_xor
	bitwise_not
	bitwise_shift_left
	bitwise_shift_right
	range
)

/// The number in decimal digits, with `-` before a negative number.
///
/// ```i
/// int.to_string(-42)  // "-42"
/// ```
to_string = (x : Int) -> String { core.inspect(x) }

/// Reads a decimal number: an optional `-` or `+`, then one or more digits
/// `0` to `9`. Any other text gives `Error(Nil)`.
///
/// ```i
/// int.parse("-42")  // Ok(-42)
/// int.parse("4.2")  // Error(Nil)
/// ```
parse = (text : String) -> Result(Int, Nil) { parse_base(text, 10) }

/// Reads a number in the given base, from 2 to 36: an optional `-` or `+`,
/// then one or more digits. Digits after `9` are the letters `a` to `z`, in
/// lower or upper case. A base outside 2 to 36, or any other text, gives
/// `Error(Nil)`.
///
/// ```i
/// int.parse_base("ff", 16)  // Ok(255)
/// ```
parse_base = (text : String, base : Int) -> Result(Int, Nil) {
	{ base >= 2 && base <= 36 } |>
		(False) { Error(Nil) }
		| (True) {
			<<text:utf8>> |>
				(<<45, rest:bytes>>) {
					parse_digits(rest, base, 0, False) |>
						(Ok(n)) { Ok(-n) }
						| (Error(e)) { Error(e) }
				}
				| (<<43, rest:bytes>>) { parse_digits(rest, base, 0, False) }
				| (bytes) { parse_digits(bytes, base, 0, False) }
		}
}

parse_digits = (bytes : BitArray, base : Int, acc : Int, seen : Bool) -> Result(
	Int,
	Nil,
) {
	bytes |>
		(<<>>) { seen |> (True) { Ok(acc) } | (False) { Error(Nil) } }
		| (<<byte, rest:bytes>>) {
			d = digit_value(byte)
			{ d >= 0 && d < base } |>
				(True) { parse_digits(rest, base, acc * base + d, True) }
				| (False) { Error(Nil) }
		}
		| (_) { Error(Nil) }
}

// The value of one ASCII digit or letter, or -1.
digit_value = (byte : Int) -> Int {
	{ byte >= 48 && byte <= 57 } |>
		(True) { byte - 48 }
		| (False) {
			{ byte >= 97 && byte <= 122 } |>
				(True) { byte - 87 }
				| (False) {
					{ byte >= 65 && byte <= 90 } |>
						(True) { byte - 55 }
						| (False) { -1 }
				}
		}
}

/// The number in the given base, from 2 to 36, with upper-case letters for
/// the digits after `9`. A base outside 2 to 36 gives `Error(Nil)`.
///
/// ```i
/// int.to_base(255, 16)  // Ok("FF")
/// int.to_base(-5, 2)    // Ok("-101")
/// ```
to_base = (x : Int, base : Int) -> Result(String, Nil) {
	{ base >= 2 && base <= 36 } |>
		(False) { Error(Nil) }
		| (True) { Ok(base_string(x, base)) }
}

base_string = (x : Int, base : Int) -> String {
	x |>
		(0) { "0" }
		| (_) {
			go = (n, acc) {
				n |>
					(0) { acc }
					| (_) { go(n / base, [digit_char(n % base), ..acc]) }
			}
			{ x < 0 } |>
				(True) { core.concat(["-", ..go(-x, [])]) }
				| (False) { core.concat(go(x, [])) }
		}
}

digit_char = (d : Int) -> String {
	core.byte_slice("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ", d, 1) |>
		(Ok(s)) { s }
		| (Error(_)) { "?" }
}

/// The number in base 2.
to_base2 = (x : Int) -> String { base_string(x, 2) }

/// The number in base 8.
to_base8 = (x : Int) -> String { base_string(x, 8) }

/// The number in base 16, with upper-case letters.
to_base16 = (x : Int) -> String { base_string(x, 16) }

/// The number in base 36, with upper-case letters.
to_base36 = (x : Int) -> String { base_string(x, 36) }

/// The nearest Float. A number too big for a Float stops the program.
to_float = (x : Int) -> Float { core.to_float(x) }

/// The number without its sign.
abs = (x : Int) -> Int { { x < 0 } |> (True) { -x } | (False) { x } }

/// The number with the other sign.
negate = (x : Int) -> Int { -x }

/// The smaller of two numbers.
min = (a : Int, b : Int) -> Int { { a < b } |> (True) { a } | (False) { b } }

/// The greater of two numbers.
max = (a : Int, b : Int) -> Int { { a > b } |> (True) { a } | (False) { b } }

/// The number, but not less than `min` and not greater than `max`. When
/// `min` is greater than `max`, the two bounds are swapped.
///
/// ```i
/// int.clamp(40, 50, 60)  // 50
/// ```
clamp = (x : Int, min : Int, max : Int) -> Int {
	(low, high) = { min <= max } |>
		(True) { (min, max) }
		| (False) { (max, min) }
	{ x < low } |>
		(True) { low }
		| (False) { { x > high } |> (True) { high } | (False) { x } }
}

/// Compares two numbers.
///
/// ```i
/// int.compare(1, 2)  // Less
/// ```
compare = (a : Int, with : Int) -> Order {
	{ a == with } |>
		(True) { Equal }
		| (False) { { a < with } |> (True) { Less } | (False) { Greater } }
}

/// `True` when the number is a multiple of 2.
is_even = (x : Int) -> Bool { x % 2 == 0 }

/// `True` when the number is not a multiple of 2.
is_odd = (x : Int) -> Bool { x % 2 != 0 }

/// `base` multiplied by itself `of` times. A negative exponent gives
/// `Error(Nil)`, because the result would not be a whole number.
///
/// ```i
/// int.power(2, 10)  // Ok(1024)
/// ```
power = (base : Int, of : Int) -> Result(Int, Nil) {
	{ of < 0 } |> (True) { Error(Nil) } | (False) { Ok(int_power(base, of, 1)) }
}

int_power = (base : Int, exponent : Int, acc : Int) -> Int {
	exponent |>
		(0) { acc }
		| (_) {
			next = exponent % 2 |> (0) { acc } | (_) { acc * base }
			int_power(base * base, exponent / 2, next)
		}
}

/// The square root as a Float. A negative number gives `Error(Nil)`.
///
/// ```i
/// int.square_root(16)  // Ok(4.0)
/// ```
square_root = (x : Int) -> Result(Float, Nil) {
	{ x < 0 } |>
		(True) { Error(Nil) }
		| (False) { Ok(core.sqrt(core.to_float(x))) }
}

/// The sum of the numbers. The sum of no numbers is `0`.
sum = (numbers : List(Int)) -> Int {
	go = (rest, acc) { rest |> ([]) { acc } | ([x, ..xs]) { go(xs, acc + x) } }
	go(numbers, 0)
}

/// The product of the numbers. The product of no numbers is `1`.
product = (numbers : List(Int)) -> Int {
	go = (rest, acc) { rest |> ([]) { acc } | ([x, ..xs]) { go(xs, acc * x) } }
	go(numbers, 1)
}

/// The digits of a number in a base of 2 or more, highest first. The sign
/// is not kept. A base less than 2 gives `Error(Nil)`.
///
/// ```i
/// int.digits(234, 10)  // Ok([2, 3, 4])
/// ```
digits = (x : Int, base : Int) -> Result(List(Int), Nil) {
	{ base < 2 } |>
		(True) { Error(Nil) }
		| (False) {
			go = (n, acc) {
				{ n < base } |>
					(True) { [n, ..acc] }
					| (False) { go(n / base, [n % base, ..acc]) }
			}
			Ok(go(abs(x), []))
		}
}

/// The number made from digits in a base of 2 or more, highest first. A
/// base less than 2, or a digit that is not from `0` to `base - 1`, gives
/// `Error(Nil)`.
///
/// ```i
/// int.undigits([2, 3, 4], 10)  // Ok(234)
/// ```
undigits = (numbers : List(Int), base : Int) -> Result(Int, Nil) {
	{ base < 2 } |>
		(True) { Error(Nil) }
		| (False) {
			go = (rest, acc) {
				rest |>
					([]) { Ok(acc) }
					| ([d, ..ds]) {
						{ d >= 0 && d < base } |>
							(True) { go(ds, acc * base + d) }
							| (False) { Error(Nil) }
					}
			}
			go(numbers, 0)
		}
}

/// `a / by`, rounded toward zero, or `Error(Nil)` when `by` is `0`.
divide = (a : Int, by : Int) -> Result(Int, Nil) {
	by |> (0) { Error(Nil) } | (_) { Ok(a / by) }
}

/// The remainder of `a / by`, with the sign of `a`, or `Error(Nil)` when
/// `by` is `0`.
///
/// ```i
/// int.remainder(-13, 3)  // Ok(-1)
/// ```
remainder = (a : Int, by : Int) -> Result(Int, Nil) {
	by |> (0) { Error(Nil) } | (_) { Ok(a % by) }
}

/// The remainder of `a` divided by `by`, rounded down, with the sign of
/// `by`, or `Error(Nil)` when `by` is `0`.
///
/// ```i
/// int.modulo(-13, 3)  // Ok(2)
/// ```
modulo = (a : Int, by : Int) -> Result(Int, Nil) {
	by |>
		(0) { Error(Nil) }
		| (_) {
			r = a % by
			{ r != 0 && { r < 0 } != { by < 0 } } |>
				(True) { Ok(r + by) }
				| (False) { Ok(r) }
		}
}

/// `a / by`, rounded down, or `Error(Nil)` when `by` is `0`.
///
/// ```i
/// int.floor_divide(-7, 2)  // Ok(-4)
/// ```
floor_divide = (a : Int, by : Int) -> Result(Int, Nil) {
	by |>
		(0) { Error(Nil) }
		| (_) {
			q = a / by
			{ a % by != 0 && { a < 0 } != { by < 0 } } |>
				(True) { Ok(q - 1) }
				| (False) { Ok(q) }
		}
}

/// `a + b`, as a function.
add = (a : Int, b : Int) -> Int { a + b }

/// `a - b`, as a function.
subtract = (a : Int, b : Int) -> Int { a - b }

/// `a * b`, as a function.
multiply = (a : Int, b : Int) -> Int { a * b }

/// The bits that are set in both numbers (two's complement).
bitwise_and = (x : Int, y : Int) -> Int { core.bit_and(x, y) }

/// The bits that are set in one or both numbers (two's complement).
bitwise_or = (x : Int, y : Int) -> Int { core.bit_or(x, y) }

/// The bits that are set in exactly one of the numbers (two's complement).
bitwise_xor = (x : Int, y : Int) -> Int { core.bit_xor(x, y) }

/// Every bit changed: `-x - 1`.
bitwise_not = (x : Int) -> Int { core.bit_not(x) }

/// `x` × 2^`y`.
bitwise_shift_left = (x : Int, y : Int) -> Int { core.shift_left(x, y) }

/// `x` ÷ 2^`y`, rounded down.
bitwise_shift_right = (x : Int, y : Int) -> Int { core.shift_right(x, y) }

/// Folds over the numbers from `from` to `to`, with `from` but without
/// `to`. It counts up or down, as needed.
///
/// ```i
/// int.range(0, 3, [], (acc, i) { [i, ..acc] })  // [2, 1, 0]
/// ```
range = <Acc, N>(
	from : Int,
	to : Int,
	with : Acc,
	run : (Acc, Int) <- N -> Acc
) -> Acc {
	step = { from <= to } |> (True) { 1 } | (False) { -1 }
	go = (i, acc) {
		{ i == to } |> (True) { acc } | (False) { go(i + step, run(acc, i)) }
	}
	go(from, with)
}
