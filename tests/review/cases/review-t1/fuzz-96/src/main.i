f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "ab" - "b", _)) { 0 }
		| (("a" <> _, False)) { 1 }
		| ((_ : String - "ab" - "b", _)) { 2 }
		| (("ab", False)) { 3 }
		| ((_, _)) { 4 }
		| (("b", _)) { 5 }
		| ((_ : { "a" | "b" }, True)) { 6 }
		| (("ab", False)) { 7 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 4)
	assert(f(("ab", False)) == 1)
	assert(f(("b", True)) == 4)
	assert(f(("b", False)) == 4)
	assert(f(("c", True)) == 0)
	assert(f(("c", False)) == 0)
	assert(f(("", True)) == 0)
	assert(f(("", False)) == 0)
	Nil
}
