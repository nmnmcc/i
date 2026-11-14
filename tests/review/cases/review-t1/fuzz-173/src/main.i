f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, _)) { 0 }
		| (("b", _)) { 1 }
		| ((_ : String - "ab" - "b", _)) { 2 }
		| (("a", False)) { 3 }
		| ((_ : String - "a", False)) { 4 }
		| (("a" <> _, _)) { 5 }
		| (("ab", False)) { 6 }
		| ((_, False)) { 7 }
		| ((_ : String - "a", False)) { 8 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 2)
	assert(f(("c", False)) == 2)
	assert(f(("", True)) == 2)
	assert(f(("", False)) == 2)
	Nil
}
