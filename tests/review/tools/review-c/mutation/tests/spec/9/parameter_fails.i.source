// spec: 6.11, 9.6, 11.7
// expect: stop "assertion failed: pattern (1, x) did not match (2, 3)"
// at: spec/parameter_fails:6:11
// at: spec/parameter_fails:10:3

second = ((1, x)) { x }

@main
main = () {
  second((2, 3))
  Nil
}
