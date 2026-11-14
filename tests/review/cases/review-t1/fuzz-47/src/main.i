f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : { "a" | "b" }, False)) { 0 }
		| (("ab", True)) { 1 }
		| ((_ : String - "a", True)) { 2 }
		| (("b", True)) { 3 }
		| (("a" <> _, _) ? False) { 4 }
}
