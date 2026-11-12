User {
	User(name: String)
}

f = (n: Int) {
	{
		n |> {
			0 -> User(name: "yes")
			_ -> User(name: "no")
		}
	}.name
}
