f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : { "a" | "b" }, False)) { 0 }
		| (("ab", False)) { 1 }
		| (("b", True)) { 2 }
}
