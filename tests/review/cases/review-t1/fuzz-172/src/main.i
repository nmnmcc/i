f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "ab" - "b", True)) { 0 }
		| ((_, True)) { 1 }
		| ((_ : String - "ab" - "b", False)) { 2 }
		| (("a" <> _, _) ? False) { 3 }
		| (("b", False)) { 4 }
}
