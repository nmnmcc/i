// spec: 6.2
// expect: stop "panic: first"
// at: spec/main:12:10

// The arguments of a call are worked out from left to right as they are
// written, also with labels.
pair = (a: Int, b: Int) { a + b }

main = () {
	x = 1
	{
		_syn3_0 = panic("first")
		_syn3_1 = panic("second")
		pair(_syn3_1, _syn3_0)
	}
}
