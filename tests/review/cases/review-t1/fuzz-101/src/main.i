f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", True)) { 0 }
		| ((_ : String - "a", True)) { 1 }
		| (("ab", False)) { 2 }
		| ((_ : { "a" | "b" }, False) ? False) { 3 }
		| ((_ : String - "ab" - "b", False) ? False) { 4 }
		| ((_ : { "a" | "b" }, _)) { 5 }
		| ((_ : String - "a", False) ? False) { 6 }
		| (("a" <> _, False)) { 7 }
		| ((_ : String - "a", True)) { 8 }
}
