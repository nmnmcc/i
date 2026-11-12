f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "ab" - "b", True)) { 0 }
		| ((_, True)) { 1 }
		| ((_, True)) { 2 }
		| ((_: "a" | "b", _)) { 3 }
		| (("a", _)) { 4 }
		| (("a", False)) { 5 }
		| ((_, True)) { 6 }
		| (("a" <> _, True)) { 7 }
		| ((_: String - "a", True)) { 8 }
}
