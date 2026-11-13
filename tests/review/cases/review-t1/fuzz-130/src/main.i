f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("ab", True) ? False) { 0 }
		| (("ab", True) ? False) { 1 }
		| ((_: { "a" | "b" }, True)) { 2 }
		| (("ab", False)) { 3 }
		| (("a" <> _, False)) { 4 }
		| (("a", _)) { 5 }
		| ((_: { "a" | "b" }, _)) { 6 }
		| ((_: String - "a", _)) { 7 }
}

main = () {
	assert(f(("a", True)) == 2)
	assert(f(("a", False)) == 4)
	assert(f(("ab", True)) == 7)
	assert(f(("ab", False)) == 3)
	assert(f(("b", True)) == 2)
	assert(f(("b", False)) == 6)
	assert(f(("c", True)) == 7)
	assert(f(("c", False)) == 7)
	assert(f(("", True)) == 7)
	assert(f(("", False)) == 7)
	Nil
}
