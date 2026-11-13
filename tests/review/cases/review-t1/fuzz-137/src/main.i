f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", False) ? False) { 0 }
		| ((_: String - "ab" - "b", True) ? False) { 1 }
}
