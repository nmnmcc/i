// spec: 11.6, 6.10
// expect: pass
(
	i/core
)

check = (got: String, want: String) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(got <> " is not " <> want)
	}
}

show = (x) { core.inspect(x) }

// The right side of `&&` and `||` in tail position is a tail position.
even = (n) { n == 0 || odd(n - 1) }

odd = (n) { n != 0 && even(n - 1) }

@test
top_level = () {
	check(show(even(1000000)), "True")
	check(show(odd(1000000)), "False")
	check(show(even(1000001)), "False")
	check(show(odd(1000001)), "True")
}

@test
local = () {
	// The example of §6.10.
	(even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })
	check(show(even(1000000)), "True")
	check(show(odd(1000001)), "True")
	check(show(even(7)), "False")
}

@test
values = () {
	// Not in tail position, `&&` and `||` still give their values.
	check(
		show((
			True && False
			False || True
			False && panic("no")
			True || panic("no")
		))
		"(False, True, False, True)"
	)
}
