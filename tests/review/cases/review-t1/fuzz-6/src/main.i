f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, True) ? False) { 0 }
		| (("a", True)) { 1 }
		| ((_: String - "ab" - "b", True)) { 2 }
		| (("b", _)) { 3 }
		| ((_, True)) { 4 }
		| ((_: { "a" | "b" }, True) ? False) { 5 }
}
