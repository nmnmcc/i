f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "a", True)) { 0 }
		| (("b", True)) { 1 }
		| ((_ : String - "a", _) ? False) { 2 }
}
