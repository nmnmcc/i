f = () { (20, 22) |> ((a, b) = t) { a + b } }

main = () {
	assert(f() == 42)
	Nil
}
