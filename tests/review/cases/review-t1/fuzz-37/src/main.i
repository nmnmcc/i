f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", _)) { 0 }
		| (("a", True)) { 1 }
		| ((_: String - "a", _)) { 2 }
		| (("a", _)) { 3 }
}

main = () {
	assert(f(("a", True)) == 1)
	assert(f(("a", False)) == 3)
	assert(f(("ab", True)) == 2)
	assert(f(("ab", False)) == 2)
	assert(f(("b", True)) == 0)
	assert(f(("b", False)) == 0)
	assert(f(("c", True)) == 2)
	assert(f(("c", False)) == 2)
	assert(f(("", True)) == 2)
	assert(f(("", False)) == 2)
	Nil
}
