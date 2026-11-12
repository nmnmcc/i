// spec: 7.3
// expect: stop "panic: caught group ctor"

// literal-types §5.3: deferred fitted failure in its own effect frame.
Choice = No | Yes

No {
	No
}

Yes {
	Yes(Int)
}

pair = (No, () {
	// Written `Choice`: inferred, the recursive `pair` would make the part
	// `Yes` (syntax three D4 widens only with known other values).
	c: Choice = pair.0
	Yes(n) = c
	n
})


main = () {
	! Assert = Assert((_) { panic("caught group ctor") })
	pair.1()
}
