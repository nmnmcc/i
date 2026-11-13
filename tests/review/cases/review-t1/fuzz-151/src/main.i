f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |> ((_: String - "a", True) ? False) { 0 } | ((_: "a" | "b", False)) { 1 }
}
