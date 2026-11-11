f = (n: Int) {
	n |> {
		0 | 1 ? True -> 42
		_            -> 7
	}
}


main = () {
	assert(f(0) == 42 && f(1) == 42 && f(2) == 7)
	Nil
}
