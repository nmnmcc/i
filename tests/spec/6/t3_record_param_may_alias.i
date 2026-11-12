// spec: 4.9
// expect: compile-error E0544
(
	main
)
f = <K>(r: (k = Int), key: K) -> Int { r.name }

main = () { Nil }
