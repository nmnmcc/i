f = (v : Int) {
	case
	v
	(x : Int) { x }
}

main = () {
	assert(f(42) == 42)
	Nil
}
