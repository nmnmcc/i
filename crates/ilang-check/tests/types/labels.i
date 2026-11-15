// §6.6: migrated function heads take positional arguments; constructor
// data keeps explicit labels (syntax three §2.3, §2.4).
User = (name = String, age = Int)

make = (name : String, age : Int) { User(name = name, age = age) }

by_label = make("bob", 3)

mixed = make("bob", 3)

ctor = User(age = 3, name = "bob")

annotated : (Int) -> Int = (y) { y }

call_annotated = annotated(1)

// The written positional function type is preserved by subtyping.
relabel : (Int) -> Int = annotated

local = () {
	f = (a : Int, b : String) { b }
	f(1, "x")
}

// A pattern takes labelled fields by label.
age = (u) {
	User(age = age, ..) = u
	age
}

names = (u) {
	User(name = n, age = age) = u
	(n, name, age)
}

// Problems.
unknown = make(height = 1, name = "x")

twice = make("a", name = "b")

missing = make("b")
// Calls through a labelled function value are valid (syntax three stage 10).
param = (f) { f(x = 1) }

bad_pattern = (u) {
	User(n, a) = u
	n
}

// Positional calls use known function types (§4.8).
// A name that is not generalized gets its type from its value.
id = (x) { x }

through = () {
	g = id(make)
	g("x", 1)
}

// A `_` part leaves the positional parameter type to inference.
holes = (f : (_) -> Int) { f(1) }

// Positional arguments first, also with `|>`.
minus = (a : Int, b : Int) { a - b }

piped = 5 |> minus(1)

// A function that uses its own result by label needs a written `->`.
Node = (value = Int, rest = Int)

walk = (n : Int) -> Node {
	n |>
		(0) { Node(value = 0, rest = 0) }
		| (_) { Node(value = walk(n - 1).value + 1, rest = 0) }
}

walk_unwritten = (n : Int) {
	n |>
		(0) { Node(value = 0, rest = 0) }
		| (_) { Node(value = walk_unwritten(n - 1).value + 1, rest = 0) }
}

// In a group, the names are known from what is written.
ping = (n : Int) { pong(n) }

pong = (m : Int) { m |> (0) { 0 } | (_) { ping(m - 1) } }

// A written positional signature determines the call interface (§6.6).
wins : (Int) -> Int = (y) { y }

wins_ok = wins(1)

wins_value = wins(1)

// A function expression in a group, called directly.
direct = { (a : Int, b : String) { b } }(1, "x")
