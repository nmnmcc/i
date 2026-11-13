f = () { { (x) -> Int { x + 1 }(41) } }

main = () {
	assert(f() == 42)
	Nil
}
