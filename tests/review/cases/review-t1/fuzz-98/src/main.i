f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: { "a" | "b" }, False)) { 0 }
		| ((_: String - "a", False) ? False) { 1 }
		| (("ab", True) ? False) { 2 }
		| ((_, False)) { 3 }
		| ((_: String - "ab" - "b", True)) { 4 }
		| (("ab", False)) { 5 }
}
