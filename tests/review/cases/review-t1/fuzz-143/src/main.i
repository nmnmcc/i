f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "ab" - "b", True)) { 0 }
		| ((_: "a" | "b", _)) { 1 }
		| ((_, _) ? False) { 2 }
		| ((_: String - "a", _)) { 3 }
		| (("ab", _)) { 4 }
		| ((_: "a" | "b", _)) { 5 }
		| ((_: String - "a", True)) { 6 }
		| (("ab", True)) { 7 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 1)
	assert(f(("ab", True)) == 3)
	assert(f(("ab", False)) == 3)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 0)
	assert(f(("c", False)) == 3)
	assert(f(("", True)) == 0)
	assert(f(("", False)) == 3)
	Nil
}
