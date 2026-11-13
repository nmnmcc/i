f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "a", False)) { 0 }
		| (("a" <> _, _)) { 1 }
		| ((_ : String - "ab" - "b", False)) { 2 }
		| ((_ : String - "ab" - "b", _) ? False) { 3 }
		| (("a", False)) { 4 }
		| (("b", True) ? False) { 5 }
		| ((_ : { "a" | "b" }, _)) { 6 }
		| ((_ : { "a" | "b" }, True)) { 7 }
}
