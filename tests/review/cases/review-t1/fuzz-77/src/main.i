f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "a", False)) { 0 }
		| (("b", True) ? False) { 1 }
		| ((_: String - "ab" - "b", False) ? False) { 2 }
		| ((_, True)) { 3 }
		| ((_: String - "a", _)) { 4 }
		| (("ab", True)) { 5 }
}
