f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, False)) { 0 }
		| (("a", True)) { 1 }
		| (("ab", _) ? False) { 2 }
		| (("a", _) ? False) { 3 }
		| ((_: { "a" | "b" }, True)) { 4 }
		| (("b", True) ? False) { 5 }
		| (("a", _) ? False) { 6 }
		| ((_, False)) { 7 }
}
