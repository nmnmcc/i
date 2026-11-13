f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", _)) { 0 }
		| (("b", False) ? False) { 1 }
		| ((_: String - "ab" - "b", False) ? False) { 2 }
		| (("a" <> _, False)) { 3 }
		| ((_: String - "ab" - "b", False)) { 4 }
		| ((_: { "a" | "b" }, True)) { 5 }
		| ((_: { "a" | "b" }, _)) { 6 }
		| (("a", False)) { 7 }
}
