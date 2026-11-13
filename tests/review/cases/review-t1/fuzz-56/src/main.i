f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: "a" | "b", False)) { 0 }
		| (("ab", False)) { 1 }
		| (("a", True) ? False) { 2 }
		| (("a" <> _, _)) { 3 }
		| ((_, False)) { 4 }
		| ((_: "a" | "b", False)) { 5 }
		| (("a", True)) { 6 }
		| (("a", False)) { 7 }
}
