// spec: 7.1, 7.2
// expect: compile-error E0201
Wrapper = (data = (name = String, age = Int))
main = () {
	Wrapper(data = (name = n, age = _)) = Wrapper(data = (name = "Ada", age = 36))
	_ = n
	data
}
