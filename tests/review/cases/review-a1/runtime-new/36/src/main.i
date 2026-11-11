f = (v: Int | String) {
	v |> {
		x: Int -> x
		_      -> 0
	}
}


main = () {
	assert(f(42) == 42 && f("s") == 0)
	Nil
}
