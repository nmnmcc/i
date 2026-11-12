// spec: 4.8, 4.9
// expect: compile-error E0512
(
	main
)

main = () {
	value = (age = 1)
	value.name
}
