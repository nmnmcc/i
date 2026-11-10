f = () {
	{
		go = (n) -> Int { n + 1 }
		go(41)
	}
}

@main
main = () {
	assert(f() == 42)
	Nil
}
