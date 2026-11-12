f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, True)) { 0 }
		| ((_: String - "a", _)) { 1 }
		| ((_: String - "ab" - "b", False)) { 2 }
		| ((_: String - "a", True)) { 3 }
		| ((_: "a" | "b", _)) { 4 }
		| ((_, False)) { 5 }
		| ((_: "a" | "b", False)) { 6 }
		| (("a", _)) { 7 }
		| ((_, False)) { 8 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 2)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 1)
	assert(f(("b", True)) == 0)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 0)
	assert(f(("c", False)) == 1)
	assert(f(("", True)) == 0)
	assert(f(("", False)) == 1)
	Nil
}
