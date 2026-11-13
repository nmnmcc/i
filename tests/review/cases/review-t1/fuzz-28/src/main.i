f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: { "a" | "b" }, _)) { 0 }
		| (("a", False)) { 1 }
		| (("ab", False)) { 2 }
		| ((_: String - "ab" - "b", False)) { 3 }
		| (("ab", _)) { 4 }
}
