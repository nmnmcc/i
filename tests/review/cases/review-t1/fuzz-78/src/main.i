f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", True)) { 0 }
		| (("b", _)) { 1 }
		| ((_: "a" | "b", True)) { 2 }
		| (("ab", False)) { 3 }
		| ((_, False) ? False) { 4 }
}
