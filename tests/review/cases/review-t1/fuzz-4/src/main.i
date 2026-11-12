f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |> (("ab", True)) { 0 } | ((_: String - "a", True)) { 1 }
}
