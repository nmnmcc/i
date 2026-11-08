//// BitArrays are a sequence of binary data of any length.

import i/int
import i/order
import i/string

/// Converts a string type into a bit array of UTF8 encoded data.
///
/// The resulting bit array contains the UTF-8 bytes of the string.
///
pub fn from_string(x: String) -> BitArray {
  <<x:utf8>>
}

/// Returns an integer which is the number of bits in the bit array.
///
@external(wasm, "ilang:runtime/v1", "bit_array_bit_size")
pub fn bit_size(x: BitArray) -> Int

/// Returns an integer which is the number of bytes in the bit array.
///
pub fn byte_size(x: BitArray) -> Int {
  { bit_size(x) + 7 } / 8
}

/// Pads a bit array with zeros so that it is a whole number of bytes.
///
pub fn pad_to_bytes(data: BitArray) -> BitArray {
  case bit_size(data) % 8 {
    0 -> data
    trailing_bit_count -> {
      let padding_bits = 8 - trailing_bit_count
      <<data:bits, 0:size(padding_bits)>>
    }
  }
}

/// Creates a new bit array by joining two bit arrays.
///
/// ## Examples
///
/// ```i
/// assert bit_array.append(
///     to: bit_array.from_string("butter"),
///     suffix: bit_array.from_string("fly"),
///   )
///   == bit_array.from_string("butterfly")
/// ```
///
pub fn append(to first: BitArray, suffix second: BitArray) -> BitArray {
  concat([first, second])
}

/// Extracts a sub-section of a bit array.
///
/// The slice will start at given position and continue up to specified
/// length.
/// A negative length can be used to extract bytes at the end of a bit array.
///
/// This function runs in constant time.
///
pub fn slice(
  from string: BitArray,
  at position: Int,
  take length: Int,
) -> Result(BitArray, Nil) {
  let size = bit_size(string)
  let #(start, count) = case length < 0 {
    True -> #(position + length, -length)
    False -> #(position, length)
  }
  case
    size % 8 == 0 && position >= 0 && start >= 0 && start + count <= size / 8
  {
    False -> Error(Nil)
    True ->
      case string {
        <<_:bytes-size(start), part:bytes-size(count), _:bytes>> -> Ok(part)
        _ -> Error(Nil)
      }
  }
}

/// Tests to see whether a bit array is valid UTF-8.
///
pub fn is_utf8(bits: BitArray) -> Bool {
  is_utf8_loop(bits)
}

fn is_utf8_loop(bits: BitArray) -> Bool {
  case bits {
    <<>> -> True
    <<_:utf8, rest:bytes>> -> is_utf8_loop(rest)
    _ -> False
  }
}

/// Converts a bit array to a string.
///
/// Returns an error if the bit array is invalid UTF-8 data.
///
pub fn to_string(bits: BitArray) -> Result(String, Nil) {
  case is_utf8(bits) {
    True -> Ok(unsafe_to_string(bits))
    False -> Error(Nil)
  }
}

@external(wasm, "ilang:runtime/v1", "bit_array_unsafe_to_string")
fn unsafe_to_string(a: BitArray) -> String

/// Creates a new bit array by joining multiple binaries.
///
/// ## Examples
///
/// ```i
/// assert bit_array.concat([
///     bit_array.from_string("butter"),
///     bit_array.from_string("fly"),
///   ])
///   == bit_array.from_string("butterfly")
/// ```
///
pub fn concat(bit_arrays: List(BitArray)) -> BitArray {
  let #(joined, _) = concat_n(count_parts(bit_arrays, 0), bit_arrays)
  joined
}

fn count_parts(parts: List(a), count: Int) -> Int {
  case parts {
    [] -> count
    [_, ..rest] -> count_parts(rest, count + 1)
  }
}

