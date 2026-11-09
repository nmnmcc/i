// spec: 9.6, 11.7
// expect: stop "assertion failed: assert"
// at: spec/assert_main:7:3
// at: spec/assert_main:13:3

positive = (n) {
  assert(n > 0)
  n
}

@main
main = () {
  positive(0)
  Nil
}
