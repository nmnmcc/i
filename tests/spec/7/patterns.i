// spec: 7.1, 7.2, 6.12
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

User = (name = String, age = Int)

Point = (Int, Int, label = String)

Some(A) = (A)
None = ()
Option(A) = Some(A) | None

literal = (x : { Int | String }) {
	x |>
		(0) { "zero" }
		| (-1) { "minus one" }
		| (100000000000000000000) { "big" }
		| (-100000000000000000000) { "minus big" }
		| (_) { "other" }
}

float = (x) {
	x |>
		(0.0) { "zero" }
		| (-.1.5) { "minus 1.5" }
		| (2.5) { "2.5" }
		| (_) { "other" }
}

string = (s) {
	s |>
		("") { "empty" }
		| ("abc") { "abc" }
		| ("key=" <> value) { "value " <> value }
		| ("é" <> _) { "starts with é" }
		| ("x" <> rest = whole) { whole <> "/" <> rest }
		| (_) { "other" }
}

list = (xs) {
	xs |>
		([]) { "empty" }
		| ([x]) { "one " <> show(x) }
		| ([x, y]) { "two " <> show((x, y)) }
		| ([1, ..]) { "starts with 1" }
		| ([_, _, ..rest]) { "rest " <> show(rest) }
}

test("literals", () {
	check(literal(0), "zero")
	check(literal(-1), "minus one")
	check(literal(100000000000000000000), "big")
	check(literal(-100000000000000000000), "minus big")
	check(literal(5), "other")
	check(literal(100000000000000000000 - 99999999999999999999 - 1), "zero")
	check(literal(-100000000000000000000 + 99999999999999999999), "minus one")
	check(
		literal(core.truncate(-.1.5)),
		"minus one",
	)
	check(literal(10000000000 * 10000000000), "big")
	check(literal("0"), "other")
	check(float(0.0), "zero")
	check(float(-.0.0), "zero")
	check(float(-.1.5), "minus 1.5")
	check(float(2.5), "2.5")
	check(float(2.4), "other")
	check(string(""), "empty")
	check(string("abc"), "abc")
	check(string("key=v"), "value v")
	check(string("key="), "value ")
	check(string("éa"), "starts with é")
	check(string("xyz"), "xyz/yz")
	check(string("ab"), "other")
})

test("lists", () {
	check(list([]), "empty")
	check(list([7]), "one 7")
	check(list([7, 8]), "two (7, 8)")
	check(list([1, 2, 3]), "starts with 1")
	check(list([2, 3, 4, 5]), "rest [4, 5]")
})

test("constructors", () {
	u = User(name = "ann", age = 30)
	u |> (User(age = age, name = name)) { check(name <> show(age), "ann30") }
	u |> (User(name = n, ..)) { check(n <> name, "annann") }
	u |> (User(..)) { check("any", "any") }
	u |> (User(age = age : Int, name = name : String)) { check(name, "ann") }
	Point(1, 2, label = "p") |> (Point(x, y, label = label)) {
		check(show((x, y, label)), "(1, 2, \"p\")")
	}
	Point(1, 2, label = "p") |> (Point(1, _, ..)) { Nil } | (_) { panic("no") }
	Some(Some(3)) |>
		(Some(None)) { panic("no") }
		| (Some(Some(n))) { check(show(n), "3") }
		| (None) { panic("no") }
	res : Result(List(Int), String) = Ok([1])
	res |>
		(Error(_)) { panic("no") }
		| (Ok([])) { panic("no") }
		| (Ok([x])) { check(show(x), "1") }
		| (Ok(_)) { panic("no") }
	(True, False, Nil) |>
		((True, True, _)) { panic("no") }
		| ((True, False, Nil)) { Nil }
		| (_) { panic("no") }
})

test("named_and_tuples", () {
	(1, (2, 3)) |> ((a, (b, c) = inner)) {
		check(show((a, inner, b, c)), "(1, (2, 3), 2, 3)")
	}
	(Nil, (1)) |> ((Nil, (x))) { check(show(x), "1") }
})

kind = (v : { Int | String | List(Int) | (Int, Int) | Nil | Bool | Float }) {
	v |>
		(i : Int) { "int " <> show(i) }
		| (s : String) { "string " <> s }
		| (_ : List(_)) { "list" }
		| ((a, b)) { "pair " <> show(a + b) }
		| (Nil) { "nil" }
		| (b : Bool) { "bool " <> show(b) }
		| (f : Float) { "float " <> show(f) }
}

test("type_tests", () {
	check(kind(1), "int 1")
	check(kind(100000000000000000000), "int 100000000000000000000")
	check(kind("s"), "string s")
	check(kind([1]), "list")
	check(kind([]), "list")
	check(kind((1, 2)), "pair 3")
	check(kind(Nil), "nil")
	check(kind(False), "bool False")
	check(kind(1.5), "float 1.5")
})

choices = (x) {
	x |>
		(0 | 1 | 2) { "small" }
		| (Some(n) | Ok(n) ? n > 10) { "big " <> show(n) }
		| (_) { "other" }
}

pick = (p) { p |> ((n, 0) | (0, n)) { n } | (_) { -1 } }

positive = (p) { p |> ((n, _) | (_, n) ? n > 0) { n } | (_) { -1 } }

test("choices_and_guards", () {
	check(choices(1), "small")
	check(choices(2), "small")
	check(choices(3), "other")
	check(
		show(pick((5, 0))),
		"5",
	)
	check(
		show(pick((0, 6))),
		"6",
	)
	// The first choice that matches binds the names.
	check(
		show(pick((0, 0))),
		"0",
	)
	check(
		show(pick((1, 1))),
		"-1",
	)
	// A choice whose guard is `False` goes on with the next choice.
	check(
		show(positive((0, 5))),
		"5",
	)
	check(
		show(positive((3, 5))),
		"3",
	)
	check(
		show(positive((0, -5))),
		"-1",
	)
	n = 4
	r = n |> (x ? x > 5) { "more" } | (x ? x > 3) { "between" } | (_) { "less" }
	check(r, "between")
})

guarded = (x : Option(Int)) {
	x |>
		(Some(n) ? n > 0) { "positive" }
		| (Some(0)) { "zero" }
		| (Some(_)) { "negative" }
		| (None) { "none" }
}

test("guards", () {
	check(guarded(Some(3)), "positive")
	check(guarded(Some(0)), "zero")
	check(guarded(Some(-3)), "negative")
	check(guarded(None), "none")
})

either = (v : Result(Int, String | Nil)) {
	v |>
		(Ok(n)) { show(n) }
		| (Error(_ : Nil)) { "nil" }
		| (Error(other)) { "error " <> other }
}

test("narrowing", () {
	check(either(Ok(1)), "1")
	check(either(Error(Nil)), "nil")
	check(either(Error("bad")), "error bad")
})

test("nested_case", () {
	v = [1, 2] |>
		([a, b]) { { a < b } |> (True) { "lt" } | (False) { "ge" } }
		| (_) { "no" }
	check(v, "lt")
})