// A balanced join avoids repeatedly copying a growing prefix.
fn concat_n(count: Int, parts: List(BitArray)) -> #(BitArray, List(BitArray)) {
  case count, parts {
    0, _ -> #(<<>>, parts)
    _, [] -> #(<<>>, [])
    1, [first, ..rest] -> #(first, rest)
    n, _ -> {
      let left_count = n / 2
      let #(left, rest) = concat_n(left_count, parts)
      let #(right, rest) = concat_n(n - left_count, rest)
      #(<<left:bits, right:bits>>, rest)
    }
  }
}

fn reverse_parts(parts: List(a), accumulator: List(a)) -> List(a) {
  case parts {
    [] -> accumulator
    [first, ..rest] -> reverse_parts(rest, [first, ..accumulator])
  }
}

/// Encodes a BitArray into a base 64 encoded string.
///
/// If the bit array does not contain a whole number of bytes then it is padded
/// with zero bits prior to being encoded.
///
pub fn base64_encode(input: BitArray, padding: Bool) -> String {
  input
  |> pad_to_bytes
  |> encode64(padding, [])
  |> reverse_parts([])
  |> concat
  |> unsafe_to_string
}

fn digit64(digit: Int) -> Int {
  case digit {
    n if n < 26 -> 65 + n
    n if n < 52 -> 97 + n - 26
    n if n < 62 -> 48 + n - 52
    62 -> 43
    _ -> 47
  }
}

fn encode64(
  input: BitArray,
  padding: Bool,
  chunks: List(BitArray),
) -> List(BitArray) {
  case input {
    <<a, b, c, rest:bytes>> -> {
      let a0 = digit64(a / 4)
      let a1 = digit64(a % 4 * 16 + b / 16)
      let a2 = digit64(b % 16 * 4 + c / 64)
      let a3 = digit64(c % 64)
      encode64(rest, padding, [<<a0, a1, a2, a3>>, ..chunks])
    }
    <<a, b>> -> {
      let a0 = digit64(a / 4)
      let a1 = digit64(a % 4 * 16 + b / 16)
      let a2 = digit64(b % 16 * 4)
      let last = case padding {
        True -> <<a0, a1, a2, 61>>
        False -> <<a0, a1, a2>>
      }
      [last, ..chunks]
    }
    <<a>> -> {
      let a0 = digit64(a / 4)
      let a1 = digit64(a % 4 * 16)
      let last = case padding {
        True -> <<a0, a1, 61, 61>>
        False -> <<a0, a1>>
      }
      [last, ..chunks]
    }
    _ -> chunks
  }
}

/// Decodes a base 64 encoded string into a `BitArray`.
///
pub fn base64_decode(encoded: String) -> Result(BitArray, Nil) {
  let bytes =
    strip_base64_whitespace(<<encoded:utf8>>, [])
    |> reverse_parts([])
    |> concat
  let padded = case byte_size(bytes) % 4 {
    0 -> Ok(bytes)
    2 -> Ok(<<bytes:bits, 61, 61>>)
    3 -> Ok(<<bytes:bits, 61>>)
    _ -> Error(Nil)
  }
  case padded {
    Error(_) -> Error(Nil)
    Ok(bytes) -> decode64(bytes, [])
  }
}

fn strip_base64_whitespace(
  bytes: BitArray,
  chunks: List(BitArray),
) -> List(BitArray) {
  case bytes {
    <<9, rest:bytes>>
    | <<10, rest:bytes>>
    | <<13, rest:bytes>>
    | <<32, rest:bytes>> -> strip_base64_whitespace(rest, chunks)
    <<byte, rest:bytes>> -> strip_base64_whitespace(rest, [<<byte>>, ..chunks])
    _ -> chunks
  }
}

fn value64(byte: Int) -> Int {
  case byte {
    n if n >= 65 && n <= 90 -> n - 65
    n if n >= 97 && n <= 122 -> n - 97 + 26
    n if n >= 48 && n <= 57 -> n - 48 + 52
    43 -> 62
    47 -> 63
    _ -> -1
  }
}

