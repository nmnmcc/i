// spec: 6.8, 6.11
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

show = (x) { core.inspect(x) }

adder = (n) { (x) { x + n } }

compose = (f, g) { (x) { g(f(x)) } }

map = (xs, f) { xs |> ([]) { [] } | ([x, ..rest]) { [f(x), ..map(rest, f)] } }

fold = (xs, acc, f) {
	xs |> ([]) { acc } | ([x, ..rest]) { fold(rest, f(acc, x), f) }
}

test("captures", () {
	add5 = adder(5)
	check(
		show(add5(1)),
		"6",
	)
	check(
		show(adder(-1)(1)),
		"0",
	)
	base = 100
	scale = 3
	f = (x) { base + x * scale }
	check(
		show(f(2)),
		"106",
	)
	nested = (a) { (b) { (c) { (a, b, c, base) } } }
	check(
		show(nested(1)(2)(3)),
		"(1, 2, 3, 100)",
	)
	check(
		show(compose(adder(1), (x) { x * 10 })(4)),
		"50",
	)
})

test("higher_order", () {
	k = 3
	check(
		show(map([1, 2, 3], (x) { x * k })),
		"[3, 6, 9]",
	)
	check(
		show(fold([1, 2, 3, 4], 0, (a, b) { a + b })),
		"10",
	)
	fs = map([1, 2, 3], adder)
	check(
		show(map(fs, (g) { g(10) })),
		"[11, 12, 13]",
	)
	check(
		show(map(["a", "b"], core.inspect)),
		"[\"\\\"a\\\"\", \"\\\"b\\\"\"]",
	)
})

test("patterns_in_parameters", () {
	swap = ((a, b)) { (b, a) }
	check(
		show(swap((1, 2))),
		"(2, 1)",
	)
	both = ((a, _) = p, _ : Int) { (p, a) }
	check(
		show(both((1, 2), 3)),
		"((1, 2), 1)",
	)
	typed = (x : Int, y : String) -> String { core.inspect(x) <> y }
	check(typed(1, "!"), "1!")
})
