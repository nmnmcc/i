f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("ab", False)) { 0 }
		| ((_ : String - "ab" - "b", False)) { 1 }
		| ((_ : String - "ab" - "b", True)) { 2 }
		| (("ab", True) ? False) { 3 }
		| (("a", _)) { 4 }
		| ((_, True)) { 5 }
		| ((_ : String - "a", _)) { 6 }
}

main = () {
	assert(f(("a", True)) == 2)
	assert(f(("a", False)) == 1)
	assert(f(("ab", True)) == 5)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 5)
	assert(f(("b", False)) == 6)
	assert(f(("c", True)) == 2)
	assert(f(("c", False)) == 1)
	assert(f(("", True)) == 2)
	assert(f(("", False)) == 1)
	Nil
}
