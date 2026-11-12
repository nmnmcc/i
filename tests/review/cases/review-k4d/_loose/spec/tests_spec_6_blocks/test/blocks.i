// spec: 6.10, 5
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

pair = (a, b) { (a, b) }

test("shadowing", () {
	x = 1
	x = x + 1
	y = {
		x = x * 10
		x + 1
	}
	check(show((x, y)), "(2, 21)")
	check = "a local name may cover a top-level name"
	{ check == "" } |> {
		True  -> panic("no")
		False -> Nil
	}
})

test("values_in_every_place", () {
	n = 3
	check(show(pair(n |> {
		3 -> "three"
		_ -> "other"
	}, {
		m = n + 1
		m * 2
	})), "(\"three\", 8)")
	check(show([{ n > 2 } |> {
		True  -> 1
		False -> 2
	}, ..n |> {
		_ -> [9]
	}]), "[1, 9]")
	s = (n, "x") |> {
		(3, t) -> t <> t
		_      -> ""
	}
	check(s, "xx")
})

test("recursion_inside_closures", () {
	outer = (limit) {
		// A direct binding inside a function body, whose names are used inside
		// its own function bodies.
		(up, down) = ((i) {
			{ i >= limit } |> {
				True  -> down(i)
				False -> up(i + 1)
			}
		}, (i) { [i, limit] })
		up
	}
	check(show(outer(5)(0)), "[5, 5]")
	f = outer(2)
	check(show(f(1)), "[2, 2]")
	check(show(f == f), "True")
	check(show(outer(2) == outer(2)), "False")
})

test("parameters_that_name_the_whole_value", () {
	g = (all = (a, b), c: Int) { show((all, a + b + c)) }
	check(g((1, 2), 3), "((1, 2), 6)")
})
