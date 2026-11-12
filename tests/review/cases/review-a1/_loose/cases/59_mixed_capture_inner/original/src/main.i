outer = <A>(x: A) {
	inner = (y: a, z: A) { (y, z) }
	inner(42, x)
}
