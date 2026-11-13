f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_: String - "a", False)) { 0 }
		| ((_, True)) { 1 }
		| ((_: String - "a", _)) { 2 }
		| ((_: { "a" | "b" }, True)) { 3 }
}
