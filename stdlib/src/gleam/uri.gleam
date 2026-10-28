//// Utilities for working with URIs
////
//// This module provides functions for working with URIs (for example, parsing
//// URIs or encoding query strings). The functions in this module are implemented
//// according to [RFC 3986](https://tools.ietf.org/html/rfc3986).
////
//// Query encoding (Form encoding) is defined in the
//// [W3C specification](https://www.w3.org/TR/html52/sec-forms.html#urlencoded-form-data).

import gleam/bit_array
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/string

/// Type representing the parsed components of a URI.
/// All components of a URI are optional, except the path.
///
pub type Uri {
  Uri(
    scheme: Option(String),
    userinfo: Option(String),
    host: Option(String),
    port: Option(Int),
    path: String,
    query: Option(String),
    fragment: Option(String),
  )
}

/// Constant representing an empty URI, equivalent to "".
///
/// ## Examples
///
/// ```gleam
/// assert Uri(..empty, scheme: Some("https"), host: Some("example.com"))
///   == Uri(
///     scheme: Some("https"),
///     userinfo: None,
///     host: Some("example.com"),
///     port: None,
///     path: "",
///     query: None,
///     fragment: None,
///   )
/// ```
///
pub const empty =
  Uri(
    scheme: None,
    userinfo: None,
    host: None,
    port: None,
    path: "",
    query: None,
    fragment: None,
  )

/// Parses a compliant URI string into the `Uri` type.
/// If the string is not a valid URI string then an error is returned.
///
/// The opposite operation is `uri.to_string`.
///
/// ## Examples
///
/// ```gleam
/// assert uri.parse("https://example.com:1234/a/b?query=true#fragment")
///   == Ok(Uri(
///     scheme: Some("https"),
///     userinfo: None,
///     host: Some("example.com"),
///     port: Some(1234),
///     path: "/a/b",
///     query: Some("query=true"),
///     fragment: Some("fragment"),
///   ))
/// ```
///
pub fn parse(uri_string: String) -> Result(Uri, Nil) {
  let #(without_fragment, fragment) = split_optional(uri_string, "#")
  let #(resource, query) = split_optional(without_fragment, "?")
  let pieces = Uri(..empty, query: query, fragment: fragment)
  case
    valid_optional_component(query, 3) && valid_optional_component(fragment, 3)
  {
    False -> Error(Nil)
    True -> parse_resource(resource, pieces)
  }
}

fn split_optional(
  value: String,
  separator: String,
) -> #(String, Option(String)) {
  case string.split_once(value, separator) {
    Ok(#(before, after)) -> #(before, Some(after))
    Error(_) -> #(value, None)
  }
}

fn parse_resource(resource: String, pieces: Uri) -> Result(Uri, Nil) {
  case string.split_once(resource, ":") {
    Error(_) -> parse_hierarchy(resource, pieces)
    Ok(#(scheme, rest)) -> {
      case string.contains(scheme, "/") {
        True -> parse_hierarchy(resource, pieces)
        False -> {
          case valid_scheme(bit_array.from_string(scheme)) {
            True -> {
              let pieces = Uri(..pieces, scheme: Some(string.lowercase(scheme)))
              parse_hierarchy(rest, pieces)
            }
            False -> Error(Nil)
          }
        }
      }
    }
  }
}

fn valid_scheme(bytes: BitArray) -> Bool {
  case bytes {
    <<first, rest:bytes>> -> is_alpha(first) && valid_scheme_rest(rest)
    _ -> False
  }
}

fn valid_scheme_rest(bytes: BitArray) -> Bool {
  case bytes {
    <<>> -> True
    <<byte, rest:bytes>> -> {
      let allowed =
        is_alpha(byte)
        || is_digit(byte)
        || byte == 43
        || byte == 45
        || byte == 46
      allowed && valid_scheme_rest(rest)
    }
    _ -> False
  }
}

