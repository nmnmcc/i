f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, False)) { 0 }
		| ((_ : String - "ab" - "b", _)) { 1 }
		| (("a", True)) { 2 }
		| (("a", True)) { 3 }
		| ((_ : String - "a", _)) { 4 }
		| (("a" <> _, False)) { 5 }
		| (("ab", True)) { 6 }
		| (("a" <> _, _)) { 7 }
		| ((_, True)) { 8 }
}

main = () {
	assert(f(("a", True)) == 1)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 4)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 4)
	assert(f(("b", False)) == 4)
	assert(f(("c", True)) == 1)
	assert(f(("c", False)) == 1)
	assert(f(("", True)) == 1)
	assert(f(("", False)) == 1)
	Nil
}
