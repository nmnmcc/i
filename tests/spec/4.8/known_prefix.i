// spec: 4.8
// expect: pass

// §5.4: slicing a prefix gives whole String, not the original value set.
(
	testlib = (test) = std/test
)

f = (s: String) {
	"a" <> r = s
	r == "bc"
}

test("prefixes", () {
	s = "abc"
	assert(s |> {
		"a" <> r -> r == "bc"
		_        -> False
	})
	assert(f(s))
})

// §5.4: the excluded full string does not exclude the sliced remainder.
cofinite = (s: String - "a") {
	s |> {
		"x" <> r -> r == "a"
		_        -> False
	}
}

test("cofinite_remainder", () { assert(cofinite("xa")) })
