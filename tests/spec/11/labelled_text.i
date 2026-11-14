// spec: 11.8
// expect: pass
(
	i/core
)

User = (name = String, age = Int)
ScreenPoint = (x = Int, y = Int)
Mixed = (Int, z = String, a = Int)
Pair = (Int, String)
Empty = ()
Wrapper(A) = (z = Int, value = A)

main = () {
	assert(
		core.inspect(ScreenPoint(y = 4, x = 3)) == "ScreenPoint(x = 3, y = 4)",
	)
	assert(
		core.inspect(User(name = "ada", age = 36)) ==
			"User(age = 36, name = \"ada\")",
	)
	assert(
		core.inspect(Mixed(7, z = "last", a = 2)) ==
			"Mixed(7, a = 2, z = \"last\")",
	)
	assert(core.inspect(Pair(1, "two")) == "Pair(1, \"two\")")
	assert(core.inspect(Empty) == "Empty")
	assert(
		core.inspect(Wrapper(z = 9, value = User(name = "ada", age = 36))) ==
			"Wrapper(value = User(age = 36, name = \"ada\"), z = 9)",
	)
	assert(
		core.inspect(AssertionFailure(message = "bad")) ==
			"AssertionFailure(message = \"bad\")",
	)
	assert(
		core.inspect(Assert(fail = (_) { panic("unused") })) ==
			"Assert(fail = //fn(a) { ... })",
	)
	// Display ordering does not change the values under the written keys.
	mixed = Mixed(7, z = "last", a = 2)
	mixed |> (Mixed(n, a = a, z = z)) { assert((n, a, z) == (7, 2, "last")) }
}
