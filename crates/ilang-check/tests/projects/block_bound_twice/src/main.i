// A block is one scope: a name is bound there once (§5).
f = () {
	x = 1
	x = 2
	x
}

g = () {
	n : Int = 1
	(n, m) = (2, 3)
	n + m
}
