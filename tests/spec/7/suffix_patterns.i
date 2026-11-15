// spec: 7.1, 4.8
// expect: pass
tail = (s : String) -> String {
	s |>
		("a" <> r <> ".txt") { "both:\(r)" }
		| (r <> ".txt") { "txt:\(r)" }
		| (_) { "none" }
}
main = () {
	assert(tail("file.txt") == "txt:file")
	assert(tail("ab.txt") == "both:b")
	assert(tail("a.txt") == "both:")
	assert(tail("x") == "none")
	assert({ "😀é" |> (r <> "é") { r } | (_) { "bad" } } == "😀")
	Nil
}
