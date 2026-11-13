f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, _) ? False) { 0 }
		| ((_ : String - "a", _)) { 1 }
		| ((_ : String - "ab" - "b", False)) { 2 }
		| ((_ : String - "ab" - "b", False)) { 3 }
		| ((_ : { "a" | "b" }, False)) { 4 }
		| ((_ : String - "a", True) ? False) { 5 }
		| ((_, _)) { 6 }
		| (("b", _)) { 7 }
		| ((_, False)) { 8 }
}

main = () {
	assert(f(("a", True)) == 6)
	assert(f(("a", False)) == 2)
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
