f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "a", True)) { 0 }
		| ((_: String - "a", False)) { 1 }
		| (("ab", False)) { 2 }
		| ((_, _)) { 3 }
		| ((_, True)) { 4 }
		| ((_: String - "a", _)) { 5 }
		| (("a" <> _, _)) { 6 }
		| (("b", False)) { 7 }
}

main = () {
	assert(f(("a", True)) == 3)
	assert(f(("a", False)) == 3)
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
