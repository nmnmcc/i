f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, True) ? False) { 0 }
		| (("ab", False)) { 1 }
		| (("a" <> _, _)) { 2 }
		| (("b", True)) { 3 }
}
