f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "ab" - "b", True)) { 0 }
		| ((_, _) ? False) { 1 }
		| ((_ : String - "a", False)) { 2 }
		| ((_ : String - "ab" - "b", True)) { 3 }
		| (("a" <> _, False)) { 4 }
		| ((_ : { "a" | "b" }, _)) { 5 }
		| ((_ : String - "a", False)) { 6 }
}
