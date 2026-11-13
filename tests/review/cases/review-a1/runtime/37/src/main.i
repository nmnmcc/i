f = (v: Int) { v |> (x: Int) { x } }

main = () {
	assert(f(42) == 42)
	Nil
}
