f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, False)) { 0 }
		| ((_: String - "a", _)) { 1 }
		| ((_, False)) { 2 }
		| ((_: { "a" | "b" }, False)) { 3 }
		| ((_: { "a" | "b" }, _)) { 4 }
		| ((_: String - "ab" - "b", True)) { 5 }
}

main = () {
	assert(f(("a", True)) == 4)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 1)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 0)
	assert(f(("c", True)) == 1)
	assert(f(("c", False)) == 0)
	assert(f(("", True)) == 1)
	assert(f(("", False)) == 0)
	Nil
}