fn parse_hierarchy(resource: String, pieces: Uri) -> Result(Uri, Nil) {
  case resource {
    "//" <> authority_and_path -> {
      let #(authority, path) = case string.split_once(authority_and_path, "/") {
        Ok(#(authority, path)) -> #(authority, "/" <> path)
        Error(_) -> #(authority_and_path, "")
      }
      case valid_component(bit_array.from_string(path), 2) {
        True -> parse_authority(authority, Uri(..pieces, path: path))
        False -> Error(Nil)
      }
    }
    _ -> {
      case valid_component(bit_array.from_string(resource), 2) {
        True -> Ok(Uri(..pieces, path: resource))
        False -> Error(Nil)
      }
    }
  }
}

fn parse_authority(authority: String, pieces: Uri) -> Result(Uri, Nil) {
  case string.split_once(authority, "@") {
    Error(_) -> parse_host(authority, pieces)
    Ok(#(userinfo, host)) -> {
      case valid_component(bit_array.from_string(userinfo), 0) {
        True -> parse_host(host, Uri(..pieces, userinfo: Some(userinfo)))
        False -> Error(Nil)
      }
    }
  }
}

fn parse_host(authority: String, pieces: Uri) -> Result(Uri, Nil) {
  case authority {
    "[" <> rest -> {
      case string.split_once(rest, "]") {
        Error(_) -> Error(Nil)
        Ok(#(host, suffix)) -> {
          case valid_ipv6(host) {
            False -> Error(Nil)
            True -> {
              let pieces = Uri(..pieces, host: Some(host))
              case suffix {
                "" -> Ok(pieces)
                ":" <> port -> parse_port(port, pieces)
                _ -> Error(Nil)
              }
            }
          }
        }
      }
    }
    _ -> {
      let #(host, port) = split_optional(authority, ":")
      case valid_component(bit_array.from_string(host), 1) {
        False -> Error(Nil)
        True -> {
          let pieces = Uri(..pieces, host: Some(host))
          case port {
            None -> Ok(pieces)
            Some(port) -> parse_port(port, pieces)
          }
        }
      }
    }
  }
}

fn parse_port(port: String, pieces: Uri) -> Result(Uri, Nil) {
  case port {
    "" -> Ok(pieces)
    _ -> {
      case decimal(bit_array.from_string(port), 0) {
        Ok(port) -> Ok(Uri(..pieces, port: Some(port)))
        Error(_) -> Error(Nil)
      }
    }
  }
}

fn decimal(bytes: BitArray, accumulator: Int) -> Result(Int, Nil) {
  case bytes {
    <<>> -> Ok(accumulator)
    <<byte, rest:bytes>> if byte >= 48 && byte <= 57 ->
      decimal(rest, accumulator * 10 + byte - 48)
    _ -> Error(Nil)
  }
}

// An IPv4 tail occupies the final two groups of an IPv6 address. Compression
// must replace at least one group and can appear at most once.
fn valid_ipv6(host: String) -> Bool {
  case string.split_once(host, "::") {
    Error(_) -> {
      case ipv6_units(string.split(host, ":"), True) {
        Ok(8) -> True
        _ -> False
      }
    }
    Ok(#(left, right)) -> {
      let left_parts = case left {
        "" -> []
        _ -> string.split(left, ":")
      }
      let right_parts = case right {
        "" -> []
        _ -> string.split(right, ":")
      }
      case ipv6_units(left_parts, False), ipv6_units(right_parts, True) {
        Ok(left_size), Ok(right_size) -> left_size + right_size < 8
        _, _ -> False
      }
    }
  }
}

fn ipv6_units(parts: List(String), allow_ipv4: Bool) -> Result(Int, Nil) {
  case parts {
    [] -> Ok(0)
    [part] -> {
      case allow_ipv4 && string.contains(part, ".") {
        True -> {
          case valid_ipv4(string.split(part, ".")) {
            True -> Ok(2)
            False -> Error(Nil)
          }
        }
        False -> {
          case valid_hex_group(bit_array.from_string(part), 0) {
            True -> Ok(1)
            False -> Error(Nil)
          }
        }
      }
    }
    [part, ..rest] -> {
      case valid_hex_group(bit_array.from_string(part), 0) {
        False -> Error(Nil)
        True -> {
          case ipv6_units(rest, allow_ipv4) {
            Ok(size) -> Ok(size + 1)
            Error(_) -> Error(Nil)
          }
        }
      }
    }
  }
}

