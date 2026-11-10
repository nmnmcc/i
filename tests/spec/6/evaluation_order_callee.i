// spec: 6.2
// expect: stop "panic: callee"
// at: spec/evaluation_order_callee:7:35
// at: spec/evaluation_order_callee:12:5

// The function is worked out before the arguments.
pick = (n: Int) -> (Int) -> Int { panic("callee") }

@main
main = () {
	n = 1
	{ pick(n) }(panic("argument"))
	Nil
}
