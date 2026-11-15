f = (n : Int) { n |> (0) { 42 } | (_) { 7 }() }

main = () {
	assert(f(0) == 42 && f(1) == 7)
	Nil
}
