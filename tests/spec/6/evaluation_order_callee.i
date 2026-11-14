// spec: 6.2
// expect: stop "panic: callee"
// at: spec/main:7:36
// at: spec/main:11:4

// The function is worked out before the arguments.
pick = (n : Int) -> (Int) -> Int { panic("callee") }

main = () {
	n = 1
	{ pick(n) }(panic("argument"))
	Nil
}
