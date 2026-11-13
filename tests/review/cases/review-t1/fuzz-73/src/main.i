f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: "a" | "b", False)) { 0 }
		| (("a" <> _, False)) { 1 }
		| ((_, True)) { 2 }
		| (("a", _)) { 3 }
		| (("a" <> _, False) ? False) { 4 }
}
