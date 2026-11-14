// spec: 1.7
// expect: pass
(
	main
)
Pair = (Int, Int)
main = () {
	mk = Pair
	(b, c) = (1, 2)
	assert(mk(b, c) == Pair(1, 2))
}
