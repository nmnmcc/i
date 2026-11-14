f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, _)) { 0 }
		| (("ab", True)) { 1 }
		| ((_, _)) { 2 }
		| (("ab", True)) { 3 }
		| ((_ : String - "ab" - "b", False)) { 4 }
		| (("a", False)) { 5 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 2)
	assert(f(("b", False)) == 2)
	assert(f(("c", True)) == 2)
	assert(f(("c", False)) == 2)
	assert(f(("", True)) == 2)
	assert(f(("", False)) == 2)
	Nil
}
