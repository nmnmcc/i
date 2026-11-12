f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", _)) { 0 }
		| (("a" <> _, False)) { 1 }
		| (("a" <> _, True)) { 2 }
		| (("a", False)) { 3 }
		| ((_, _)) { 4 }
		| ((_, False)) { 5 }
		| ((_: String - "ab" - "b", True) ? False) { 6 }
		| ((_: "a" | "b", _)) { 7 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 2)
	assert(f(("ab", False)) == 1)
	assert(f(("b", True)) == 4)
	assert(f(("b", False)) == 4)
	assert(f(("c", True)) == 4)
	assert(f(("c", False)) == 4)
	assert(f(("", True)) == 4)
	assert(f(("", False)) == 4)
	Nil
}
