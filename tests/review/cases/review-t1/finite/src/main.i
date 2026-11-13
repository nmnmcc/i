f = (0) { "zero" } | (1) { "one" }
g = (s : { "a" | "ab" }) { s |> ("a") { 0 } | ("a" <> _) { 1 } }

main = () {
	assert(f(1) == "one")
	assert(g("ab") == 1)
	Nil
}
