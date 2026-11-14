(
	fail = i/fail
)
f = (b : Bool) { b |> (True) { fail.raise("a") } | (False) { fail.raise("b") } }
g = (b : Bool) {
	fail.catch(() { f(b) }) |>
		(Error("a")) { 1 }
		| (Error(e)) {
			x : "b" = e
			2
		}
		| (Ok(_)) { 0 }
}

main = () {
	assert(g(False) == 2)
	Nil
}
