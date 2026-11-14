f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, False)) { 0 }
		| (("ab", False)) { 1 }
		| (("a" <> _, False)) { 2 }
		| (("b", _) ? False) { 3 }
		| ((_ : String - "a", _)) { 4 }
		| (("b", _)) { 5 }
}
