// spec: 7.3
// expect: stop "panic: own"

// literal-types §5.3: deferred fitted failure in its own effect frame.
f = (s : String) -> Int {
	(a, b) = (1, 2)
	{
		"a" = s
		a
	} <- (Assert(fail = (_) { panic("own") }))
}

main = () { f("b") }
