f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |> (("a", True)) { 0 } | (("b", True) ? False) { 1 }
}
