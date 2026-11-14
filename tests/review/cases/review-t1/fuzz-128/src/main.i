f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a", True)) { 0 }
		| ((_ : String - "a", False) ? False) { 1 }
		| ((_ : { "a" | "b" }, False)) { 2 }
}
