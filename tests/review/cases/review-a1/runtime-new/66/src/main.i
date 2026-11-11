f = (x) { x + 1 }

g = () {
	(x) { f(x) * 2 } |> {
		f -> f(1)
	}
}


main = () {
	assert(g() == 4)
	Nil
}
