// spec: 6.12
// expect: compile-error E0507
// syn3-labels: keep
(
	main
)
classify = (0) { 0 } | (n) { n }

main = () {
	assert(classify(x = 1) == 1)
	Nil
}
