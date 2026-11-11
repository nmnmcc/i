// spec: 7.3
// expect: stop "panic: caught group match"

// literal-types §5.3: deferred fitted failure in its own effect frame.
pair = ("b", () {
	pair |> {
		("a", _) -> 1
	}
})


main = () {
	! Assert = Assert((_) { panic("caught group match") })
	pair.1()
}
