outer = <A>(x : A) {
	inner = <A>(y : A) { y }
	(x, inner(42), inner("s"))
}

main = () {
	assert(outer(True) == (True, 42, "s"))
	Nil
}
