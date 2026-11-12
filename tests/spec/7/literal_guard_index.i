// spec: 7.4
// expect: pass
// Guarded literals never enter the prefix of rows that cover later rows.
(
	testlib = (test) = std/test
)

f = (s: "a" | "b") {
	s |> {
		"a" ? False -> 0
		"a"         -> 1
		"b"         -> 2
	}
}

test("later_rows", () {
	assert(f("a") == 1)
	assert(f("b") == 2)
})
