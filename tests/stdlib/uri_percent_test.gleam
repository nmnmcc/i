import gleam/uri

pub fn percent_encode_safe_characters_test() -> Nil {
  assert uri.percent_encode("AZaz09!$'()*+-._~") == "AZaz09!$'()*+-._~"
}

pub fn percent_encode_reserved_characters_test() -> Nil {
  assert uri.percent_encode(" ,;:?[]@/\\&#=%")
    == "%20%2C%3B%3A%3F%5B%5D%40%2F%5C%26%23%3D%25"
}

pub fn percent_codec_unicode_bytes_test() -> Nil {
  assert uri.percent_encode("ñ雪😀") == "%C3%B1%E9%9B%AA%F0%9F%98%80"
  assert uri.percent_decode("%C3%B1%E9%9B%AA%F0%9F%98%80") == Ok("ñ雪😀")
}

pub fn percent_codec_combining_sequence_test() -> Nil {
  assert uri.percent_encode("e\u{301}") == "e%CC%81"
  assert uri.percent_decode("e%CC%81") == Ok("e\u{301}")
}

pub fn percent_decode_accepts_lowercase_hex_test() -> Nil {
  assert uri.percent_decode("%c3%b1%f0%9f%98%80") == Ok("ñ😀")
}

pub fn percent_decode_keeps_plus_test() -> Nil {
  assert uri.percent_decode("100%25%20great+fun") == Ok("100% great+fun")
  assert uri.percent_decode("%2B+") == Ok("++")
}

pub fn percent_decode_preserves_incomplete_escape_test() -> Nil {
  // The fixed Erlang stdlib baseline treats an incomplete escape literally.
  assert uri.percent_decode("tail%") == Ok("tail%")
  assert uri.percent_decode("tail%A") == Ok("tail%A")
}

pub fn percent_decode_rejects_invalid_hex_test() -> Nil {
  assert uri.percent_decode("%GG") == Error(Nil)
  assert uri.percent_decode("%0x") == Error(Nil)
  assert uri.percent_decode("before%xxafter") == Error(Nil)
}

pub fn percent_decode_rejects_invalid_utf8_test() -> Nil {
  assert uri.percent_decode("%C2") == Error(Nil)
  assert uri.percent_decode("%80") == Error(Nil)
  assert uri.percent_decode("%C0%AF") == Error(Nil)
  assert uri.percent_decode("%ED%A0%80") == Error(Nil)
  assert uri.percent_decode("%F4%90%80%80") == Error(Nil)
}

pub fn percent_codec_empty_and_nul_test() -> Nil {
  assert uri.percent_encode("") == ""
  assert uri.percent_decode("") == Ok("")
  assert uri.percent_encode("\u{0}") == "%00"
  assert uri.percent_decode("%00") == Ok("\u{0}")
}

pub fn percent_decode_decodes_once_test() -> Nil {
  assert uri.percent_decode("%252F") == Ok("%2F")
}
