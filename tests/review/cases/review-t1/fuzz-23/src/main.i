f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "ab" - "b", True)) { 0 }
		| (("a", True)) { 1 }
		| (("a" <> _, True)) { 2 }
		| (("a", _)) { 3 }
		| (("a", _)) { 4 }
		| ((_, True)) { 5 }
		| ((_ : { "a" | "b" }, False)) { 6 }
		| ((_ : String - "a", _)) { 7 }
		| (("a", False)) { 8 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 3)
	assert(f(("ab", True)) == 2)
	assert(f(("ab", False)) == 7)
	assert(f(("b", True)) == 5)
	assert(f(("b", False)) == 6)
	assert(f(("c", True)) == 0)
	assert(f(("c", False)) == 7)
	assert(f(("", True)) == 0)
	assert(f(("", False)) == 7)
	Nil
}
