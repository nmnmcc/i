f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", False)) { 0 }
		| ((_, _)) { 1 }
		| (("ab", False)) { 2 }
		| (("ab", False)) { 3 }
		| ((_, True)) { 4 }
		| (("b", True)) { 5 }
}

main = () {
	assert(f(("a", True)) == 1)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 1)
	assert(f(("ab", False)) == 1)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 1)
	assert(f(("c", False)) == 1)
	assert(f(("", True)) == 1)
	assert(f(("", False)) == 1)
	Nil
}
