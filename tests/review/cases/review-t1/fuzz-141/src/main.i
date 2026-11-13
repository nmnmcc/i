f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, True) ? False) { 0 }
		| ((_: String - "a", True)) { 1 }
		| ((_, _)) { 2 }
		| (("a" <> _, False)) { 3 }
}

main = () {
	assert(f(("a", True)) == 2)
	assert(f(("a", False)) == 2)
	assert(f(("ab", True)) == 1)
	assert(f(("ab", False)) == 2)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 2)
	assert(f(("c", True)) == 1)
	assert(f(("c", False)) == 2)
	assert(f(("", True)) == 1)
	assert(f(("", False)) == 2)
	Nil
}
