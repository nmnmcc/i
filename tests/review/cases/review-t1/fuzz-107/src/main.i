f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", True)) { 0 }
		| (("b", True)) { 1 }
		| ((_: String - "a", _)) { 2 }
}
