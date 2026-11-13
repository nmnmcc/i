// spec: 6.6, 6.11
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

span = (from: Int, to: Int) { to - from }

describe = (name: String, age: Int, city: String) {
	name <> "/" <> core.inspect(age) <> "/" <> city
}

User = (name = String, age = Int)

relabel = (first: Int = x, (a, b), last: Int) { x + first + a + b + last }

test("labels", () {
	check(
		show(span(1, 10)),
		"9",
	)
	check(
		show(span(1, 10)),
		"9",
	)
	check(
		show(span(1, 10)),
		"9",
	)
	check(
		show(span(1, 10)),
		"9",
	)
	check(describe("ann", 3, "oslo"), "ann/3/oslo")
	check(describe("ann", 3, "oslo"), "ann/3/oslo")
	check(show(User(age = 3, name = "x")), "User(\"x\", 3)")
	check(show(User(name = "x", age = 3)), "User(\"x\", 3)")
	check(
		show(relabel(1, (10, 20), 100)),
		"132",
	)
	check(
		show(1 |> relabel((10, 20), 100)),
		"132",
	)
})

test("pipes", () {
	check(
		show(1 |> span(10)),
		"9",
	)
	check(
		show(1 |> span(10)),
		"9",
	)
	check("ann" |> describe(3, "oslo"), "ann/3/oslo")
	check(show([1, 2] |> core.inspect |> core.byte_size), "6")
	check(show(5 |> (n) { n * 2 }), "10")
	check(show("x" |> User(age = 1)), "User(\"x\", 1)")
})
