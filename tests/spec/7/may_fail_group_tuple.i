// spec: 7.3
// expect: stop "panic: caught group tuple"

// literal-types §5.3: deferred fitted failure in its own effect frame.
pair = ("b", () {
	("a", _) = pair
	1
})

@main
main = () {
	! Assert = Assert((_) { panic("caught group tuple") })
	pair.1()
}
