f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", True)) { 0 }
		| ((_ : { "a" | "b" }, _) ? False) { 1 }
		| (("b", False)) { 2 }
		| ((_, True)) { 3 }
}
