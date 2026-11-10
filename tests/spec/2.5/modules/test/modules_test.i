(
	i/core
	app/app/top
	app/app/base
)

check = (got: String, want: String) {
	{ got == want } |> {
		True  -> Nil
		False -> panic(got <> " is not " <> want)
	}
}

@test
across_modules = () {
	check(core.inspect(top.answer), "42")
	check(core.inspect(top.colors), "[Red, Blue(0), Green]")
}

each = (xs: List(base.Color)) {
	xs |> {
		[]          -> []
		[c, ..rest] -> [base.describe(c), ..each(rest)]
	}
}

@test
constructors_of_another_module = () {
	check(core.inspect(each(top.colors)), "[\"red\", \"blue 0\", \"green\"]")
}
