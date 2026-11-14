A(T, N) = ((Int) <- N -> T)
B(N) = ((Int) <- N -> Int)
ua = <T, N>(d : Int) <- (s : A(T, N)) {
	A(f) = s
	f(d)
}
ub = <N>(d : Int) <- (s : B(N)) {
	B(f) = s
	f(d)
}
bad = () {
	_syn3_0 : B(A(Int, Never)) = B((d : Int) { ua(d - 1) + 1 })
	_syn3_1 : A(String, B(A(Int, Never))) = A((d : Int) {
		d |>
			(0) { "str" }
			| (_) {
				ub(d)
				"str"
			}
	})
	ua(1) <- (_syn3_0, _syn3_1)
}

main = () {
	bad()
	Nil
}
