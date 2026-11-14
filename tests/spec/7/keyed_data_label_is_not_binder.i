// spec: 7.1
// expect: compile-error E0201
main = () {
	(name = n) = (name = "Ada")
	_ = n
	name
}
