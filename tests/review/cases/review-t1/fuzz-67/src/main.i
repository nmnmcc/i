f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : { "a" | "b" }, False) ? False) { 0 }
		| (("a", True)) { 1 }
		| (("a", True)) { 2 }
		| ((_ : String - "a", True)) { 3 }
}
