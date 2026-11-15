f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "a", _)) { 0 }
		| (("a" <> _, _) ? False) { 1 }
		| ((_ : String - "a", False)) { 2 }
		| ((_, False)) { 3 }
		| ((_ : { "a" | "b" }, True)) { 4 }
		| ((_ : { "a" | "b" }, False)) { 5 }
		| (("ab", _)) { 6 }
		| ((_ : { "a" | "b" }, True)) { 7 }
		| ((_ : String - "ab" - "b", True)) { 8 }
}

main = () {
	assert(f(("a", True)) == 4)
	assert(f(("a", False)) == 3)
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
