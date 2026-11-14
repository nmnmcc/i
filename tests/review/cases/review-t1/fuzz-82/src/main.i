f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("ab", True)) { 0 }
		| ((_, False)) { 1 }
		| (("ab", True)) { 2 }
		| (("a" <> _, True)) { 3 }
		| (("ab", _)) { 4 }
		| ((_ : String - "ab" - "b", True)) { 5 }
}
