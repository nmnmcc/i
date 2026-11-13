f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", _)) { 0 }
		| ((_ : String - "a", True)) { 1 }
		| ((_ : String - "ab" - "b", _) ? False) { 2 }
		| ((_ : { "a" | "b" }, True)) { 3 }
		| (("b", False)) { 4 }
}
