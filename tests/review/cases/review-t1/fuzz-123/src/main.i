f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: { "a" | "b" }, True)) { 0 }
		| (("a", _)) { 1 }
		| (("ab", True)) { 2 }
		| ((_: String - "ab" - "b", True)) { 3 }
		| (("ab", True) ? False) { 4 }
}
