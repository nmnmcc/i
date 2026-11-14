f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : { "a" | "b" }, True)) { 0 }
		| ((_ : String - "ab" - "b", False)) { 1 }
		| ((_ : String - "a", _)) { 2 }
		| (("ab", _)) { 3 }
		| ((_ : String - "ab" - "b", False)) { 4 }
		| (("b", _) ? False) { 5 }
		| (("a" <> _, _)) { 6 }
		| (("a" <> _, _)) { 7 }
		| (("b", _)) { 8 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 1)
	assert(f(("ab", True)) == 2)
	assert(f(("ab", False)) == 2)
	assert(f(("b", True)) == 0)
	assert(f(("b", False)) == 2)
	assert(f(("c", True)) == 2)
	assert(f(("c", False)) == 1)
	assert(f(("", True)) == 2)
	assert(f(("", False)) == 1)
	Nil
}
