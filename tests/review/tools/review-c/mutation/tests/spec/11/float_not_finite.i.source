// spec: 11.3, 11.7
// expect: stop "float: result is not finite"
// at: spec/float_not_finite:9:3
// at: spec/float_not_finite:14:3

big = (x: Float) {
  y = x *. 1.0
  // The operator is the innermost expression; its position is its first token.
  y *. 1.0e300
}

@main
main = () {
  big(1.0e300)
  Nil
}
