User = (name = String)
f = (n : Int) {
	case
	n
	(0) { User(name = "yes") } | (_) { User(name = "no") }.name
}

main = () {
	assert(f(0) == "yes" && f(1) == "no")
	Nil
}
