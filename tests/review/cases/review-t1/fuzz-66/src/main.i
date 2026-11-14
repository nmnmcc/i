f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "ab" - "b", False)) { 0 }
		| ((_ : String - "ab" - "b", False)) { 1 }
		| (("a" <> _, False) ? False) { 2 }
		| ((_ : String - "ab" - "b", _)) { 3 }
		| (("a" <> _, True)) { 4 }
		| ((_, True)) { 5 }
		| ((_ : String - "ab" - "b", True)) { 6 }
}
