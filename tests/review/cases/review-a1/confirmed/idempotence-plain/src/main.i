f = () {
	{
		g = (x) -> Int { x }
		{
			g(42) |> {
				_ -> 42
			}
		}
	} * 10
}

@main
main = () {
	assert(f() == 420)
	Nil
}
