// spec: 4.9
// expect: compile-error E0543
(
	main
)
f = <K>(r: (k = Int), key: K) {
	x: key = key
	x
}

main = () { Nil }
