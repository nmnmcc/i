f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", True)) { 0 }
		| ((_: String - "a", False)) { 1 }
		| (("b", _)) { 2 }
		| (("a", False)) { 3 }
		| (("a" <> _, False)) { 4 }
		| ((_: String - "a", True)) { 5 }
		| ((_: String - "a", True) ? False) { 6 }
		| ((_: "a" | "b", True)) { 7 }
		| ((_: "a" | "b", False)) { 8 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 3)
	assert(f(("ab", True)) == 5)
	assert(f(("ab", False)) == 1)
	assert(f(("b", True)) == 2)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 5)
	assert(f(("c", False)) == 1)
	assert(f(("", True)) == 5)
	assert(f(("", False)) == 1)
	Nil
}
