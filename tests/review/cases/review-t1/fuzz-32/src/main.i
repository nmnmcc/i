f = (x : ("a" | "ab" | "b" | "c" | "", Bool)) {
	x |> (("a", _)) { 0 } | ((_ : String - "a", True)) { 1 }
}
