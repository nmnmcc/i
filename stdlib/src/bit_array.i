//// Functions for `BitArray`, the type of lists of bits.
////
//// A bit array may have any number of bits, not only whole bytes. Functions
//// that count bytes say so. The `base16` and `base64` functions work on
//// whole bytes: they first fill the last byte with zero bits.
////
//// Decoding text raises `BitArrayError`, whose `offset` is a UTF-8 byte
//// offset into the text. `to_string` and `slice` keep `()`, because the
//// decoder or the bounds name the failure and there is no useful position
//// to report.
(
	i/fail
	i/core
	order = (Order, Less, Equal, Greater) = i/order
)

(
	from_string
	to_string
	BitArrayError(..)
	is_utf8
	bit_size
	byte_size
	append
	concat
	slice
	starts_with
	pad_to_bytes
	compare
	inspect
	base16_encode
	base16_decode
	base64_encode
	base64_decode
	base64_url_encode
	base64_url_decode
)

/// The UTF-8 bytes of a string.
from_string = (string : String) -> BitArray { <<string:utf8>> }

/// A failure to read bytes from text. `offset` is a UTF-8 byte offset into
/// the text: the first byte that cannot be used, the start of a group that
/// cannot be completed, or the end of base 16 text with a missing digit.
/// `reason` is one of:
///
/// - `"invalid digit"`: a base 16 character that is not a hex digit.
/// - `"missing digit"`: base 16 text with an odd number of digits.
/// - `"invalid character"`: a base 64 character outside its alphabet.
/// - `"incomplete group"`: base 64 text that ends part-way through a group.
/// - `"bad padding"`: base 64 `=` in the wrong place or a wrong length.
BitArrayError = (offset = Int, reason = String)

_to_string_result = (bits : BitArray) -> Result(String, ()) {
	core.bit_array_to_string(bits)
}

/// `True` when the bits are whole bytes of correct UTF-8.
is_utf8 = (bits : BitArray) -> Bool {
	core.bit_array_to_string(bits) |>
		(Ok(_)) { True() }
		& (Error(_)) { False() }
}

/// The number of bits.
bit_size = (bits : BitArray) -> Int { core.bit_size(bits) }

/// The number of bytes, counting a last byte that is not whole as one.
///
/// ```i
/// bit_array.byte_size(<<1, 2:u4>>)  // 2
/// ```
byte_size = (bits : BitArray) -> Int { { core.bit_size(bits) + 7 } / 8 }

/// The bits of `to`, then the bits of `suffix`.
append = (to : BitArray, suffix : BitArray) -> BitArray {
	<<to:bits, suffix:bits>>
}

/// The bit arrays one after the other, made with one copy. Their sizes may
/// be any number of bits.
///
/// ```i
/// bit_array.concat([<<1>>, <<2, 3>>])  // <<1, 2, 3>>
/// ```
concat = (bit_arrays : List(BitArray)) -> BitArray {
	core.bit_array_concat(bit_arrays)
}

reverse = <A>(list : List(A)) -> List(A) {
	go = (rest, acc) {
		rest |> ([]) { acc } & ([x, ..xs]) { go(xs, [x, ..acc]) }
	}
	go(list, [])
}

_slice_result = (from : BitArray, at : Int, take : Int) -> Result(
	BitArray,
	(),
) {
	(start, length) = { take < 0 } |>
		(True()) { (at + take, -take) }
		& (False()) { (at, take) }
	{ start < 0 || { start + length } * 8 > core.bit_size(from) } |>
		(True()) { Error(()) }
		& (False()) {
			from |>
				(<<_:bytes(start), part:bytes(length), _:bits>>) { Ok(part) }
				& (_) { Error(()) }
		}
}

/// `True` when the bit array starts with the bits of `prefix`.
starts_with = (bits : BitArray, prefix : BitArray) -> Bool {
	n = core.bit_size(prefix)
	bits |> (<<start:bits(n), _:bits>>) { start == prefix } & (_) { False() }
}

/// The bit array with zero bits after it, up to a whole number of bytes.
pad_to_bytes = (bits : BitArray) -> BitArray {
	core.bit_size(bits) % 8 |> (0) { bits } & (r) { <<bits:bits, 0:u(8 - r)>> }
}

