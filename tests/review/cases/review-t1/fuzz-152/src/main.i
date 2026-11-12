f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "ab" - "b", False)) { 0 }
		| (("a", _)) { 1 }
		| (("a", False)) { 2 }
}
