f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("b", False)) { 0 }
		| ((_ : String - "a", False)) { 1 }
		| ((_ : String - "ab" - "b", False)) { 2 }
		| ((_ : String - "a", False)) { 3 }
}
