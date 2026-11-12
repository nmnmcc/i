(
	testlib = (test) = std/test
	app/api
)

helper = () { assert(api.public() == 1) }

test("check", () { helper() })
