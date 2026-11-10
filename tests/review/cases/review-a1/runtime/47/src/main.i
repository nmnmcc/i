User {
	User(name: String)
}

f = (n: Int) {
	{
		n |> {
			0 -> User("yes")
			_ -> User("no")
		}
	}.name
}

@main
main = () {
	assert(f(0) == "yes" && f(1) == "no")
	Nil
}
