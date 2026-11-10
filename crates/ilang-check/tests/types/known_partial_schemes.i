// §4.8: a generalized local binding whose type scheme has a type variable
// that is not generalized (from a name outside it): that variable is
// replaced by the matching part of the known type of the value.
User {
	User(name: String, age: Int)
}

map(A, B) = (xs: List(A), f: (A) -> B) -> List(B) {
	xs |> {
		[]          -> []
		[x, ..rest] -> [f(x), ..map(rest, f)]
	}
}

users = [User("bob", 3)]

name = map(users, (p) {
	q = p
	q.name
})

part = map(users, (p) {
	q = (p, 1)
	q.0.name
})

thunk = map(users, (p) {
	f = () { p }
	f().age
})

tuple_pattern = map(users, (p) {
	(a, b) = (p, 2)
	a.name
})

// With nothing to match, the part is `_`: `.name` needs a known type.
unknown = (p) {
	q = p
	q.name
}