/// Compares two bit arrays byte by byte, from the start. When one is the
/// start of the other, the shorter one is `Less`. When the whole bytes are
/// the same and a part of a byte is left in both, these parts are compared
/// as unsigned numbers, and then by their number of bits.
///
/// ```i
/// bit_array.compare(<<1, 2>>, <<1, 3>>)  // Less
/// bit_array.compare(<<4:u8>>, <<4:u5>>)  // Greater
/// ```
compare = (a : BitArray, with : BitArray) -> Order {
	(a, with) |>
		((<<x, xs:bits>>, <<y, ys:bits>>)) {
			{ x == y } |>
				(True()) { compare(xs, ys) }
				& (False()) { int_order(x, y) }
		}
		& ((<<>>, <<>>)) { Equal() }
		& ((_, <<>>)) { Greater() }
		& ((<<>>, _)) { Less() }
		& (_) {
			m = core.bit_size(a)
			n = core.bit_size(with)
			(a, with) |>
				((<<x:u(m)>>, <<y:u(n)>>) ? x != y) { int_order(x, y) }
				& (_) { int_order(m, n) }
		}
}

int_order = (a : Int, b : Int) -> Order {
	{ a == b } |>
		(True()) { Equal() }
		& (False()) {
			{ a < b } |> (True()) { Less() } & (False()) { Greater() }
		}
}

/// The text form of the bit array.
///
/// ```i
/// bit_array.inspect(<<1, 2:u2>>)  // "<<1, 2:u2>>"
/// ```
inspect = (bits : BitArray) -> String { core.inspect(bits) }

hex_digits = "0123456789ABCDEF"

char_at = (alphabet : String, i : Int) -> String {
	core.byte_slice(alphabet, i, 1) |> (Ok(s)) { s } & (Error(_)) { "" }
}

/// The bytes in base 16, with two upper-case hex digits for each byte.
///
/// ```i
/// bit_array.base16_encode(<<255, 1>>)  // "FF01"
/// ```
base16_encode = (bits : BitArray) -> String {
	go = (rest, acc) {
		rest |>
			(<<high:u4, low:u4, more:bits>>) {
				go(
					more,
					[
						char_at(hex_digits, low),
						char_at(hex_digits, high),
						..acc,
					],
				)
			}
			& (_) { core.concat(reverse(acc)) }
	}
	go(pad_to_bytes(bits), [])
}

hex_value = (byte : Int) -> Int {
	{ byte >= 48 && byte <= 57 } |>
		(True()) { byte - 48 }
		& (False()) {
			{ byte >= 65 && byte <= 70 } |>
				(True()) { byte - 55 }
				& (False()) {
					{ byte >= 97 && byte <= 102 } |>
						(True()) { byte - 87 }
						& (False()) { -1 }
				}
		}
}

_base16_decode_result = (input : String) -> Result(BitArray, BitArrayError) {
	go = (rest, acc, at) {
		rest |>
			(<<>>) { Ok(concat(reverse(acc))) }
			& (<<a, b, more:bytes>>) {
				x = hex_value(a)
				y = hex_value(b)
				{ x >= 0 && y >= 0 } |>
					(True()) { go(more, [<<{ x * 16 + y }>>, ..acc], at + 2) }
					& (False()) {
						Error(
							BitArrayError(
								offset = { x < 0 } |>
									(True()) { at }
									& (False()) { at + 1 },
								reason = "invalid digit",
							),
						)
					}
			}
			& (<<a>>) {
				{ hex_value(a) >= 0 } |>
					(True()) {
						Error(
							BitArrayError(
								offset = at + 1,
								reason = "missing digit",
							),
						)
					}
					& (False()) {
						Error(
							BitArrayError(
								offset = at,
								reason = "invalid digit",
							),
						)
					}
			}
			& (_) {
				Error(BitArrayError(offset = at, reason = "invalid digit"))
			}
	}
	go(<<input:utf8>>, [], 0)
}

standard = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"

url_safe = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"

