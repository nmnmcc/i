f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |> ((_: String - "a", True)) { 0 } | (("a", False) ? False) { 1 }
}
