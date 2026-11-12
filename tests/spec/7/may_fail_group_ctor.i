No = ()
Yes = (Int)
// spec: 7.3
// expect: stop "panic: caught group ctor"

// literal-types §5.3: deferred fitted failure in its own effect frame.
Choice = No | Yes

pair = (
	No,
	() {
		(Yes(n), _) = pair
		n
	},
)

main = () { pair.1() <- (Assert(fail = (_) { panic("caught group ctor") })) }
