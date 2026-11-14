f = (p : (String, Int)) {
	p |> ((_ : String - "ab", _)) { 1 } | (("a" <> _, n)) { n + 1 } | (_) { 0 }
}

main = () {
	assert(f(("ab", 5)) == 6)
	Nil
}
