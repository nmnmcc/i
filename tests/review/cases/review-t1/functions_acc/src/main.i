(
	list = std/list
)
Box(A) = (get = () -> A)

main = () {
	b = list.fold([1, 2, 3], Box(get = () { 0 }), (acc, x) {
		Box(get = () { acc.get() + x })
	})
	assert(b.get() == 6)
	Nil
}
