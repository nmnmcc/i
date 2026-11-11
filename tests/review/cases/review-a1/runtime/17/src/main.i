f = () {
	{
		g = (x) -> Int { x + 1 }
		{ g(41) }
	}
}


main = () {
	assert(f() == 42)
	Nil
}
