outer(A) = (x: A) {
	inner(A1) = (y: A1, z: A) { (y, z) }
	inner(42, x)
}


main = () {
	assert(outer("s") == (42, "s"))
	Nil
}
