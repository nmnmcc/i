f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", False)) { 0 }
		| ((_: String - "a", False)) { 1 }
		| (("a", False) ? False) { 2 }
		| (("a", _)) { 3 }
		| (("b", _)) { 4 }
		| (("a", _)) { 5 }
		| (("ab", True)) { 6 }
		| ((_: String - "ab" - "b", _)) { 7 }
}

main = () {
	assert(f(("a", True)) == 3)
	assert(f(("a", False)) == 3)
	assert(f(("ab", True)) == 6)
	assert(f(("ab", False)) == 1)
	assert(f(("b", True)) == 4)
	assert(f(("b", False)) == 0)
	assert(f(("c", True)) == 7)
	assert(f(("c", False)) == 1)
	assert(f(("", True)) == 7)
	assert(f(("", False)) == 1)
	Nil
}
