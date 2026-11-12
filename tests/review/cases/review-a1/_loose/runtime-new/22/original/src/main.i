g = (p: (Int, Int)) { p |> ((a, b)) { (x) -> Int { a + b + x } } }
f = () { g((20, 21))(1) }

main = () {
	assert(f() == 42)
	Nil
}
