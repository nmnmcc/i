f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "ab" - "b", False) ? False) { 0 }
		| ((_: String - "a", True)) { 1 }
		| ((_: String - "ab" - "b", True)) { 2 }
		| (("b", False)) { 3 }
		| (("b", False)) { 4 }
		| (("b", True)) { 5 }
		| (("b", True)) { 6 }
		| ((_: String - "ab" - "b", _)) { 7 }
		| ((_, _)) { 8 }
}

main = () {
	assert(f(("a", True)) == 2)
	assert(f(("a", False)) == 7)
	assert(f(("ab", True)) == 1)
	assert(f(("ab", False)) == 8)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 3)
	assert(f(("c", True)) == 1)
	assert(f(("c", False)) == 7)
	assert(f(("", True)) == 1)
	assert(f(("", False)) == 7)
	Nil
}
