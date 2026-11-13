f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, _)) { 0 }
		| ((_, _)) { 1 }
		| (("b", True)) { 2 }
		| (("a", False)) { 3 }
		| (("ab", False)) { 4 }
		| ((_: "a" | "b", True)) { 5 }
		| (("a", True)) { 6 }
		| (("a", True)) { 7 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 1)
	assert(f(("c", False)) == 1)
	assert(f(("", True)) == 1)
	assert(f(("", False)) == 1)
	Nil
}
