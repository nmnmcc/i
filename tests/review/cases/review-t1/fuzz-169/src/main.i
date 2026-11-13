f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("ab", True)) { 0 }
		| (("a" <> _, True)) { 1 }
		| ((_: String - "a", _)) { 2 }
}