encode64 = (bits : BitArray, alphabet : String, padding : Bool) -> String {
	pad = padding |> (True()) { "=" } & (False()) { "" }
	c = (i) { char_at(alphabet, i) }
	go = (rest, acc) {
		rest |>
			(<<a:u6, b:u6, x:u6, d:u6, more:bits>>) {
				go(more, [c(d), c(x), c(b), c(a), ..acc])
			}
			& (<<a:u6, b:u6, x:u4>>) {
				core.concat(
					reverse([pad, c(x * 4), c(b), c(a), ..acc]),
				)
			}
			& (<<a:u6, b:u2>>) {
				core.concat(
					reverse([pad, pad, c(b * 16), c(a), ..acc]),
				)
			}
			& (_) { core.concat(reverse(acc)) }
	}
	go(pad_to_bytes(bits), [])
}

/// The bytes in base 64 (RFC 4648), with `+` and `/`. With `padding`, `=`
/// is put at the end to make the length a multiple of 4.
///
/// ```i
/// bit_array.base64_encode(<<"hi":utf8>>, True())  // "aGk="
/// ```
base64_encode = (input : BitArray, padding : Bool) -> String {
	encode64(input, standard, padding)
}

/// Like `base64_encode`, with `-` and `_` in the place of `+` and `/`, so
/// that the text may be used in a URL.
base64_url_encode = (input : BitArray, padding : Bool) -> String {
	encode64(input, url_safe, padding)
}

// The value of a base 64 character, or -1. `62` and `63` are the characters
// for 62 and 63 in this alphabet.
value64 = (byte : Int, c62 : Int, c63 : Int) -> Int {
	{ byte >= 65 && byte <= 90 } |>
		(True()) { byte - 65 }
		& (False()) {
			{ byte >= 97 && byte <= 122 } |>
				(True()) { byte - 71 }
				& (False()) {
					{ byte >= 48 && byte <= 57 } |>
						(True()) { byte + 4 }
						& (False()) {
							{ byte == c62 } |>
								(True()) { 62 }
								& (False()) {
									{ byte == c63 } |>
										(True()) { 63 }
										& (False()) { -1 }
								}
						}
				}
		}
}

// The first value that is not a base 64 digit, or `at + 3`.
first_invalid = (a : Int, b : Int, c : Int, d : Int, at : Int) -> Int {
	{ a < 0 } |>
		(True()) { at }
		& (False()) {
			{ b < 0 } |>
				(True()) { at + 1 }
				& (False()) {
					{ c < 0 } |> (True()) { at + 2 } & (False()) { at + 3 }
				}
		}
}

decode64 = (input : String, c62 : Int, c63 : Int) -> Result(
	BitArray,
	BitArrayError,
) {
	v = (byte) { value64(byte, c62, c63) }
	ok = (a, b) { a >= 0 && b >= 0 }
	go = (rest, acc, at) {
		rest |>
			(<<>>) { Ok(concat(reverse(acc))) }
			& (<<a, b, c, d, more:bytes>> ? c != 61 && d != 61) {
				(w, x, y, z) = (v(a), v(b), v(c), v(d))
				{ ok(w, x) && ok(y, z) } |>
					(True()) {
						go(more, [<<w:u6, x:u6, y:u6, z:u6>>, ..acc], at + 4)
					}
					& (False()) {
						Error(
							BitArrayError(
								offset = first_invalid(w, x, y, z, at),
								reason = "invalid character",
							),
						)
					}
			}
			// The end, with or without `=`: 3 or 2 characters for 2 or 1 bytes.
			& (<<a, b, c>> | <<a, b, c, 61>> ? c != 61) {
				(w, x, y) = (v(a), v(b), v(c))
				{ ok(w, x) && y >= 0 } |>
					(True()) {
						go(<<>>, [<<w:u6, x:u6, { y / 4 }:u4>>, ..acc], at + 3)
					}
					& (False()) {
						Error(
							BitArrayError(
								offset = first_invalid(w, x, y, 0, at),
								reason = "invalid character",
							),
						)
					}
			}
			& (<<a, b>> | <<a, b, 61, 61>>) {
				(w, x) = (v(a), v(b))
				ok(w, x) |>
					(True()) {
						go(<<>>, [<<w:u6, { x / 16 }:u2>>, ..acc], at + 2)
					}
					& (False()) {
						Error(
							BitArrayError(
								offset = first_invalid(w, x, 0, 0, at),
								reason = "invalid character",
							),
						)
					}
			}
			& (<<61, _:bytes>>) {
				Error(BitArrayError(offset = at, reason = "bad padding"))
			}
			& (<<_, 61, _:bytes>>) {
				Error(BitArrayError(offset = at + 1, reason = "bad padding"))
			}
			& (<<_, _, 61, _:bytes>>) {
				Error(BitArrayError(offset = at + 2, reason = "bad padding"))
			}
			& (<<_, _, _, 61, _:bytes>>) {
				Error(BitArrayError(offset = at + 3, reason = "bad padding"))
			}
			& (<<a>>) {
				{ v(a) < 0 } |>
					(True()) {
						Error(
							BitArrayError(
								offset = at,
								reason = "invalid character",
							),
						)
					}
					& (False()) {
						Error(
							BitArrayError(
								offset = at,
								reason = "incomplete group",
							),
						)
					}
			}
			& (_) { Error(BitArrayError(offset = at, reason = "bad padding")) }
	}
	go(<<input:utf8>>, [], 0)
}