fn valid_hex_group(bytes: BitArray, size: Int) -> Bool {
  case bytes {
    <<>> -> size > 0 && size <= 4
    <<byte, rest:bytes>> if size < 4 -> {
      case hex_value(byte) {
        Ok(_) -> valid_hex_group(rest, size + 1)
        Error(_) -> False
      }
    }
    _ -> False
  }
}

fn valid_ipv4(parts: List(String)) -> Bool {
  case parts {
    [a, b, c, d] ->
      valid_octet(a) && valid_octet(b) && valid_octet(c) && valid_octet(d)
    _ -> False
  }
}

fn valid_octet(part: String) -> Bool {
  case bit_array.from_string(part) {
    <<>> -> False
    <<48>> -> True
    <<48, _:bytes>> -> False
    bytes -> {
      case decimal(bytes, 0) {
        Ok(value) -> value <= 255
        Error(_) -> False
      }
    }
  }
}

fn valid_optional_component(value: Option(String), kind: Int) -> Bool {
  case value {
    None -> True
    Some(value) -> valid_component(bit_array.from_string(value), kind)
  }
}

// kind: 0 = userinfo, 1 = registered host, 2 = path, 3 = query/fragment.
// Checking bytes deliberately rejects unescaped non-ASCII characters, as the
// Erlang target does for RFC 3986 URI strings. Percent escapes are not decoded.
fn valid_component(bytes: BitArray, kind: Int) -> Bool {
  case bytes {
    <<>> -> True
    <<37, high, low, rest:bytes>> -> {
      case hex_value(high), hex_value(low) {
        Ok(_), Ok(_) -> valid_component(rest, kind)
        _, _ -> False
      }
    }
    <<byte, rest:bytes>> -> {
      let allowed =
        is_unreserved(byte)
        || is_subdelimiter(byte)
        || { kind == 0 && byte == 58 }
        || { kind >= 2 && { byte == 58 || byte == 64 || byte == 47 } }
        || { kind == 3 && byte == 63 }
      allowed && valid_component(rest, kind)
    }
    _ -> False
  }
}

fn is_alpha(byte: Int) -> Bool {
  { byte >= 65 && byte <= 90 } || { byte >= 97 && byte <= 122 }
}

fn is_digit(byte: Int) -> Bool {
  byte >= 48 && byte <= 57
}

fn is_unreserved(byte: Int) -> Bool {
  is_alpha(byte)
  || is_digit(byte)
  || byte == 45
  || byte == 46
  || byte == 95
  || byte == 126
}

fn is_subdelimiter(byte: Int) -> Bool {
  case byte {
    33 | 36 | 38 | 39 | 40 | 41 | 42 | 43 | 44 | 59 | 61 -> True
    _ -> False
  }
}

