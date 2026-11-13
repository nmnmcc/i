f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", True)) { 0 }
		| ((_: String - "ab" - "b", True)) { 1 }
		| ((_: { "a" | "b" }, False)) { 2 }
}
