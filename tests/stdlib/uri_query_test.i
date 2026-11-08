import i/uri

pub fn query_form_decoding_test() -> Nil {
  assert uri.parse_query("weebl+bob=1&city=%C3%B6rebro")
    == Ok([#("weebl bob", "1"), #("city", "örebro")])
}

pub fn query_duplicate_keys_remain_ordered_test() -> Nil {
  assert uri.parse_query("a[]=1&a[]=2&a[]=3")
    == Ok([#("a[]", "1"), #("a[]", "2"), #("a[]", "3")])
}

pub fn query_empty_input_and_missing_value_test() -> Nil {
  assert uri.parse_query("") == Ok([])
  assert uri.parse_query("present") == Ok([#("present", "")])
  assert uri.parse_query("present=") == Ok([#("present", "")])
  assert uri.parse_query("=value") == Ok([#("", "value")])
}

pub fn query_retains_empty_sections_test() -> Nil {
  assert uri.parse_query("&") == Ok([#("", ""), #("", "")])
  assert uri.parse_query("a=1&&") == Ok([#("a", "1"), #("", ""), #("", "")])
}

pub fn query_only_first_equals_separates_test() -> Nil {
  assert uri.parse_query("key=a=b=c") == Ok([#("key", "a=b=c")])
  assert uri.parse_query("a%3Db=c%3Dd") == Ok([#("a=b", "c=d")])
}

pub fn query_plus_and_encoded_plus_differ_test() -> Nil {
  assert uri.parse_query("x=one+two%2Bthree&%2B=+")
    == Ok([#("x", "one two+three"), #("+", " ")])
}

pub fn query_numeric_character_reference_test() -> Nil {
  assert uri.parse_query("city=%26%2326481%3B%26%2320140%3B")
    == Ok([#("city", "東京")])
  assert uri.parse_query("symbol=%26%239731%3B") == Ok([#("symbol", "☃")])
}

pub fn query_rejects_invalid_encoding_test() -> Nil {
  assert uri.parse_query("%C2") == Error(Nil)
  assert uri.parse_query("x=%FF") == Error(Nil)
  assert uri.parse_query("x=%zz") == Error(Nil)
}

pub fn query_incomplete_escape_matches_percent_decode_test() -> Nil {
  assert uri.parse_query("x=%") == Ok([#("x", "%")])
  assert uri.parse_query("x=%A") == Ok([#("x", "%A")])
}

pub fn query_encoding_roundtrip_test() -> Nil {
  let pairs = [#("snow 雪", "1+2=3"), #("emoji", "😀"), #("empty", "")]
  let encoded = uri.query_to_string(pairs)
  assert encoded == "snow%20%E9%9B%AA=1%2B2%3D3&emoji=%F0%9F%98%80&empty="
  assert uri.parse_query(encoded) == Ok(pairs)
}

pub fn query_empty_key_roundtrip_test() -> Nil {
  let pairs = [#("", "value"), #("", ""), #("key", "")]
  assert uri.parse_query(uri.query_to_string(pairs)) == Ok(pairs)
}

pub fn query_encoding_special_characters_test() -> Nil {
  assert uri.query_to_string([#("key", "!$'()*+,-._~;/%")])
    == "key=!$'()*%2B%2C-._~%3B%2F%25"
  assert uri.query_to_string([]) == ""
}
