f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", False) ? False) { 0 }
		| ((_: { "a" | "b" }, True)) { 1 }
		| (("a" <> _, True)) { 2 }
		| (("a", True)) { 3 }
		| ((_, _)) { 4 }
		| (("ab", _)) { 5 }
}

main = () {
	assert(f(("a", True)) == 1)
	assert(f(("a", False)) == 4)
	assert(f(("ab", True)) == 2)
	assert(f(("ab", False)) == 4)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 4)
	assert(f(("c", True)) == 4)
	assert(f(("c", False)) == 4)
	assert(f(("", True)) == 4)
	assert(f(("", False)) == 4)
	Nil
}
