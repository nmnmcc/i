f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, True)) { 0 }
		| (("b", False)) { 1 }
		| (("ab", True)) { 2 }
		| ((_, _)) { 3 }
		| ((_ : String - "a", True)) { 4 }
		| (("b", _)) { 5 }
		| ((_, _)) { 6 }
		| ((_ : { "a" | "b" }, True)) { 7 }
		| ((_ : String - "ab" - "b", False) ? False) { 8 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 3)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 3)
	assert(f(("b", True)) == 0)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 0)
	assert(f(("c", False)) == 3)
	assert(f(("", True)) == 0)
	assert(f(("", False)) == 3)
	Nil
}
