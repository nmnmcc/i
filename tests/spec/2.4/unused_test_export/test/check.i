(
	testlib = (test) = std/test
	app/helpers
)

test("check", () { helpers.used() })
