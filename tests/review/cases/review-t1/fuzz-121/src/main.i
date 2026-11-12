f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "ab" - "b", True)) { 0 }
		| ((_: String - "ab" - "b", _) ? False) { 1 }
		| (("b", True)) { 2 }
		| ((_, False)) { 3 }
}
