// spec: 1.1, 6.10, 11.7
// expect: stop "assertion failed: pattern [a,"
// at: b] did not match [1]
// at: spec/binding_text_crlf:9:3

// The source text of the pattern has `\n` for each `\r\n`.

main = () {
  [a,
b] = [1]
  Nil
}
