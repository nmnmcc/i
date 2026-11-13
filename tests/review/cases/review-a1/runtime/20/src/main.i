g = (p: (Int, Int)) { p |> ((a, b) = t) { (x) { a + b + x } } }

f = () { g((20, 21))(1) }

main = () {
	assert(f() == 42)
	Nil
}
