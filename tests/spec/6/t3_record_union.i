// spec: 4.9
// expect: pass
(
	main
)
id = (x: (a = 1) | (b = "x")) -> (a = Int) | (b = String) { x }

main = () {
	assert(id((a = 1)) == (a = 1))
	assert(id((b = "x")) == (b = "x"))
	Nil
}
