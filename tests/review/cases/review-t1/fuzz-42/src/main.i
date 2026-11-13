f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", _)) { 0 }
		| ((_: String - "a", True)) { 1 }
		| ((_: String - "ab" - "b", _)) { 2 }
		| ((_: String - "ab" - "b", False)) { 3 }
		| (("a", True)) { 4 }
}
