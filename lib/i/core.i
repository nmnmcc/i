//// The operations that are not written in i (§13).

(
	not
	to_float
	truncate
	floor
	ceiling
	sqrt
	float_to_bits
	float_from_bits
	bit_and
	bit_or
	bit_xor
	bit_not
	shift_left
	shift_right
	concat
	byte_size
	byte_slice
	codepoints
	from_codepoints
	codepoint_to_int
	int_to_codepoint
	to_upper
	to_lower
	index_of
	bit_size
	bit_array_to_string
	bit_array_concat
	inspect
	hash
)

/// `True` for `False`, and `False` for `True`.
@external("ilang:core", "not")
not: (Bool) -> Bool

/// The nearest Float, with ties to even.
@external("ilang:core", "to_float")
to_float: (Int) -> Float

/// The Float without its fraction, rounded toward zero.
@external("ilang:core", "truncate")
truncate: (Float) -> Int

/// The greatest whole number that is not greater.
@external("ilang:core", "floor")
floor: (Float) -> Float

/// The smallest whole number that is not smaller.
@external("ilang:core", "ceiling")
ceiling: (Float) -> Float

/// The square root.
@external("ilang:core", "sqrt")
sqrt: (Float) -> Float

/// The 64 bits of the IEEE 754 binary64 value.
@external("ilang:core", "float_to_bits")
float_to_bits: (Float) -> Int

/// The float with the lowest 64 bits of the number.
@external("ilang:core", "float_from_bits")
float_from_bits: (Int) -> Float

/// Bitwise and, on two's complement with no end.
@external("ilang:core", "bit_and")
bit_and: (Int, Int) -> Int

/// Bitwise or, on two's complement with no end.
@external("ilang:core", "bit_or")
bit_or: (Int, Int) -> Int

/// Bitwise exclusive or, on two's complement with no end.
@external("ilang:core", "bit_xor")
bit_xor: (Int, Int) -> Int

/// `-x - 1`.
@external("ilang:core", "bit_not")
bit_not: (Int) -> Int

/// `x` times 2^n.
@external("ilang:core", "shift_left")
shift_left: (Int, Int) -> Int

/// `x` divided by 2^n, rounded down.
@external("ilang:core", "shift_right")
shift_right: (Int, Int) -> Int

/// The strings one after the other.
@external("ilang:core", "concat")
concat: (List(String)) -> String

/// The number of UTF-8 bytes.
@external("ilang:core", "byte_size")
byte_size: (String) -> Int

/// The bytes from `start`, `length` of them.
@external("ilang:core", "byte_slice")
byte_slice: (String, start: Int, length: Int) -> Result(String, Nil)

/// The characters in order.
@external("ilang:core", "codepoints")
codepoints: (String) -> List(UtfCodepoint)

/// The string of these characters.
@external("ilang:core", "from_codepoints")
from_codepoints: (List(UtfCodepoint)) -> String

/// The number of the character.
@external("ilang:core", "codepoint_to_int")
codepoint_to_int: (UtfCodepoint) -> Int

/// The character with this number.
@external("ilang:core", "int_to_codepoint")
int_to_codepoint: (Int) -> Result(UtfCodepoint, Nil)

/// Each character by its simple uppercase mapping in Unicode.
@external("ilang:core", "to_upper")
to_upper: (String) -> String

/// Each character by its simple lowercase mapping in Unicode.
@external("ilang:core", "to_lower")
to_lower: (String) -> String

/// The byte offset of the first `find` at or after byte `from`.
@external("ilang:core", "index_of")
index_of: (in: String, find: String, from: Int) -> Result(Int, Nil)

/// The number of bits.
@external("ilang:core", "bit_size")
bit_size: (BitArray) -> Int

/// The string, when the bits are whole bytes of correct UTF-8.
@external("ilang:core", "bit_array_to_string")
bit_array_to_string: (BitArray) -> Result(String, Nil)

/// The bit arrays one after the other.
@external("ilang:core", "bit_array_concat")
bit_array_concat: (List(BitArray)) -> BitArray

/// The text form of the value (§11.8).
@external("ilang:core", "inspect")
inspect(A): (A) -> String

/// A number from 0 to 2^32 - 1; values that are the same give the same number.
@external("ilang:core", "hash")
hash(A): (A) -> Int
