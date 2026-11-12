f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, True)) { 0 }
		| ((_: String - "ab" - "b", False)) { 1 }
		| (("a", _)) { 2 }
		| (("a" <> _, False)) { 3 }
		| (("ab", _)) { 4 }
		| ((_: String - "ab" - "b", True)) { 5 }
		| (("ab", False)) { 6 }
		| (("ab", False) ? False) { 7 }
		| (("a", _)) { 8 }
}
