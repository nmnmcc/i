f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("ab", _)) { 0 }
		| ((_ : { "a" | "b" }, _)) { 1 }
		| (("a" <> _, False)) { 2 }
		| ((_ : String - "a", False)) { 3 }
		| (("ab", _)) { 4 }
		| ((_ : String - "a", _)) { 5 }
		| ((_, True)) { 6 }
		| ((_ : { "a" | "b" }, False)) { 7 }
		| (("a" <> _, True)) { 8 }
}

main = () {
	assert(f(("a", True)) == 1)
	assert(f(("a", False)) == 1)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 5)
	assert(f(("c", False)) == 3)
	assert(f(("", True)) == 5)
	assert(f(("", False)) == 3)
	Nil
}