fn decode64(bytes: BitArray, chunks: List(BitArray)) -> Result(BitArray, Nil) {
  case bytes {
    <<>> -> Ok(concat(reverse_parts(chunks, [])))
    <<a, b, 61, 61>> -> {
      let a = value64(a)
      let b = value64(b)
      case a >= 0 && b >= 0 {
        True -> {
          let byte = a * 4 + b / 16
          Ok(concat(reverse_parts([<<byte>>, ..chunks], [])))
        }
        False -> Error(Nil)
      }
    }
    <<a, b, c, 61>> -> {
      let a = value64(a)
      let b = value64(b)
      let c = value64(c)
      case a >= 0 && b >= 0 && c >= 0 {
        True -> {
          let first = a * 4 + b / 16
          let second = b % 16 * 16 + c / 4
          Ok(concat(reverse_parts([<<first, second>>, ..chunks], [])))
        }
        False -> Error(Nil)
      }
    }
    <<a, b, c, d, rest:bytes>> -> {
      let a = value64(a)
      let b = value64(b)
      let c = value64(c)
      let d = value64(d)
      case a >= 0 && b >= 0 && c >= 0 && d >= 0 {
        True -> {
          let first = a * 4 + b / 16
          let second = b % 16 * 16 + c / 4
          let third = c % 4 * 64 + d
          decode64(rest, [<<first, second, third>>, ..chunks])
        }
        False -> Error(Nil)
      }
    }
    _ -> Error(Nil)
  }
}

/// Encodes a `BitArray` into a base 64 encoded string with URL and filename
/// safe alphabet.
///
/// If the bit array does not contain a whole number of bytes then it is padded
/// with zero bits prior to being encoded.
///
pub fn base64_url_encode(input: BitArray, padding: Bool) -> String {
  input
  |> base64_encode(padding)
  |> string.replace("+", "-")
  |> string.replace("/", "_")
}

/// Decodes a base 64 encoded string with URL and filename safe alphabet into a
/// `BitArray`.
///
pub fn base64_url_decode(encoded: String) -> Result(BitArray, Nil) {
  encoded
  |> string.replace("-", "+")
  |> string.replace("_", "/")
  |> base64_decode()
}

/// Encodes a `BitArray` into a base 16 encoded string.
///
/// If the bit array does not contain a whole number of bytes then it is padded
/// with zero bits prior to being encoded.
///
pub fn base16_encode(input: BitArray) -> String {
  input
  |> pad_to_bytes
  |> encode16([])
  |> reverse_parts([])
  |> concat
  |> unsafe_to_string
}

fn encode16(bytes: BitArray, chunks: List(BitArray)) -> List(BitArray) {
  case bytes {
    <<byte, rest:bytes>> -> {
      let high = digit16(byte / 16)
      let low = digit16(byte % 16)
      encode16(rest, [<<high, low>>, ..chunks])
    }
    _ -> chunks
  }
}

fn digit16(digit: Int) -> Int {
  case digit < 10 {
    True -> digit + 48
    False -> digit + 55
  }
}

/// Decodes a base 16 encoded string into a `BitArray`.
///
pub fn base16_decode(input: String) -> Result(BitArray, Nil) {
  decode16(<<input:utf8>>, [])
}

fn value16(byte: Int) -> Int {
  case byte {
    n if n >= 48 && n <= 57 -> n - 48
    n if n >= 65 && n <= 70 -> n - 65 + 10
    n if n >= 97 && n <= 102 -> n - 97 + 10
    _ -> -1
  }
}

fn decode16(bytes: BitArray, chunks: List(BitArray)) -> Result(BitArray, Nil) {
  case bytes {
    <<>> -> Ok(concat(reverse_parts(chunks, [])))
    <<high, low, rest:bytes>> -> {
      let high = value16(high)
      let low = value16(low)
      case high >= 0 && low >= 0 {
        True -> {
          let byte = high * 16 + low
          decode16(rest, [<<byte>>, ..chunks])
        }
        False -> Error(Nil)
      }
    }
    _ -> Error(Nil)
  }
}

