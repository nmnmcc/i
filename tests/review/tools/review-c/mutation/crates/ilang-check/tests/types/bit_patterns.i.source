// §8.4: a string format in a pattern needs a string literal, and `bits` or
// `bytes` with no length may only be the last segment.
text_name = (b) {
  case b {
    <<s: utf8, rest: bits>> -> 1
    _ -> 0
  }
}
not_last = (b) {
  case b {
    <<x: bits, c>> -> 1
    <<y: bytes, d: u8>> -> 2
    _ -> 0
  }
}
fine = (b) {
  case b {
    <<"hi": utf8, rest: bytes>> -> 1
    <<x: bits(8), y: u(4), rest: bits>> -> 2
    _ -> 0
  }
}
