f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: { "a" | "b" }, True)) { 0 }
		| (("ab", True) ? False) { 1 }
		| ((_: String - "ab" - "b", _)) { 2 }
		| ((_: String - "a", True)) { 3 }
		| (("b", _)) { 4 }
}
