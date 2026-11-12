f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: "a" | "b", _)) { 0 }
		| ((_: String - "ab" - "b", _)) { 1 }
		| ((_: "a" | "b", False)) { 2 }
		| (("b", _)) { 3 }
		| ((_: String - "ab" - "b", False)) { 4 }
		| ((_: String - "a", True) ? False) { 5 }
		| (("a", _)) { 6 }
		| (("a", True)) { 7 }
		| ((_, False)) { 8 }
}
