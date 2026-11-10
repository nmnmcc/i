f = () {
	{
		id(A) = (x: A) -> A { x }
		id(42)
	}
}

@main
main = () {
	assert(f() == 42)
	Nil
}
