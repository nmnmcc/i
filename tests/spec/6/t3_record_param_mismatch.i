// spec: 4.9
// expect: compile-error E0512
(
	main
)
f = <K>(r : (k = Int), key : K) -> Int { 1 }

main = () { f((name = 9), "other") }
