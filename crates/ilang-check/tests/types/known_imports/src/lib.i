(
	User(..)
	x
	y
	mk
	xs
)

User {
	User(name: String, age: Int)
}

// Not a direct value, so not generalized: its value has no known type, but
// its type scheme is `User`.
x = { (a) { a } }(User(name: "bob", age: 3))

xs = { (a) { a } }([User(name: "ann", age: 4)])

// A direct value: generalized.
y = User(name: "ann", age: 4)

mk = (a) { a }
