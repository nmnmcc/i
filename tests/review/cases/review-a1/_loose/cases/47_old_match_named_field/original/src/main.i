User = (name = String)
f = (n : Int) {
	case
	n
	(0) { User(name = "yes") } | (_) { User(name = "no") }.name
}
