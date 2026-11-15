f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("ab", False)) { 0 }
		| ((_ : String - "a", True)) { 1 }
		| ((_ : String - "a", True) ? False) { 2 }
		| (("ab", True)) { 3 }
		| ((_, _)) { 4 }
		| ((_ : String - "ab" - "b", _)) { 5 }
}

main = () {
	assert(f(("a", True)) == 4)
	assert(f(("a", False)) == 4)
	assert(f(("ab", True)) == 1)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 4)
	assert(f(("c", True)) == 1)
	assert(f(("c", False)) == 4)
	assert(f(("", True)) == 1)
	assert(f(("", False)) == 4)
	Nil
}
