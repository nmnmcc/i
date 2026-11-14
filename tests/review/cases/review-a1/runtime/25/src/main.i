f = (v : (Int) -> Int) { v |> (g : (Int) -> Int ? True) { g(41) } | (_) { 0 } }

main = () {
	assert(f((x) { x + 1 }) == 42)
	Nil
}
