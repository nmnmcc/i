//// The operations that are not written in i.
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
	portable
)

/// `True` for `False`, and `False` for `True`.
@runtime("ilang:core", "not")
not : (Bool) -> Bool

/// The nearest Float, with ties to even.
@runtime("ilang:core", "to_float")
to_float : (Int) -> Float

/// The Float without its fraction, rounded toward zero.
@runtime("ilang:core", "truncate")
truncate : (Float) -> Int

/// The greatest whole number that is not greater.
@runtime("ilang:core", "floor")
floor : (Float) -> Float

/// The smallest whole number that is not smaller.
@runtime("ilang:core", "ceiling")
ceiling : (Float) -> Float

/// The square root.
@runtime("ilang:core", "sqrt")
sqrt : (Float) -> Float

/// The 64 bits of the IEEE 754 binary64 value.
@runtime("ilang:core", "float_to_bits")
float_to_bits : (Float) -> Int

/// The float with the lowest 64 bits of the number.
@runtime("ilang:core", "float_from_bits")
float_from_bits : (Int) -> Float

/// Bitwise and, on two's complement with no end.
@runtime("ilang:core", "bit_and")
bit_and : (Int, Int) -> Int

/// Bitwise or, on two's complement with no end.
@runtime("ilang:core", "bit_or")
bit_or : (Int, Int) -> Int

/// Bitwise exclusive or, on two's complement with no end.
@runtime("ilang:core", "bit_xor")
bit_xor : (Int, Int) -> Int

/// `-x - 1`.
@runtime("ilang:core", "bit_not")
bit_not : (Int) -> Int

/// `x` times 2^n.
@runtime("ilang:core", "shift_left")
shift_left : (Int, Int) -> Int

/// `x` divided by 2^n, rounded down.
@runtime("ilang:core", "shift_right")
shift_right : (Int, Int) -> Int

/// The strings one after the other.
@runtime("ilang:core", "concat")
concat : (List(String)) -> String

/// The number of UTF-8 bytes.
@runtime("ilang:core", "byte_size")
byte_size : (String) -> Int

/// The bytes from `start`, `length` of them.
@runtime("ilang:core", "byte_slice")
byte_slice : (String, Int, Int) -> Result(String, ())

/// The characters in order.
@runtime("ilang:core", "codepoints")
codepoints : (String) -> List(UtfCodepoint)

/// The string of these characters.
@runtime("ilang:core", "from_codepoints")
from_codepoints : (List(UtfCodepoint)) -> String

/// The number of the character.
@runtime("ilang:core", "codepoint_to_int")
codepoint_to_int : (UtfCodepoint) -> Int

/// The character with this number.
@runtime("ilang:core", "int_to_codepoint")
int_to_codepoint : (Int) -> Result(UtfCodepoint, ())

/// Each character by its simple uppercase mapping in Unicode.
@runtime("ilang:core", "to_upper")
to_upper : (String) -> String

/// Each character by its simple lowercase mapping in Unicode.
@runtime("ilang:core", "to_lower")
to_lower : (String) -> String

/// The byte offset of the first `find` at or after byte `from`.
@runtime("ilang:core", "index_of")
index_of : (String, String, Int) -> Result(Int, ())

/// The number of bits.
@runtime("ilang:core", "bit_size")
bit_size : (BitArray) -> Int

/// The string, when the bits are whole bytes of correct UTF-8.
@runtime("ilang:core", "bit_array_to_string")
bit_array_to_string : (BitArray) -> Result(String, ())

/// The bit arrays one after the other.
@runtime("ilang:core", "bit_array_concat")
bit_array_concat : (List(BitArray)) -> BitArray

/// The text form of the value.
@runtime("ilang:core", "inspect")
inspect : <A>(A) -> String

/// A number from 0 to 2^32 - 1; values that are the same give the same number.
@runtime("ilang:core", "hash")
hash : <A>(A) -> Int

/// Whether this value and every capture or held value can cross workers.
@runtime("ilang:core", "portable")
portable : <A>(A) -> Bool
