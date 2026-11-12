(
	testlib = (test, suite, skip, fails, each) = std/test
	fail = i/fail
)

test("adds two numbers", () { assert(1 + 1 == 2) })

skip("not run", () { assert(False) })

fails("expected failure", () { assert(False) })

Problem {
	Problem
}

fails("typed error", () { fail.raise(Problem) })

SuiteService {
	SuiteService(value: Int)
}

read = (service ! SuiteService) { service.value }

suite("outer", () {
	! SuiteService = SuiteService(value: 42)
	suite("inner", () {
		test("service", () { assert(read() == 42) })
		each("cases", [("one", 1), ("two", 2)], (number) {
			assert(read() == 42 && number > 0)
		})
	})
})

each("numbers", [("one", 1), ("two", 2)], (number) { assert(number > 0) })
