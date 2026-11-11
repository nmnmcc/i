outer(A1) = (x: A1) {
	inner(B) = (y: B, z: A1) { z }
	inner(42, x)
}


main = () {
	assert(outer("s") == "s")
	Nil
}
