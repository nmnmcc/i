// spec: 7.3
// expect: stop "panic: caught group parameter"

// literal-types §5.3: deferred fitted failure in its own effect frame.
pair = ("b", {
	("a", _) -> 1
})


main = () {
	! Assert = Assert((_) { panic("caught group parameter") })
	pair.1(pair)
}
