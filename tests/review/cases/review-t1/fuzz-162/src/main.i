f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "a", False)) { 0 }
		| ((_, False)) { 1 }
		| (("a" <> _, False)) { 2 }
		| ((_ : String - "a", False)) { 3 }
		| ((_ : { "a" | "b" }, _)) { 4 }
		| ((_ : String - "a", _)) { 5 }
}

main = () {
	assert(f(("a", True)) == 4)
	assert(f(("a", False)) == 1)
	assert(f(("ab", True)) == 5)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 4)
	assert(f(("b", False)) == 0)
	assert(f(("c", True)) == 5)
	assert(f(("c", False)) == 0)
	assert(f(("", True)) == 5)
	assert(f(("", False)) == 0)
	Nil
}
