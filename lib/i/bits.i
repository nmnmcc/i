//// Integer segments of bit arrays (§8.3, §8.4) that do not fit 63 bits.
//// The runtime reads and writes narrower segments itself, with its fixed
//// width primitives, and calls these functions for the others.
////
//// A segment of `width` bits holds its value in two's complement. Big
//// endian puts the most significant bit first. Little endian puts the
//// groups of 8 bits least significant group first, each group most
//// significant bit first; the last group may be shorter.
////
//// The two private primitives read and write at most 14 bits, big endian.
//// They are not part of the language (§13).
(
	i/core
)

(
	read_int
	from_int
)

/// The `width` bits at `start`, big endian, as a number from 0.
@external("ilang:bits", "read_unsigned")
read_unsigned : (BitArray, Int, Int) -> Int

/// `value`, from 0 to 2^width - 1, as `width` bits, big endian.
@external("ilang:bits", "from_unsigned")
from_unsigned : (Int, Int) -> BitArray

/// The Int in the `width` bits of `bits` at `start`.
read_int = (
	bits : BitArray,
	start : Int,
	width : Int,
	signed : Bool,
	little : Bool
) -> Int {
	value = little |>
		(True) { read_little(bits, start, width, 0, 1) }
		| (False) { read_big(bits, start, width, 0) }
	{ signed && width > 0 && value >= core.shift_left(1, width - 1) } |>
		(True) { value - core.shift_left(1, width) }
		| (False) { value }
}

// Big endian: the bits in chunks, most significant first.
read_big = (bits : BitArray, start : Int, left : Int, acc : Int) -> Int {
	{ left == 0 } |>
		(True) { acc }
		| (False) {
			n = min(left, 14)
			read_big(
				bits,
				start + n,
				left - n,
				core.shift_left(acc, n) + read_unsigned(bits, start, n),
			)
		}
}

// Little endian: groups of 8 bits, least significant first; `scale` is the
// value of the lowest bit of the group.
read_little = (
	bits : BitArray,
	start : Int,
	left : Int,
	acc : Int,
	scale : Int
) -> Int {
	{ left == 0 } |>
		(True) { acc }
		| (False) {
			n = min(left, 8)
			read_little(
				bits,
				start + n,
				left - n,
				acc + read_unsigned(bits, start, n) * scale,
				core.shift_left(scale, 8),
			)
		}
}

/// The `width` low bits of `value`, in two's complement.
from_int = (value : Int, width : Int, little : Bool) -> BitArray {
	u = core.bit_and(value, core.shift_left(1, width) - 1)
	core.bit_array_concat(
		little |>
			(True) { write_little(u, width, []) }
			| (False) { write_big(u, width, []) },
	)
}

// Big endian: chunks from the least significant one, each put in front.
write_big = (u : Int, left : Int, acc : List(BitArray)) -> List(BitArray) {
	{ left == 0 } |>
		(True) { acc }
		| (False) {
			n = min(left, 14)
			low = core.bit_and(u, core.shift_left(1, n) - 1)
			write_big(
				core.shift_right(u, n),
				left - n,
				[from_unsigned(low, n), ..acc],
			)
		}
}

// Little endian: groups of 8 bits from the least significant one, in
// order; the last group has the bits that are left.
write_little = (u : Int, left : Int, acc : List(BitArray)) -> List(BitArray) {
	{ left == 0 } |>
		(True) { reverse(acc, []) }
		| (False) {
			n = min(left, 8)
			low = core.bit_and(u, core.shift_left(1, n) - 1)
			write_little(
				core.shift_right(u, n),
				left - n,
				[from_unsigned(low, n), ..acc],
			)
		}
}

reverse = (list : List(BitArray), acc : List(BitArray)) -> List(BitArray) {
	list |> ([]) { acc } | ([x, ..xs]) { reverse(xs, [x, ..acc]) }
}

min = (a : Int, b : Int) -> Int { { a < b } |> (True) { a } | (False) { b } }
