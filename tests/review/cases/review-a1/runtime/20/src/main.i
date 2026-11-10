g = (p: (Int, Int)) {
	p |> {
		t = (a, b) -> { (x) { a + b + x } }
	}
}

f = () { g((20, 21))(1) }

@main
main = () {
	assert(f() == 42)
	Nil
}
