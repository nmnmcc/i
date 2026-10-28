import gleam/wleam_unicode as unicode

pub fn unicode_empty_test() {
  assert unicode.graphemes("") == []
  assert unicode.lowercase("") == ""
  assert unicode.uppercase("") == ""
}

pub fn unicode_final_sigma_test() {
  assert unicode.lowercase("ΟΣ") == "ος"
  assert unicode.lowercase("ΟΣΑ") == "οσα"
  assert unicode.lowercase("Σ") == "σ"
  assert unicode.lowercase("ΑΣ\u{301}") == "ας\u{301}"
  assert unicode.lowercase("ΑΣ\u{301}Α") == "ασ\u{301}α"
  assert unicode.lowercase("Α'Σ") == "α'ς"
  assert unicode.lowercase("ΑΣ'Α") == "ασ'α"
  assert unicode.lowercase("Α Σ") == "α σ"
}

pub fn unicode_full_mapping_order_test() {
  assert unicode.uppercase("Straße ﬃ") == "STRASSE FFI"
  assert unicode.lowercase("İABC") == "i\u{307}abc"
  assert unicode.uppercase("ΐ") == "Ι\u{308}\u{301}"
}

pub fn unicode_grapheme_preservation_test() {
  assert unicode.graphemes("a\u{301}\r\nb") == ["a\u{301}", "\r\n", "b"]
  assert unicode.graphemes("👩🏽‍🚀🇺🇳🇹🇼") == ["👩🏽‍🚀", "🇺🇳", "🇹🇼"]
  assert unicode.graphemes("🇺🇳🇹") == ["🇺🇳", "🇹"]
  assert unicode.graphemes("\u{915}\u{94d}\u{915}") == ["\u{915}\u{94d}\u{915}"]
  assert unicode.from_codepoints(unicode.codepoints("a\u{301}")) == "a\u{301}"
}

pub fn unicode_pattern_whitespace_test() {
  assert unicode.is_pattern_whitespace(9)
  assert unicode.is_pattern_whitespace(13)
  assert unicode.is_pattern_whitespace(32)
  assert unicode.is_pattern_whitespace(133)
  assert unicode.is_pattern_whitespace(8206)
  assert unicode.is_pattern_whitespace(8207)
  assert unicode.is_pattern_whitespace(8232)
  assert unicode.is_pattern_whitespace(8233)
  assert !unicode.is_pattern_whitespace(160)
  assert !unicode.is_pattern_whitespace(8192)
  assert !unicode.is_pattern_whitespace(8239)
  assert !unicode.is_pattern_whitespace(-1)
}
