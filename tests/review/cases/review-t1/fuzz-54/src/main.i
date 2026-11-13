f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, False)) { 0 }
		| ((_ : { "a" | "b" }, _)) { 1 }
		| ((_ : String - "a", True)) { 2 }
}

main = () {
	assert(f(("a", True)) == 1)
	assert(f(("a", False)) == 0)
	assert(f(("ab", True)) == 2)
	assert(f(("ab", False)) == 0)
	assert(f(("b", True)) == 1)
	assert(f(("b", False)) == 0)
	assert(f(("c", True)) == 2)
	assert(f(("c", False)) == 0)
	assert(f(("", True)) == 2)
	assert(f(("", False)) == 0)
	Nil
}
