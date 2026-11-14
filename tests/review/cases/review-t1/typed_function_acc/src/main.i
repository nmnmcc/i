(
	list = std/list
)

main = () {
	assert(
		list.fold([1, 2, 3], 0, (n : Int, x : Int) {
			{ n == 3 } |> (True) { n } | (False) { n + x }
		}) ==
			3,
	)
	Nil
}
