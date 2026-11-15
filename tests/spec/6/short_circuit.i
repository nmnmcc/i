// spec: 6.2, 6.9
// expect: pass
(
	testlib = (test) = std/test
	i/core
)

check = (got : String, want : String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

test("and_or", () {
	check(
		core.inspect(False && panic("and")),
		"False",
	)
	check(
		core.inspect(True || panic("or")),
		"True",
	)
	check(core.inspect(True && False), "False")
	check(core.inspect(False || True), "True")
	check(
		core.inspect(True && True || panic("never")),
		"True",
	)
})
