import gleam/io
import gleam/string

pub fn main() {
  let family = "👨‍👩‍👧‍👦"
  let combining = "á"
  assert string.length(family <> combining) == 2
  assert string.to_graphemes(family <> combining) == [family, combining]
  assert string.reverse(family <> combining) == combining <> family
  assert string.uppercase("Straße") == "STRASSE"
  assert string.lowercase("Straße") == "straße"
  assert string.byte_size("你好") == 6
  assert string.slice("ab你好", 2, 2) == "你好"
  assert string.from_utf_codepoints(string.to_utf_codepoints("🙂你好")) == "🙂你好"
  io.println("PASS:unicode")
}
