f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", False)) { 0 }
		| (("b", True)) { 1 }
		| ((_ : String - "a", True)) { 2 }
		| (("a", True) ? False) { 3 }
		| (("a" <> _, False)) { 4 }
		| ((_ : String - "a", _)) { 5 }
}
