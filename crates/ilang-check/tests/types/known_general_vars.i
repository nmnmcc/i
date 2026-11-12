// §4.8: a type variable that is generalized in a known type `K` stands for
// any type, so it matches nothing. `map(id)` gives `b` nothing, so the call
// has no known type, and `.age` needs one.
User {
	User(name: String, age: Int)
}

map(A, B) = (xs: List(A), f: (A) -> B) -> List(B) {
	xs |> {
		[]          -> []
		[x, ..rest] -> [f(x), ..map(rest, f)]
	}
}

id = (x) { x }

users = [User(name: "bob", age: 3)]

through_id = users |> map(id) |> map((v) { v.age })

// The variable of the constructor `Box` matches nothing, but `b` gets
// `Box(C)` as a whole, so `.item` is known.
Box(A) {
	Box(item: A)
}

boxed = users |> map(Box) |> map((b) { b.item })

// A written type works it out.
written = users |> map(id) |> map((v: User) { v.age })
