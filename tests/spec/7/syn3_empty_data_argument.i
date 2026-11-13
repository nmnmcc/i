// spec: 6.4, 7.1
// expect: pass
(
	main
)
// `()` is the empty data, a value and a pattern of the type `()` (§7.1).
f = (()) { 1 }

main = () {
	assert(f(()) == 1)
	Nil
}
