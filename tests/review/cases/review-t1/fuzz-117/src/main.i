f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "ab" - "b", _)) { 0 }
		| ((_: String - "ab" - "b", True)) { 1 }
		| ((_: String - "a", _)) { 2 }
		| (("b", _)) { 3 }
		| (("b", True)) { 4 }
		| ((_: { "a" | "b" }, False)) { 5 }
		| ((_: { "a" | "b" }, False)) { 6 }
		| (("ab", False)) { 7 }
		| ((_: String - "a", _)) { 8 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 2)
	assert(f(("ab", False)) == 2)
	assert(f(("b", True)) == 2)
	assert(f(("b", False)) == 2)
	assert(f(("c", True)) == 0)
	assert(f(("c", False)) == 0)
	assert(f(("", True)) == 0)
	assert(f(("", False)) == 0)
	Nil
}
