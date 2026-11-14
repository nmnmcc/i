// spec: 4.8, 6.12
// expect: pass
(
	main
)
User = (name = String)
Some(A) = (A)
None = ()
Option(A) = Some(A) | None
get = (opt : Option(User)) { opt |> (Some(u)) { u.name } }
single : (Int) <- (Assert) -> Int = (0) { 7 }

main = () {
	assert(get(Some(User(name = "Ada"))) == "Ada")
	assert(
		{
			(u) { u.name } |> (f : (User) -> String) { f(User(name = "Bob")) }
		} ==
			"Bob",
	)
	assert(single(0) == 7)
	Nil
}