/// Converts a bit array to a string containing the decimal value of each byte.
///
/// Use this over `string.inspect` when you have a bit array you want printed
/// in the array syntax even if it is valid UTF-8.
///
/// ## Examples
///
/// ```i
/// assert bit_array.inspect(<<0, 20, 0x20, 255>>) == "<<0, 20, 32, 255>>"
/// ```
///
/// ```i
/// assert bit_array.inspect(<<100, 5:3>>) == "<<100, 5:size(3)>>"
/// ```
///
pub fn inspect(input: BitArray) -> String {
  inspect_loop(input, "<<") <> ">>"
}

fn inspect_loop(input: BitArray, accumulator: String) -> String {
  case input {
    <<>> -> accumulator

    <<x:size(1)>> -> accumulator <> int.to_string(x) <> ":size(1)"
    <<x:size(2)>> -> accumulator <> int.to_string(x) <> ":size(2)"
    <<x:size(3)>> -> accumulator <> int.to_string(x) <> ":size(3)"
    <<x:size(4)>> -> accumulator <> int.to_string(x) <> ":size(4)"
    <<x:size(5)>> -> accumulator <> int.to_string(x) <> ":size(5)"
    <<x:size(6)>> -> accumulator <> int.to_string(x) <> ":size(6)"
    <<x:size(7)>> -> accumulator <> int.to_string(x) <> ":size(7)"

    <<x, rest:bits>> -> {
      let suffix = case rest {
        <<>> -> ""
        _ -> ", "
      }

      let accumulator = accumulator <> int.to_string(x) <> suffix
      inspect_loop(rest, accumulator)
    }

    _ -> accumulator
  }
}

/// Compare two bit arrays as sequences of bytes.
///
/// ## Examples
///
/// ```i
/// assert bit_array.compare(<<1>>, <<2>>) == Lt
/// ```
///
/// ```i
/// assert bit_array.compare(<<"AB":utf8>>, <<"AA":utf8>>) == Gt
/// ```
///
/// ```i
/// assert bit_array.compare(<<1, 2:size(2)>>, with: <<1, 2:size(2)>>) == Eq
/// ```
///
pub fn compare(a: BitArray, with b: BitArray) -> order.Order {
  case a, b {
    <<first_byte, first_rest:bits>>, <<second_byte, second_rest:bits>> ->
      case first_byte, second_byte {
        f, s if f > s -> order.Gt
        f, s if f < s -> order.Lt
        _, _ -> compare(first_rest, second_rest)
      }

    <<>>, <<>> -> order.Eq
    // First has more items, example: "AB" > "A":
    _, <<>> -> order.Gt
    // Second has more items, example: "A" < "AB":
    <<>>, _ -> order.Lt
    // Compare the values and lengths of the remaining partial bytes.
    first, second ->
      case bit_array_to_int_and_size(first), bit_array_to_int_and_size(second) {
        #(a, _), #(b, _) if a > b -> order.Gt
        #(a, _), #(b, _) if a < b -> order.Lt
        #(_, size_a), #(_, size_b) if size_a > size_b -> order.Gt
        #(_, size_a), #(_, size_b) if size_a < size_b -> order.Lt
        _, _ -> order.Eq
      }
  }
}

fn bit_array_to_int_and_size(a: BitArray) -> #(Int, Int) {
  let size = bit_size(a)
  case a {
    <<value:size(size)>> -> #(value, size)
    _ -> #(0, size)
  }
}

/// Checks whether the first `BitArray` starts with the second one.
///
/// ## Examples
///
/// ```i
/// assert bit_array.starts_with(<<1, 2, 3, 4>>, <<1, 2>>)
/// ```
///
pub fn starts_with(bits: BitArray, prefix: BitArray) -> Bool {
  let prefix_size = bit_size(prefix)

  case bits {
    <<pref:bits-size(prefix_size), _:bits>> if pref == prefix -> True
    _ -> False
  }
}
