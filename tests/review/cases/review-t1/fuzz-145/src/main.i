f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "a", False)) { 0 }
		| (("ab", _)) { 1 }
		| (("a" <> _, False)) { 2 }
		| ((_, False)) { 3 }
		| ((_: { "a" | "b" }, False) ? False) { 4 }
		| ((_: String - "a", True)) { 5 }
		| ((_, False)) { 6 }
}
