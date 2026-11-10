// §4.6 and §6.6: labels put arguments in order; they have no effect on
// types.
User {
	User(name: String, age: Int)
}

make = (name: String, age: Int) { User(name, age) }

by_label = make(age: 3, name: "bob")

mixed = make("bob", age: 3)

ctor = User(age: 3, name: "bob")

annotated: (x: Int) -> Int = (y) { y }

call_annotated = annotated(x: 1)

// Labels have no effect on subtyping.
relabel: (z: Int) -> Int = annotated

local = () {
	f = (a: Int, b: String) { b }
	f(b: "x", a: 1)
}

// A pattern takes labelled fields by label.
age = (u) {
	User(age, ..) = u
	age
}

names = (u) {
	User(n = name, age) = u
	(n, name, age)
}

// Problems.
unknown = make(height: 1, name: "x")

twice = make("a", name: "b")

missing = make(name: "b")

param = (f) { f(x: 1) }

bad_pattern = (u) {
	User(n, a) = u
	n
}

// Labels come from known function types (§4.8).
// A name that is not generalized gets its labels from its value.
id = (x) { x }

through = () {
	g = id(make)
	g(age: 1, name: "x")
}

// A `_` part is fine: only the labels are needed.
holes = (f: (x: _) -> Int) { f(x: 1) }

// Positional arguments first, also with `|>`.
minus = (a: Int, b: Int) { a - b }

piped = 5 |> minus(b: 1)

// A function that uses its own result by label needs a written `->`.
Node {
	Node(value: Int, rest: Int)
}

walk = (n: Int) -> Node {
	n |> {
		0 -> Node(0, 0)
		_ -> Node(value: walk(n: n - 1).value + 1, rest: 0)
	}
}

walk_unwritten = (n: Int) {
	n |> {
		0 -> Node(0, 0)
		_ -> Node(walk_unwritten(n - 1).value + 1, 0)
	}
}

// In a group, the names are known from what is written.
ping = (n: Int) { pong(m: n) }

pong = (m: Int) {
	m |> {
		0 -> 0
		_ -> ping(n: m - 1)
	}
}

// An annotation label wins over the labels of the value.
wins: (x: Int) -> Int = (y) { y }

wins_ok = wins(x: 1)

wins_value = wins(y: 1)

// A function expression in a group, called directly.
direct = { (a: Int, b: String) { b } }(b: "x", a: 1)