_base64_decode_result = (encoded : String) -> Result(BitArray, BitArrayError) {
	decode64(encoded, 43, 47)
}

_base64_url_decode_result = (encoded : String) -> Result(
	BitArray,
	BitArrayError,
) { decode64(encoded, 45, 95) }

/// The string, when the bits are whole bytes of correct UTF-8; otherwise
/// raises `()`, because the UTF-8 decoder reports only success or failure
/// and there is no position to report.
///
/// ```i
/// fail.catch(() { bit_array.to_string(<<104, 105>>) })  // Ok("hi")
/// fail.catch(() { bit_array.to_string(<<255>>) })       // Error(())
/// ```
///
/// Reports failure through `Fail(())`.
to_string = (bits : BitArray) -> String {
	fail.from_result(_to_string_result(bits))
}

/// `take` bytes from byte `position`, counting from 0. A negative `take`
/// takes the bytes before `position`. Raises `()` when they are not all in
/// the bit array, because the bounds are the whole failure and there is no
/// text position to report.
///
/// ```i
/// fail.catch(() { bit_array.slice(<<1, 2, 3, 4>>, 1, 2) })   // Ok(<<2, 3>>)
/// fail.catch(() { bit_array.slice(<<1, 2, 3, 4>>, 3, -2) })  // Ok(<<2, 3>>)
/// ```
///
/// Reports failure through `Fail(())`.
slice = (from : BitArray, at : Int, take : Int) -> BitArray {
	fail.from_result(_slice_result(from, at, take))
}

/// Reads base 16 text: two hex digits, upper or lower case, for each byte.
/// Any other text raises `BitArrayError`, which gives the offset and the
/// reason.
///
/// ```i
/// fail.catch(() { bit_array.base16_decode("ff01") })  // Ok(<<255, 1>>)
/// fail.catch(() { bit_array.base16_decode("zz") })    // Error(BitArrayError(offset = 0, reason = "invalid digit"))
/// ```
///
/// Reports failure through `Fail(BitArrayError)`.
base16_decode = (input : String) -> BitArray {
	fail.from_result(_base16_decode_result(input))
}

/// Reads base 64 text (RFC 4648), with `+` and `/`. The `=` padding at the
/// end may be there or not. Any other text raises `BitArrayError`, which
/// gives the offset and the reason.
///
/// ```i
/// fail.catch(() { bit_array.base64_decode("aGk=") })  // Ok(<<104, 105>>)
/// fail.catch(() { bit_array.base64_decode("aGk") })   // Ok(<<104, 105>>)
/// fail.catch(() { bit_array.base64_decode("!") })     // Error(BitArrayError(offset = 0, reason = "invalid character"))
/// ```
///
/// Reports failure through `Fail(BitArrayError)`.
base64_decode = (encoded : String) -> BitArray {
	fail.from_result(_base64_decode_result(encoded))
}

/// Like `base64_decode`, with `-` and `_` in the place of `+` and `/`.
///
/// Reports failure through `Fail(BitArrayError)`.
base64_url_decode = (encoded : String) -> BitArray {
	fail.from_result(_base64_url_decode_result(encoded))
}
