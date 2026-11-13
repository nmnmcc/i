f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", _)) { 0 }
		| (("b", True) ? False) { 1 }
		| (("ab", True)) { 2 }
		| ((_: { "a" | "b" }, False)) { 3 }
		| ((_: { "a" | "b" }, True)) { 4 }
		| ((_: { "a" | "b" }, False) ? False) { 5 }
}
