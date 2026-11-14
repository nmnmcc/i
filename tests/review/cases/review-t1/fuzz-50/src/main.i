f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, True)) { 0 }
		| ((_ : String - "a", False)) { 1 }
		| ((_, _)) { 2 }
}

main = () {
	assert(f(("a", True)) == 0)
	assert(f(("a", False)) == 2)
	assert(f(("ab", True)) == 0)
	assert(f(("ab", False)) == 1)
	assert(f(("b", True)) == 2)
	assert(f(("b", False)) == 1)
	assert(f(("c", True)) == 2)
	assert(f(("c", False)) == 1)
	assert(f(("", True)) == 2)
	assert(f(("", False)) == 1)
	Nil
}
