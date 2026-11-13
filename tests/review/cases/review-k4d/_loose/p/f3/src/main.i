A(T, N) = ((Int) <- N -> T)
B(N) = ((Int) <- N -> Int)
ua = <T, N>(d: Int) <- (s: A(T, N)) {
	A(f) = s
	f(d)
}
ub = <N>(d: Int) <- (s: B(N)) {
	B(f) = s
	f(d)
}
bad = () {
	ua(1) <- (
		B((d: Int) { ua(d - 1) + 1 }),
		A((d: Int) {
			d |>
				(0) { "str" }
				| (_) {
					ub(d)
					"str"
				}
		}),
	)
}

main = () {
	bad()
	Nil
}
