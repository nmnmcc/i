outer = <A>(x : A) {
	inner = (y : a, z : A) { (y, z) }
	inner(42, x)
}

main = () {
	assert(outer("s") == (42, "s"))
	Nil
}
