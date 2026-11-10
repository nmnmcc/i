f = (v: Int) {
	v |> {
		x: Int -> x
	}
}

@main
main = () {
	assert(f(42) == 42)
	Nil
}
