f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |>
		((_ : { "a" | "b" }, _)) { 0 }
		| ((_ : String - "ab" - "b", True)) { 1 }
		| ((_ : String - "ab" - "b", _)) { 2 }
}
