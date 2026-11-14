S(A, N) = (() <- N -> A)
use = <A, N>() <- (s : S(A, N)) {
	S(f) = s
	f()
}
example = () {
	_syn3_0 : S(String, Never) = S(() { "a" })
	_syn3_1 : S(Int, S(String, Never)) = S(() {
		use()
		1
	})
	use() <- (_syn3_0, _syn3_1)
}

main = () {
	example()
	Nil
}
