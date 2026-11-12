// §4.8: the left side of `|>` is the first argument of the call, so it has
// the expected type of the first parameter. A function expression with a
// parameter with no written type is typed last, with the expected type that
// the other arguments give.
User {
	User(name: String, age: Int)
}

apply(A) = (f: (User) -> A, x: User) -> A { f(x) }

map(A, B) = (xs: List(A), f: (A) -> B) -> List(B) {
	xs |> {
		[]          -> []
		[x, ..rest] -> [f(x), ..map(rest, f)]
	}
}

flip(A, B) = (f: (A) -> B, xs: List(A)) -> List(B) { map(xs, f) }

u = User(name: "bob", age: 3)

left_fn = (u) { u.name } |> apply(u)

left_fn_later = (u) { u.age } |> flip([User(name: "ann", age: 4)])

// A tuple on the left: its parts have the parts of the expected type.
pair = (p: ((User) -> Int, Int)) { p }

left_tuple = ((u) { u.age }, 1) |> pair

// With no known type for the callee, there is no expected type.
unknown = (g) { (u) { u.name } |> g }
