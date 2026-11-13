(
	list = std/list
)
Box(A) = (value = A)
rec = (n, acc) { n |> (0) { acc } | (_) { rec(n - 1, acc + 1) } }

main = () {
	assert(rec(4, 0) == 4)
	assert(
		list.fold([1, 2, 3], 0, (n, x) {
			{ n == 1 } |> (True) { n + x } | (False) { n + 1 }
		}) ==
			4,
	)
	assert(
		list.fold([1, 2], (0, ""), (p, x) { (p.0 + x, p.1 <> "a") }).1 == "aa",
	)
	assert(
		list.fold([1, 2], Box(value = 0), (p, x) {
			Box(value = p.value + x)
		}).value ==
			3,
	)
	Nil
}
