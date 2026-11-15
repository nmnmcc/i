f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", _)) { 0 }
		| ((_ : String - "ab" - "b", _) ? False) { 1 }
		| (("b", True)) { 2 }
		| (("a" <> _, True) ? False) { 3 }
		| ((_ : String - "ab" - "b", _) ? False) { 4 }
		| (("b", False)) { 5 }
		| (("ab", _)) { 6 }
		| (("ab", False)) { 7 }
}
