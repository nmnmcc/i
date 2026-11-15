// spec: 4.8
// expect: compile-error E0506
(
	main
)
User = (name = String)
main = () {
	get_name = (u) { u.name }
	assert(get_name(User(name = "Ada")) == "Ada")
}
