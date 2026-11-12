// spec: 11.2
// expect: pass
(
	testlib = (test) = std/test
	i/core
)

check = (got: String, want: String) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(got <> " is not " <> want)
	}
}

show = (x) { core.inspect(x) }

test("small", () {
	check(show(1 + 2), "3")
	check(show(10 - 25), "-15")
	check(show(6 * 7), "42")
	check(show(-{ 4 }), "-4")
	check(show(-4 - -6), "2")
	check(show(2 + 3 * 4), "14")
	check(show({ 2 + 3 } * 4), "20")
	check(show(10 - 2 - 3), "5")
})

test("big", () {
	two_100 = 1267650600228229401496703205376
	check(
		show(two_100 * two_100)
		"1606938044258990275541962092341162602522202993782792835301376"
	)
	check(show(two_100 - two_100 - 1), "-1")
	check(show(1073741823 + 1), "1073741824")
	check(show(-1073741824 - 1), "-1073741825")
	check(show(4611686018427387904 * 4), "18446744073709551616")
	check(show(18446744073709551616 / 3), "6148914691236517205")
	check(show(-18446744073709551617 % 10), "-7")
	check(show(9223372036854775807 + 1), "9223372036854775808")
})

test("division", () {
	check(show(7 / 2), "3")
	check(show(-7 / 2), "-3")
	check(show(7 / -2), "-3")
	check(show(-7 / -2), "3")
	check(show(7 % 2), "1")
	check(show(-7 % 2), "-1")
	check(show(7 % -2), "1")
	check(show(-7 % -2), "-1")
	check(show(5 / 0), "0")
	check(show(5 % 0), "0")
	check(show(-12345678901234567890 / 0), "0")
	check(show(-12345678901234567890 % 0), "0")
})

test("compare", () {
	check(show(1 < 2), "True")
	check(show(2 <= 2), "True")
	check(show(3 > 4), "False")
	check(show(-5 >= -5), "True")
	check(show(100000000000000000000 > 99999999999999999999), "True")
	check(show(-100000000000000000000 < 1), "True")
})
