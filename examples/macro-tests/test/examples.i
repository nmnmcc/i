(
	testlib = (test, suite, skip, fails, each) = std/test
	fail = i/fail
	typed = macro_tests/typed
)

test("adds two numbers", () { assert(1 + 1 == 2) })

skip("not run", () { assert(False) })

fails("expected failure", () { assert(False) })

Problem = ()

fails("typed error", () { fail.raise(Problem) })

SuiteService = (value = Int)

read = () <- (service : SuiteService) { service.value }

suite("outer", () {
	suite("inner", () {
		test("service", () { assert(read() == 42) })
		each("cases", [("one", 1), ("two", 2)], (number) {
			assert(read() == 42 && number > 0)
		})
	}) <- (SuiteService(value = 42))
})

each("numbers", [("one", 1), ("two", 2)], (number) { assert(number > 0) })

test("typed macro converts checked Int and String expressions", () {
	assert(typed.to_text(42) == "42")
	assert(typed.to_text("hello") == "hello")
	typed.assert_eq(6 * 7, 42)
})

suite("block services", () {
	{
		test("one", () { assert(read() == 21) })
		test("two", () { assert(read() == 21) })
	} <- (SuiteService(value = 21))
})

suite("nested override", () {
	{
		suite("inner", () { test("service", () { assert(read() == 84) }) }) <- (
			SuiteService(value = 84),
		)
		test("outer", () { assert(read() == 42) })
	} <- (SuiteService(value = 42))
})
