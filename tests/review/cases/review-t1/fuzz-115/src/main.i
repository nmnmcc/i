f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", True)) { 0 }
		| (("ab", True)) { 1 }
		| ((_: String - "a", _)) { 2 }
		| (("a" <> _, False)) { 3 }
		| ((_, False) ? False) { 4 }
		| ((_: { "a" | "b" }, _) ? False) { 5 }
		| ((_: String - "ab" - "b", False)) { 6 }
		| (("b", True)) { 7 }
}
