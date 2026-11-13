f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "a", False)) { 0 }
		| ((_, False) ? False) { 1 }
		| (("a" <> _, _)) { 2 }
		| (("ab", True)) { 3 }
		| ((_: { "a" | "b" }, True)) { 4 }
		| (("a" <> _, False) ? False) { 5 }
		| ((_: String - "a", True)) { 6 }
		| ((_: { "a" | "b" }, True)) { 7 }
}

main = () {
	assert(f(("a", True)) == 2)
	assert(f(("a", False)) == 2)
	assert(f(("ab", True)) == 2)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 4)
	assert(f(("b", False)) == 0)
	assert(f(("c", True)) == 6)
	assert(f(("c", False)) == 0)
	assert(f(("", True)) == 6)
	assert(f(("", False)) == 0)
	Nil
}
