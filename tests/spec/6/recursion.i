// spec: 6.10, 2.5
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

show = (x) { core.inspect(x) }

is_even = (n) { n |> (0) { True } | (_) { is_odd(n - 1) } }

is_odd = (n) { n |> (0) { False } | (_) { is_even(n - 1) } }

test("local_recursion", () {
	fact = (n) { n |> (0) { 1 } | (_) { n * fact(n - 1) } }
	check(
		show(fact(25)),
		"15511210043330985984000000",
	)
	fib = (n) {
		{ n < 2 } |> (True) { n } | (False) { fib(n - 1) + fib(n - 2) }
	}
	check(
		show(fib(20)),
		"6765",
	)
})

test("local_mutual_recursion", () {
	(even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })
	check(
		show(even(10)),
		"True",
	)
	check(
		show(odd(7)),
		"True",
	)
	check(
		show(even(7)),
		"False",
	)
	limit = 3
	(down, up) = (
		(n) { n |> (0) { up(0) } | (_) { down(n - 1) } },
		(n) { { n < limit } |> (True) { up(n + 1) } | (False) { n } },
	)
	check(
		show(down(5)),
		"3",
	)
})

test("top_level_mutual_recursion", () {
	check(
		show(is_even(1001)),
		"False",
	)
	check(
		show(is_odd(1001)),
		"True",
	)
})
