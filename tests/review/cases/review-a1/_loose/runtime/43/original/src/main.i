f = (n: Int) {
	case
	n > 0
	(True) { 42 } | (False) { 7 }
}

main = () {
	assert(f(1) == 42 && f(0) == 7)
	Nil
}
