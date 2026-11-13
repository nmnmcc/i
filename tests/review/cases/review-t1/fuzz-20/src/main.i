f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, False)) { 0 }
		| (("ab", _)) { 1 }
		| ((_, True)) { 2 }
		| (("a" <> _, True)) { 3 }
		| (("ab", _)) { 4 }
		| ((_, False) ? False) { 5 }
		| ((_: String - "a", _)) { 6 }
}

main = () {
	assert(f(("a", True)) == 2)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 1)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 2)
	assert(f(("b", False)) == 6)
	assert(f(("c", True)) == 2)
	assert(f(("c", False)) == 6)
	assert(f(("", True)) == 2)
	assert(f(("", False)) == 6)
	Nil
}
