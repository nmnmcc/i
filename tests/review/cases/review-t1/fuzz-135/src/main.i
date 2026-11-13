f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		(("a" <> _, True)) { 0 }
		| ((_ : { "a" | "b" }, True) ? False) { 1 }
		| (("a", True)) { 2 }
		| ((_, False)) { 3 }
}
