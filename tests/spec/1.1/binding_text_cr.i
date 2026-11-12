// spec: 1.1, 6.10, 11.7
// expect: stop "assertion failed: pattern [a,"
// at: b] did not match [1]
// at: spec/main:9:3

// The source text of the pattern has `\n` for each `\r`.

main = () {
  [a,b] = [1]
  Nil
}
