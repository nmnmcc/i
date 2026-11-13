f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("ab", _)) { 0 }
		| (("ab", True) ? False) { 1 }
		| ((_: { "a" | "b" }, False) ? False) { 2 }
		| ((_: { "a" | "b" }, False)) { 3 }
		| (("b", _)) { 4 }
		| ((_: String - "ab" - "b", True)) { 5 }
		| (("a" <> _, False)) { 6 }
}
