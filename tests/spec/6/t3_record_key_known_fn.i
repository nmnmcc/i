// spec: 4.8, 4.9
// expect: pass
(
	main
)
Box(A) = (item = A)
apply = <K>(b: Box((k = Int)), key: K, f: (Box((k = Int))) -> Int) -> Int {
	f(b)
}

main = () {
	assert(apply(Box(item = (a = 1)), "a", (x) { x.item.a }) == 1)
	Nil
}
