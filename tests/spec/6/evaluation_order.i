// spec: 6.2
// expect: stop "panic: first"
// at: spec/main:11:7

// The arguments of a call are worked out from left to right as they are
// written.
pair = (a : Int, b : Int) { a + b }

main = () {
	x = 1
	pair(panic("first"), panic("second"))
}
