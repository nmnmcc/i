f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, False)) { 0 }
		| ((_: String - "ab" - "b", False)) { 1 }
		| (("a" <> _, False)) { 2 }
		| (("b", _) ? False) { 3 }
		| (("ab", False)) { 4 }
}
