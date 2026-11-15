f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", True)) { 0 }
		| (("ab", False)) { 1 }
		| ((_ : String - "a", False)) { 2 }
		| (("ab", _)) { 3 }
		| (("a" <> _, _)) { 4 }
		| ((_ : String - "ab" - "b", False)) { 5 }
}
