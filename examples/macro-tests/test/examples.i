(testlib = (test, suite, skip, fails, each) = std/test)

test("adds two numbers", () {
  assert(1 + 1 == 2)
})

skip("not run", () { assert(False) })
fails("expected failure", () { assert(False) })

SuiteService { SuiteService(value: Int) }
read = (service ! SuiteService) { service.value }

suite("outer", () {
  ! SuiteService = SuiteService(42)
  suite("inner", () {
    test("service", () { assert(read() == 42) })
  })
})

each("numbers", [("one", 1), ("two", 2)], (number) { assert(number > 0) })
