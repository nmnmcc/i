f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_, True)) { 0 }
		| (("ab", True)) { 1 }
		| ((_ : String - "a", False)) { 2 }
		| ((_ : String - "ab" - "b", True)) { 3 }
}
