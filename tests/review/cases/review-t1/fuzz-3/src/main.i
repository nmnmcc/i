f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "ab" - "b", True)) { 0 }
		| ((_: "a" | "b", True)) { 1 }
		| (("ab", False)) { 2 }
		| ((_: String - "ab" - "b", True)) { 3 }
		| ((_: String - "a", True)) { 4 }
		| ((_: "a" | "b", False)) { 5 }
		| ((_, False)) { 6 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 5)
	assert(f(("ab", True)) == 4)
	assert(f(("ab", False)) == 2)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 5)
	assert(f(("c", True)) == 0)
	assert(f(("c", False)) == 6)
	assert(f(("", True)) == 0)
	assert(f(("", False)) == 6)
	Nil
}
