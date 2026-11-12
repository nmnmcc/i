f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |> (("b", True) ? False) { 0 } | ((_, False)) { 1 }
}
