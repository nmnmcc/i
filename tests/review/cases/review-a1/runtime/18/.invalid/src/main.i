// old-syntax-check: E0101
f: () -> (Int) -> Int = () { { (x) -> Int { x + 1 } } }

main = () {
	assert(f()(1) == 2)
	Nil
}
