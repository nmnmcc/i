// spec: 7.1, 7.2
// expect: compile-error E0217
main = () {
	(name = x, age = x) = (name = "Ada", age = 36)
	()
}
