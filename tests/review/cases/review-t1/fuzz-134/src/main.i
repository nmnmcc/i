f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, False)) { 0 }
		| ((_ : { "a" | "b" }, True)) { 1 }
		| (("a" <> _, _)) { 2 }
		| (("ab", True)) { 3 }
		| ((_, False)) { 4 }
		| ((_ : String - "ab" - "b", _)) { 5 }
		| ((_ : String - "ab" - "b", True)) { 6 }
		| (("ab", _)) { 7 }
}

main = () {
	assert(f(("a", True)) == 1)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 2)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 0)
	assert(f(("c", True)) == 5)
	assert(f(("c", False)) == 0)
	assert(f(("", True)) == 5)
	assert(f(("", False)) == 0)
	Nil
}
