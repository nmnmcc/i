// spec: 6.9, 4.2
// expect: pass
(
	testlib = (test) = std/test
	i/core
)

check = (got: String, want: String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

test("append", () {
	check("a" <> "b", "ab")
	check("" <> "", "")
	check("héllo" <> " " <> "wörld", "héllo wörld")
	check("multi\nline", "multi\nline")
	check("\u{1F600}" <> "!", "😀!")
	check("tab\there", "tab	here")
	check(core.inspect("a" <> "b" == "ab"), "True")
	check(core.inspect("a" != "b"), "True")
})

test("escapes", () {
	check(
		core.inspect(core.byte_size("\n\r\t\f\"\\")),
		"6",
	)
	check(
		core.inspect(core.byte_size("\u{0}\u{7FF}\u{FFFF}\u{10FFFF}")),
		"10",
	)
})
