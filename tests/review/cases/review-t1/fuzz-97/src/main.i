f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("ab", _)) { 0 }
		| (("ab", True)) { 1 }
		| ((_: String - "a", True) ? False) { 2 }
		| ((_, False) ? False) { 3 }
		| ((_: String - "a", False)) { 4 }
}
