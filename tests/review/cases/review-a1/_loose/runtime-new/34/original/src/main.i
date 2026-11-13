f = (n: Int) { n |> (42) { 7 } }

main = () {
	assert(f(42) == 7)
	Nil
}
