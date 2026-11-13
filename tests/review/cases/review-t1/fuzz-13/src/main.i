f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "a", True)) { 0 }
		| ((_: String - "a", True)) { 1 }
		| (("ab", True)) { 2 }
		| (("b", False)) { 3 }
		| (("a", _) ? False) { 4 }
		| ((_: String - "a", True)) { 5 }
		| ((_: "a" | "b", True)) { 6 }
		| (("b", False)) { 7 }
		| (("b", False)) { 8 }
}
