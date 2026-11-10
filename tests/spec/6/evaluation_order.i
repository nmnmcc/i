// spec: 6.2
// expect: stop "panic: first"
// at: spec/evaluation_order:12:11

// The arguments of a call are worked out from left to right as they are
// written, also with labels.
pair = (a: Int, b: Int) { a + b }

@main
main = () {
	x = 1
	pair(b: panic("first"), a: panic("second"))
}
