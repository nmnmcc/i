f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("ab", _)) { 0 }
		| ((_, False)) { 1 }
		| ((_ : { "a" | "b" }, False)) { 2 }
		| (("ab", False)) { 3 }
		| ((_, False)) { 4 }
		| (("a" <> _, _)) { 5 }
		| (("ab", True)) { 6 }
		| ((_, True)) { 7 }
		| (("b", True)) { 8 }
}

main = () {
	assert(f(("a", True)) == 5)
	assert(f(("a", False)) == 1)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 7)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 7)
	assert(f(("c", False)) == 1)
	assert(f(("", True)) == 7)
	assert(f(("", False)) == 1)
	Nil
}
