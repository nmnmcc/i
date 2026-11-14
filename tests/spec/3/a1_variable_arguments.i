// spec: 3, 4.7
// expect: compile-error E0230
(
	main
)
bad = <A>(x : A(Int)) { x }

main = () {
	assert(True)
	Nil
}
