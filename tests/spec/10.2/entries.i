// spec: 10.2, B.5
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

counter = [1, 2, 3]

// The value of the program entry is not used.
@main
main = () {
	check(core.inspect(counter), "[1, 2, 3]")
	core.inspect(counter)
}

@test
first = () { check(core.inspect(counter), "[1, 2, 3]") }

@test
second = () { check(core.inspect(main()), "\"[1, 2, 3]\"") }
