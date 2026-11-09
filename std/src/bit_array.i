//// Functions for `BitArray`, the type of lists of bits.
////
//// A bit array may have any number of bits, not only whole bytes. Functions
//// that count bytes say so. The `base16` and `base64` functions work on
//// whole bytes: they first fill the last byte with zero bits.

import (
  i/core
  order = (Order, Less, Equal, Greater) = std/order
)

export (
  from_string
  to_string
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
from_string = (string: String) -> BitArray { <<string:utf8>> }

/// The string, when the bits are whole bytes of correct UTF-8, or
/// `Error(Nil)`.
///
/// ```i
/// bit_array.to_string(<<104, 105>>)  // Ok("hi")
/// bit_array.to_string(<<255>>)       // Error(Nil)
/// ```
to_string = (bits: BitArray) -> Result(String, Nil) { core.bit_array_to_string(bits) }

/// `True` when the bits are whole bytes of correct UTF-8.
is_utf8 = (bits: BitArray) -> Bool {
  case core.bit_array_to_string(bits) {
    Ok(_) -> True
    Error(_) -> False
  }
}

/// The number of bits.
bit_size = (bits: BitArray) -> Int { core.bit_size(bits) }

/// The number of bytes, counting a last byte that is not whole as one.
///
/// ```i
/// bit_array.byte_size(<<1, 2:u4>>)  // 2
/// ```
byte_size = (bits: BitArray) -> Int { { core.bit_size(bits) + 7 } / 8 }

/// The bits of `to`, then the bits of `suffix`.
append = (to: BitArray, suffix: BitArray) -> BitArray { <<to:bits, suffix:bits>> }

/// The bit arrays one after the other, made with one copy. Their sizes may
/// be any number of bits.
///
/// ```i
/// bit_array.concat([<<1>>, <<2, 3>>])  // <<1, 2, 3>>
/// ```
concat = (bit_arrays: List(BitArray)) -> BitArray { core.bit_array_concat(bit_arrays) }

reverse = (list: List(a)) -> List(a) {
  go = (rest, acc) {
    case rest {
      [] -> acc
      [x, ..xs] -> go(xs, [x, ..acc])
    }
  }
  go(list, [])
}

/// `take` bytes from byte `position`, counting from 0. A negative `take`
/// takes the bytes before `position`. `Error(Nil)` when they are not all in
/// the bit array.
///
/// ```i
/// bit_array.slice(<<1, 2, 3, 4>>, at: 1, take: 2)   // Ok(<<2, 3>>)
/// bit_array.slice(<<1, 2, 3, 4>>, at: 3, take: -2)  // Ok(<<2, 3>>)
/// ```
slice = (from: BitArray, at: Int, take: Int) -> Result(BitArray, Nil) {
  (start, length) = case take < 0 {
    True -> (at + take, -take)
    False -> (at, take)
  }
  case start < 0 || { start + length } * 8 > core.bit_size(from) {
    True -> Error(Nil)
    False -> case from {
      <<_:bytes(start), part:bytes(length), _:bits>> -> Ok(part)
      _ -> Error(Nil)
    }
  }
}

/// `True` when the bit array starts with the bits of `prefix`.
starts_with = (bits: BitArray, prefix: BitArray) -> Bool {
  n = core.bit_size(prefix)
  case bits {
    <<start:bits(n), _:bits>> -> start == prefix
    _ -> False
  }
}

/// The bit array with zero bits after it, up to a whole number of bytes.
pad_to_bytes = (bits: BitArray) -> BitArray {
  case core.bit_size(bits) % 8 {
    0 -> bits
    r -> <<bits:bits, 0:u(8 - r)>>
  }
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
compare = (a: BitArray, with: BitArray) -> Order {
  case (a, with) {
    (<<x, xs:bits>>, <<y, ys:bits>>) -> case x == y {
      True -> compare(xs, ys)
      False -> int_order(x, y)
    }
    (<<>>, <<>>) -> Equal
    (_, <<>>) -> Greater
    (<<>>, _) -> Less
    _ -> {
      m = core.bit_size(a)
      n = core.bit_size(with)
      case (a, with) {
        (<<x:u(m)>>, <<y:u(n)>>) if x != y -> int_order(x, y)
        _ -> int_order(m, n)
      }
    }
  }
}

int_order = (a: Int, b: Int) -> Order {
  case a == b {
    True -> Equal
    False -> case a < b {
      True -> Less
      False -> Greater
    }
  }
}

/// The text form of the bit array (§11.8).
///
/// ```i
/// bit_array.inspect(<<1, 2:u2>>)  // "<<1, 2:u2>>"
/// ```
inspect = (bits: BitArray) -> String { core.inspect(bits) }

hex_digits = "0123456789ABCDEF"

char_at = (alphabet: String, i: Int) -> String {
  case core.byte_slice(alphabet, i, 1) {
    Ok(s) -> s
    Error(_) -> ""
  }
}

/// The bytes in base 16, with two upper-case hex digits for each byte.
///
/// ```i
/// bit_array.base16_encode(<<255, 1>>)  // "FF01"
/// ```
base16_encode = (bits: BitArray) -> String {
  go = (rest, acc) {
    case rest {
      <<high:u4, low:u4, more:bits>> -> go(more, [char_at(hex_digits, low), char_at(hex_digits, high), ..acc])
      _ -> core.concat(reverse(acc))
    }
  }
  go(pad_to_bytes(bits), [])
}

hex_value = (byte: Int) -> Int {
  case byte >= 48 && byte <= 57 {
    True -> byte - 48
    False -> case byte >= 65 && byte <= 70 {
      True -> byte - 55
      False -> case byte >= 97 && byte <= 102 {
        True -> byte - 87
        False -> -1
      }
    }
  }
}

/// Reads base 16 text: two hex digits, upper or lower case, for each byte.
/// Any other text gives `Error(Nil)`.
///
/// ```i
/// bit_array.base16_decode("ff01")  // Ok(<<255, 1>>)
/// ```
base16_decode = (input: String) -> Result(BitArray, Nil) {
  go = (rest, acc) {
    case rest {
      <<>> -> Ok(concat(reverse(acc)))
      <<a, b, more:bytes>> -> {
        x = hex_value(a)
        y = hex_value(b)
        case x >= 0 && y >= 0 {
          True -> go(more, [<<{ x * 16 + y }>>, ..acc])
          False -> Error(Nil)
        }
      }
      _ -> Error(Nil)
    }
  }
  go(<<input:utf8>>, [])
}

standard = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"

url_safe = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"

encode64 = (bits: BitArray, alphabet: String, padding: Bool) -> String {
  pad = case padding {
    True -> "="
    False -> ""
  }
  c = (i) { char_at(alphabet, i) }
  go = (rest, acc) {
    case rest {
      <<a:u6, b:u6, x:u6, d:u6, more:bits>> -> go(more, [c(d), c(x), c(b), c(a), ..acc])
      <<a:u6, b:u6, x:u4>> -> core.concat(reverse([pad, c(x * 4), c(b), c(a), ..acc]))
      <<a:u6, b:u2>> -> core.concat(reverse([pad, pad, c(b * 16), c(a), ..acc]))
      _ -> core.concat(reverse(acc))
    }
  }
  go(pad_to_bytes(bits), [])
}

/// The bytes in base 64 (RFC 4648), with `+` and `/`. With `padding`, `=`
/// is put at the end to make the length a multiple of 4.
///
/// ```i
/// bit_array.base64_encode(<<"hi":utf8>>, True)  // "aGk="
/// ```
base64_encode = (input: BitArray, padding: Bool) -> String {
  encode64(input, standard, padding)
}

/// Like `base64_encode`, with `-` and `_` in the place of `+` and `/`, so
/// that the text may be used in a URL.
base64_url_encode = (input: BitArray, padding: Bool) -> String {
  encode64(input, url_safe, padding)
}

// The value of a base 64 character, or -1. `62` and `63` are the characters
// for 62 and 63 in this alphabet.
value64 = (byte: Int, c62: Int, c63: Int) -> Int {
  case byte >= 65 && byte <= 90 {
    True -> byte - 65
    False -> case byte >= 97 && byte <= 122 {
      True -> byte - 71
      False -> case byte >= 48 && byte <= 57 {
        True -> byte + 4
        False -> case byte == c62 {
          True -> 62
          False -> case byte == c63 {
            True -> 63
            False -> -1
          }
        }
      }
    }
  }
}

decode64 = (input: String, c62: Int, c63: Int) -> Result(BitArray, Nil) {
  v = (byte) { value64(byte, c62, c63) }
  ok = (a, b) { a >= 0 && b >= 0 }
  go = (rest, acc) {
    case rest {
      <<>> -> Ok(concat(reverse(acc)))
      <<a, b, c, d, more:bytes>> if c != 61 && d != 61 -> {
        (w, x, y, z) = (v(a), v(b), v(c), v(d))
        case ok(w, x) && ok(y, z) {
          True -> go(more, [<<w:u6, x:u6, y:u6, z:u6>>, ..acc])
          False -> Error(Nil)
        }
      }
      // The end, with or without `=`: 3 or 2 characters for 2 or 1 bytes.
      <<a, b, c>> | <<a, b, c, 61>> if c != 61 -> {
        (w, x, y) = (v(a), v(b), v(c))
        case ok(w, x) && y >= 0 {
          True -> go(<<>>, [<<w:u6, x:u6, { y / 4 }:u4>>, ..acc])
          False -> Error(Nil)
        }
      }
      <<a, b>> | <<a, b, 61, 61>> -> {
        (w, x) = (v(a), v(b))
        case ok(w, x) {
          True -> go(<<>>, [<<w:u6, { x / 16 }:u2>>, ..acc])
          False -> Error(Nil)
        }
      }
      _ -> Error(Nil)
    }
  }
  go(<<input:utf8>>, [])
}

/// Reads base 64 text (RFC 4648), with `+` and `/`. The `=` padding at the
/// end may be there or not. Any other text gives `Error(Nil)`.
///
/// ```i
/// bit_array.base64_decode("aGk=")  // Ok(<<104, 105>>)
/// bit_array.base64_decode("aGk")   // Ok(<<104, 105>>)
/// ```
base64_decode = (encoded: String) -> Result(BitArray, Nil) { decode64(encoded, 43, 47) }

/// Like `base64_decode`, with `-` and `_` in the place of `+` and `/`.
base64_url_decode = (encoded: String) -> Result(BitArray, Nil) { decode64(encoded, 45, 95) }
