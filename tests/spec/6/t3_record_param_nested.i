// spec: 4.9
// expect: compile-error E0543
(
	main
)
f = <K>(r : (k = Int), key : K) -> Int {
	nested = (another : K) { another }
	1
}

main = () { Nil }
