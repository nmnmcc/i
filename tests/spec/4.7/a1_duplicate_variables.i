// spec: 4.7
// expect: compile-error E0226
(
	main
)
pair = <A, A>(x : A) { (x, x) }

main = () {
	assert(True)
	Nil
}
