f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, True)) { 0 }
		| (("ab", True)) { 1 }
		| (("b", False)) { 2 }
		| (("ab", _)) { 3 }
		| (("ab", _)) { 4 }
		| ((_: String - "a", False)) { 5 }
		| (("b", False)) { 6 }
}
