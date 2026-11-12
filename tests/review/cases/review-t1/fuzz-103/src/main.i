f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", _)) { 0 }
		| (("a", _)) { 1 }
		| ((_: String - "ab" - "b", False)) { 2 }
		| (("a", True)) { 3 }
		| (("a" <> _, True) ? False) { 4 }
}
