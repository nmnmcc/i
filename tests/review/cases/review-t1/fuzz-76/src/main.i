f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, True)) { 0 }
		| ((_, _) ? False) { 1 }
		| ((_ : String - "ab" - "b", False)) { 2 }
		| (("ab", False) ? False) { 3 }
		| ((_ : String - "ab" - "b", _)) { 4 }
		| ((_, _)) { 5 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 2)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 5)
	assert(f(("b", True)) == 0)
	assert(f(("b", False)) == 5)
	assert(f(("c", True)) == 0)
	assert(f(("c", False)) == 2)
	assert(f(("", True)) == 0)
	assert(f(("", False)) == 2)
	Nil
}
