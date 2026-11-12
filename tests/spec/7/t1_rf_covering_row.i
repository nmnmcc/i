// spec: 4.8, 7.4
// expect: pass
(
	testlib = (test) = std/test
)

strict = (s: String - "a") {
	s |> {
		"b"                   -> 1
		_: String - "a" - "b" -> 2
	}
}

f = (p: (String, Bool)) {
	p |> {
		("a", _) -> 0
		(t, _)   -> strict(t)
	}
}

test("whole_other_column_is_covered", () {
	assert(f(("a", False)) == 0)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 2)
})
