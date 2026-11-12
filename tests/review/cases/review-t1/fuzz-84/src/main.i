f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, _)) { 0 }
		| (("a", False)) { 1 }
		| (("a", _)) { 2 }
		| (("a", True)) { 3 }
		| (("a" <> _, False)) { 4 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 0)
	assert(f(("b", False)) == 0)
	assert(f(("c", True)) == 0)
	assert(f(("c", False)) == 0)
	assert(f(("", True)) == 0)
	assert(f(("", False)) == 0)
	Nil
}
