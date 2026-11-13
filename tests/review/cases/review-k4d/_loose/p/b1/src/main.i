S(A, N) = (() <- N -> A)
use = <A, N>() <- (s: S(A, N)) {
	S(f) = s
	f()
}
example = () {
	use() <- (
		S(() { "a" }),
		S(() {
			use()
			1
		}),
	)
}

main = () {
	example()
	Nil
}
