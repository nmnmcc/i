(
	testlib = (test, suite, each) = i/test
)

First = (value = Int)
Second = (value = Int)

values = () <- (first : First, second : Second) { (first.value, second.value) }

suite("provided groups", () {
	{
		test("outer before", () { assert(values() == (42, 7)) })
		testlib.suite("inner", () {
			test("override", () { assert(values() == (43, 8)) })
			each("cases", [("one", 1), ("two", 2)], (number) {
				assert(values() == (43, 8) && number > 0)
			})
		}) <- (First(value = 43), Second(value = 8))
		test("outer after", () { assert(values() == (42, 7)) })
	} <- (First(value = 42), Second(value = 7))
	test("sibling", () { assert(values() == (44, 9)) }) <- (
		First(value = 44),
		Second(value = 9),
	)
})
