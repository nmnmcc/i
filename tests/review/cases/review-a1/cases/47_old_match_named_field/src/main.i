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
