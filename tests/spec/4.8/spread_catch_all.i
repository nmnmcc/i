// spec: 4.8, 7.1
// expect: pass

// Rule 11: a constructor pattern with `..` counts as having a catch-all at
// the places of the fields that it leaves out. So the guard on the first
// clause does not leave `age` and `name` with no values.

(
	testlib = (test) = std/test
)

User {
	User(name: String, age: Int)
}

kind = (u: User) {
	u |> {
		User(age, ..) ? age > 10 -> 1
		User(..)                 -> 2
	}
}

label = (u: User) {
	u |> {
		User(age, name) ? age > 10 -> name
		User(name, ..)             -> "young " <> name
	}
}

test("spread_is_a_catch_all", () {
	(kind(User("a", 20)), kind(User("b", 5))) |> {
		(1, 2) -> Nil
		_      -> panic("kind")
	}
	(label(User("a", 20)), label(User("b", 5))) |> {
		("a", "young b") -> Nil
		_                -> panic("label")
	}
})
