f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "ab" - "b", False)) { 0 }
		| (("ab", True)) { 1 }
		| ((_: String - "ab" - "b", _)) { 2 }
		| ((_: String - "ab" - "b", False)) { 3 }
		| (("a" <> _, _)) { 4 }
		| ((_, True)) { 5 }
}
