// spec: 11.3, 11.7
// expect: stop "float: result is not finite"
// at: spec/main:8:2
// at: spec/main:12:2
big = (x: Float) {
	y = x *. 1.0
	// The operator is the innermost expression; its position is its first token.
	y *. 1.0e300
}

main = () {
	big(1.0e300)
	Nil
}
