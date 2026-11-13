// spec: 7.1, 7.4
// expect: pass

// §5.6: Only tests use value equality; Except tests also retain their scalar tag.
(
	testlib = (test) = std/test
)

strings = (v: { Int | String }) { v |> (x: { "a" | "b" }) { True } | (_) { False } }

except_string = (v: { Int | String }) {
	v |> (x: String - "a") { True } | (_) { False }
}

except_int = (v: { Int | String }) { v |> (x: Int - 5) { True } | (_) { False } }

boxed = (v: { Int | String }) { v |> (x: 1073741824) { True } | (_) { False } }

equal = (a, b) { a == b }

test("scalar_tests", () {
	assert(strings("a"))
	assert(strings("b"))
	assert(
		equal(strings("c"), False),
	)
	assert(
		equal(strings(1), False),
	)
	assert(except_string("b"))
	assert(
		equal(except_string("a"), False),
	)
	assert(
		equal(except_string(3), False),
	)
	assert(
		equal(except_string(1073741824), False),
	)
	assert(
		equal(except_string(-1073741825), False),
	)
	assert(
		equal(except_string(1000000000000000000000000000000), False),
	)
	assert(
		equal(except_int(5), False),
	)
	assert(
		equal(except_int("5"), False),
	)
	assert(except_int(1073741824))
	assert(except_int(-1073741825))
	assert(except_int(1000000000000000000000000000000))
	assert(boxed(1073741824))
	assert(
		equal(boxed(1073741825), False),
	)
	assert(
		equal(boxed("1073741824"), False),
	)
})
