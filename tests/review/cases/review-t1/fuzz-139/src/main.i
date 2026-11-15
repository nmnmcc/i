f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : String - "ab" - "b", False)) { 0 }
		| ((_ : String - "ab" - "b", _)) { 1 }
		| ((_ : String - "ab" - "b", False)) { 2 }
		| (("a" <> _, True)) { 3 }
}
