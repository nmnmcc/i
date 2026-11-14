f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", False)) { 0 }
		| (("b", True)) { 1 }
		| ((_ : { "a" | "b" }, False)) { 2 }
}