/// Parses an URL-encoded query string into a list of key value pairs.
/// Returns an error for invalid encoding.
///
/// The opposite operation is `uri.query_to_string`.
///
/// ## Examples
///
/// ```gleam
/// assert uri.parse_query("a=1&b=2") == Ok([#("a", "1"), #("b", "2")])
/// ```
///
pub fn parse_query(query: String) -> Result(List(#(String, String)), Nil) {
  case query {
    "" -> Ok([])
    _ -> parse_query_key(bit_array.from_string(query), [], [])
  }
}

fn parse_query_key(
  bytes: BitArray,
  key: List(BitArray),
  pairs: List(#(String, String)),
) -> Result(List(#(String, String)), Nil) {
  case bytes {
    <<61, rest:bytes>> -> parse_query_value(rest, key, [], pairs)
    // HTML numeric entities in keys are accepted by Erlang's form decoder.
    <<38, 35, rest:bytes>> -> parse_query_key(rest, [<<38, 35>>, ..key], pairs)
    <<38, rest:bytes>> -> finish_query_pair(key, [], Some(rest), pairs)
    <<byte, rest:bytes>> -> parse_query_key(rest, [<<byte>>, ..key], pairs)
    <<>> -> finish_query_pair(key, [], None, pairs)
    _ -> Error(Nil)
  }
}

fn parse_query_value(
  bytes: BitArray,
  key: List(BitArray),
  value: List(BitArray),
  pairs: List(#(String, String)),
) -> Result(List(#(String, String)), Nil) {
  case bytes {
    <<38, rest:bytes>> -> finish_query_pair(key, value, Some(rest), pairs)
    <<byte, rest:bytes>> ->
      parse_query_value(rest, key, [<<byte>>, ..value], pairs)
    <<>> -> finish_query_pair(key, value, None, pairs)
    _ -> Error(Nil)
  }
}

fn finish_query_pair(
  key: List(BitArray),
  value: List(BitArray),
  remaining: Option(BitArray),
  pairs: List(#(String, String)),
) -> Result(List(#(String, String)), Nil) {
  let key = decode_form(bit_array.concat(list.reverse(key)))
  let value = decode_form(bit_array.concat(list.reverse(value)))
  case key, value {
    Ok(key), Ok(value) -> {
      let pairs = [#(key, value), ..pairs]
      case remaining {
        Some(rest) -> parse_query_key(rest, [], pairs)
        None -> Ok(list.reverse(pairs))
      }
    }
    _, _ -> Error(Nil)
  }
}

fn decode_form(bytes: BitArray) -> Result(String, Nil) {
  case decode_percent_bytes(bytes, True, []) {
    Error(_) -> Error(Nil)
    Ok(bytes) -> decode_numeric_entities(bytes, [])
  }
}

fn decode_numeric_entities(
  bytes: BitArray,
  accumulator: List(BitArray),
) -> Result(String, Nil) {
  case bytes {
    <<>> -> bit_array.to_string(bit_array.concat(list.reverse(accumulator)))
    <<38, 35, rest:bytes>> -> decode_numeric_entity(rest, 0, accumulator)
    <<byte, rest:bytes>> ->
      decode_numeric_entities(rest, [<<byte>>, ..accumulator])
    _ -> Error(Nil)
  }
}

fn decode_numeric_entity(
  bytes: BitArray,
  codepoint: Int,
  accumulator: List(BitArray),
) -> Result(String, Nil) {
  case bytes {
    <<byte, rest:bytes>> if byte >= 48 && byte <= 57 ->
      decode_numeric_entity(rest, codepoint * 10 + byte - 48, accumulator)
    <<59, rest:bytes>> -> {
      case string.utf_codepoint(codepoint) {
        Ok(codepoint) ->
          decode_numeric_entities(rest, [
            <<codepoint:utf8_codepoint>>,
            ..accumulator
          ])
        Error(_) -> Error(Nil)
      }
    }
    _ -> Error(Nil)
  }
}

/// Encodes a list of key value pairs as a URI query string.
///
/// The opposite operation is `uri.parse_query`.
///
/// ## Examples
///
/// ```gleam
/// assert uri.query_to_string([#("a", "1"), #("b", "2")]) == "a=1&b=2"
/// ```
///
pub fn query_to_string(query: List(#(String, String))) -> String {
  query
  |> list.map(query_pair)
  |> string.join("&")
}

fn query_pair(pair: #(String, String)) -> String {
  percent_encode_query(pair.0) <> "=" <> percent_encode_query(pair.1)
}

fn percent_encode_query(part: String) -> String {
  percent_encode(part)
  |> string.replace(each: "+", with: "%2B")
}

/// Encodes a string into a percent encoded representation.
///
/// ## Examples
///
/// ```gleam
/// assert uri.percent_encode("100% great") == "100%25%20great"
/// ```
///
pub fn percent_encode(value: String) -> String {
  let bytes = encode_percent_bytes(bit_array.from_string(value), [])
  let assert Ok(value) = bit_array.to_string(bytes)
  value
}

fn encode_percent_bytes(
  bytes: BitArray,
  accumulator: List(BitArray),
) -> BitArray {
  case bytes {
    <<>> -> bit_array.concat(list.reverse(accumulator))
    <<byte, rest:bytes>> -> {
      let encoded = case percent_unescaped(byte) {
        True -> <<byte>>
        False -> <<37, hex_digit(byte / 16), hex_digit(byte % 16)>>
      }
      encode_percent_bytes(rest, [encoded, ..accumulator])
    }
    _ -> <<>>
  }
}

fn percent_unescaped(byte: Int) -> Bool {
  is_unreserved(byte)
  || byte == 33
  || byte == 36
  || byte == 39
  || byte == 40
  || byte == 41
  || byte == 42
  || byte == 43
}

fn hex_digit(value: Int) -> Int {
  case value < 10 {
    True -> value + 48
    False -> value + 55
  }
}

/// Decodes a percent encoded string.
///
/// ## Examples
///
/// ```gleam
/// assert uri.percent_decode("100%25%20great+fun") == Ok("100% great+fun")
/// ```
///
pub fn percent_decode(value: String) -> Result(String, Nil) {
  case decode_percent_bytes(bit_array.from_string(value), False, []) {
    Ok(bytes) -> bit_array.to_string(bytes)
    Error(_) -> Error(Nil)
  }
}

fn decode_percent_bytes(
  bytes: BitArray,
  form: Bool,
  accumulator: List(BitArray),
) -> Result(BitArray, Nil) {
  case bytes {
    <<>> -> Ok(bit_array.concat(list.reverse(accumulator)))
    <<37, high, low, rest:bytes>> -> {
      case hex_value(high), hex_value(low) {
        Ok(high), Ok(low) ->
          decode_percent_bytes(rest, form, [
            <<{ high * 16 + low }>>,
            ..accumulator
          ])
        _, _ -> Error(Nil)
      }
    }
    <<43, rest:bytes>> if form ->
      decode_percent_bytes(rest, form, [<<32>>, ..accumulator])
    // Preserve a final '%' or '%x', matching the Erlang standard library.
    <<byte, rest:bytes>> ->
      decode_percent_bytes(rest, form, [<<byte>>, ..accumulator])
    _ -> Error(Nil)
  }
}

fn hex_value(byte: Int) -> Result(Int, Nil) {
  case byte {
    byte if byte >= 48 && byte <= 57 -> Ok(byte - 48)
    byte if byte >= 65 && byte <= 70 -> Ok(byte - 55)
    byte if byte >= 97 && byte <= 102 -> Ok(byte - 87)
    _ -> Error(Nil)
  }
}

/// Splits the path section of a URI into its constituent segments.
///
/// Removes empty segments and resolves dot-segments as specified in
/// [section 5.2](https://www.ietf.org/rfc/rfc3986.html#section-5.2) of the RFC.
///
/// ## Examples
///
/// ```gleam
/// assert uri.path_segments("/users/1") == ["users", "1"]
/// ```
///
pub fn path_segments(path: String) -> List(String) {
  remove_dot_segments(string.split(path, "/"))
}

fn remove_dot_segments(input: List(String)) -> List(String) {
  remove_dot_segments_loop(input, [])
}

fn remove_dot_segments_loop(
  input: List(String),
  accumulator: List(String),
) -> List(String) {
  case input {
    [] -> list.reverse(accumulator)
    [segment, ..rest] -> {
      let accumulator = case segment, accumulator {
        "", accumulator -> accumulator
        ".", accumulator -> accumulator
        "..", [] -> []
        "..", [_, ..accumulator] -> accumulator
        segment, accumulator -> [segment, ..accumulator]
      }
      remove_dot_segments_loop(rest, accumulator)
    }
  }
}

/// Encodes a `Uri` value as a URI string.
///
/// The opposite operation is `uri.parse`.
///
/// ## Examples
///
/// ```gleam
/// let uri = Uri(..empty, scheme: Some("https"), host: Some("example.com"))
/// assert uri.to_string(uri) == "https://example.com"
/// ```
///
pub fn to_string(uri: Uri) -> String {
  // scheme:[//[userinfo@]host[:port]]path[?query][#fragment]

  let out = case uri.scheme {
    Some(scheme) -> scheme <> ":"
    None -> ""
  }

  let out = case uri.host {
    // Host is mandatory if this is present, so we ignore any port and userinfo
    // if it is not as they would be invalid.
    None -> {
      out <> uri.path
    }

    Some(host) -> {
      let out = out <> "//"
      let out = case uri.userinfo {
        Some(userinfo) -> out <> userinfo <> "@"
        None -> out
      }
      let out = out <> host_to_string(host)
      let out = case uri.port {
        Some(port) -> out <> ":" <> int.to_string(port)
        None -> out
      }
      let out = case uri.path {
        "" -> out
        "/" <> _ -> out <> uri.path
        _ -> out <> "/" <> uri.path
      }
      out
    }
  }

  let out = case uri.query {
    Some(query) -> out <> "?" <> query
    None -> out
  }
  let out = case uri.fragment {
    Some(fragment) -> out <> "#" <> fragment
    None -> out
  }

  out
}

/// Fetches the origin of a URI.
///
/// Returns the origin of a uri as defined in
/// [RFC 6454](https://tools.ietf.org/html/rfc6454)
///
/// The supported URI schemes are `http` and `https`.
/// URLs without a scheme will return `Error`.
///
/// ## Examples
///
/// ```gleam
/// let assert Ok(uri) = uri.parse("https://example.com/path?foo#bar")
/// assert uri.origin(uri) == Ok("https://example.com")
/// ```
///
pub fn origin(uri: Uri) -> Result(String, Nil) {
  let Uri(scheme: scheme, host: host, port: port, ..) = uri
  case host, scheme {
    Some(h), Some("https") if port == Some(443) ->
      Ok("https://" <> host_to_string(h))
    Some(h), Some("http") if port == Some(80) ->
      Ok("http://" <> host_to_string(h))
    Some(h), Some(s) if s == "http" || s == "https" -> {
      case port {
        Some(p) ->
          Ok(s <> "://" <> host_to_string(h) <> ":" <> int.to_string(p))
        None -> Ok(s <> "://" <> host_to_string(h))
      }
    }
    _, _ -> Error(Nil)
  }
}

/// Resolves a URI with respect to the given base URI.
///
/// The base URI must be an absolute URI or this function will return an error.
/// The algorithm for merging URIs is described in
/// [RFC 3986](https://tools.ietf.org/html/rfc3986#section-5.2).
///
pub fn merge(base: Uri, relative: Uri) -> Result(Uri, Nil) {
  case base {
    Uri(scheme: Some(_), host: Some(_), ..) ->
      case relative {
        Uri(host: Some(_), ..) -> {
          let path =
            relative.path
            |> string.split("/")
            |> remove_dot_segments()
            |> join_segments()
          let resolved =
            Uri(
              option.or(relative.scheme, base.scheme),
              None,
              relative.host,
              option.or(relative.port, base.port),
              path,
              relative.query,
              relative.fragment,
            )
          Ok(resolved)
        }
        _ -> {
          let #(new_path, new_query) = case relative.path {
            "" -> #(base.path, option.or(relative.query, base.query))
            _ -> {
              let path_segments = case string.starts_with(relative.path, "/") {
                True -> string.split(relative.path, "/")
                False ->
                  base.path
                  |> string.split("/")
                  |> drop_last()
                  |> list.append(string.split(relative.path, "/"))
              }
              let path =
                path_segments
                |> remove_dot_segments()
                |> join_segments()
              #(path, relative.query)
            }
          }
          let resolved =
            Uri(
              base.scheme,
              None,
              base.host,
              base.port,
              new_path,
              new_query,
              relative.fragment,
            )
          Ok(resolved)
        }
      }
    _ -> Error(Nil)
  }
}

fn drop_last(elements: List(a)) -> List(a) {
  list.take(from: elements, up_to: list.length(elements) - 1)
}

fn join_segments(segments: List(String)) -> String {
  string.join(["", ..segments], "/")
}

// IPv6 hosts are stored without brackets in the Uri record.
fn host_to_string(host: String) -> String {
  case string.contains(host, ":") && !string.starts_with(host, "[") {
    True -> "[" <> host <> "]"
    False -> host
  }
}
