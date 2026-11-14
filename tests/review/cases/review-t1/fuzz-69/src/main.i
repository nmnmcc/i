f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, _) ? False) { 0 }
		| (("a", _) ? False) { 1 }
		| ((_ : String - "a", _)) { 2 }
		| (("a" <> _, True)) { 3 }
		| (("a" <> _, _)) { 4 }
		| ((_, _) ? False) { 5 }
}

main = () {
	assert(f(("a", True)) == 3)
	assert(f(("a", False)) == 4)
	assert(f(("ab", True)) == 2)
	assert(f(("ab", False)) == 2)
	assert(f(("b", True)) == 2)
	assert(f(("b", False)) == 2)
	assert(f(("c", True)) == 2)
	assert(f(("c", False)) == 2)
	assert(f(("", True)) == 2)
	assert(f(("", False)) == 2)
	Nil
}
