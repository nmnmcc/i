// spec: 4.9
// expect: compile-error E0541
(
	main
)
f = <K>(r: Record, key: K) -> Int {
	h = (s: (k = Int)) { s }
	0
}
g = (key: String) { f((), key) }

main = () { Nil }
