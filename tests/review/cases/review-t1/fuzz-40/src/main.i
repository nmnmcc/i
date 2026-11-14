f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, True)) { 0 }
		| (("b", False)) { 1 }
		| (("ab", True)) { 2 }
		| ((_ : String - "a", False)) { 3 }
		| ((_, _)) { 4 }
		| ((_, True)) { 5 }
		| (("a" <> _, _)) { 6 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 4)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 3)
	assert(f(("b", True)) == 4)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 4)
	assert(f(("c", False)) == 3)
	assert(f(("", True)) == 4)
	assert(f(("", False)) == 3)
	Nil
}
