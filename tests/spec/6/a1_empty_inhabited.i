// spec: 4.8, 6.12
// expect: compile-error E0501
(
	main
)

main = () {
	core.never(1)
	Nil
}
