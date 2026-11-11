// spec: 7.3
// expect: stop "panic: caught group call"

// literal-types §5.3: deferred fitted failure in its own effect frame.
pair = ((x) { x }, () {
	"a" = pair.0("b")
	1
})


main = () {
	! Assert = Assert((_) { panic("caught group call") })
	pair.1()
}
