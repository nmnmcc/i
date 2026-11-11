// spec: 7.3
// expect: stop "panic: caught group ctor"

// literal-types §5.3: deferred fitted failure in its own effect frame.
Choice {
	No
	Yes(Int)
}

pair = (No, () {
	(Yes(n), _) = pair
	n
})


main = () {
	! Assert = Assert((_) { panic("caught group ctor") })
	pair.1()
}
