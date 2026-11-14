// spec: 7.1, 7.2
// expect: compile-error E0510
main = () {
	(name = _, name = _) = (name = "Ada")
	Nil
}
