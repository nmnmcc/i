f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "ab" - "b", True)) { 0 }
		| (("a", False)) { 1 }
		| (("a" <> _, True)) { 2 }
}
