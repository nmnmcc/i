(
	testlib = (test, suite, skip, fails, each) = std/test
	fail = i/fail
)

test("adds two numbers", () { assert(1 + 1 == 2) })

skip("not run", () { assert(False) })

fails("expected failure", () { assert(False) })

Problem = ()

fails("typed error", () { fail.raise(Problem) })

SuiteService = (value = Int)

read = () <- (service: SuiteService) { service.value }

suite("outer", () {
	suite("inner", () {
		test("service", () { assert(read() == 42) })
		each("cases", [("one", 1), ("two", 2)], (number) {
			assert(read() == 42 && number > 0)
		})
	}) <- (SuiteService(value = 42))
})

each("numbers", [("one", 1), ("two", 2)], (number) { assert(number > 0) })
