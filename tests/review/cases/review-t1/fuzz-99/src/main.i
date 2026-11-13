f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", False)) { 0 }
		| ((_: String - "ab" - "b", True)) { 1 }
		| ((_: String - "ab" - "b", False)) { 2 }
		| (("a" <> _, True)) { 3 }
		| (("ab", True)) { 4 }
		| ((_: String - "a", False)) { 5 }
}
