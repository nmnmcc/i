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
x = { (a) { a } }(User("bob", 3))

xs = { (a) { a } }([User("ann", 4)])

// A direct value: generalized.
y = User("ann", 4)

mk = (a) { a }
