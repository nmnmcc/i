// spec: 7.1, 7.2
// expect: compile-error E0501
main = () {
	(name = _) = (name = "Ada", age = 36)
	()
}
