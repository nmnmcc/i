(
	(Io) = i/io
)
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
tell = (x: Int) {
	x |> (0) { "zero" } | (1) { "one" } | (2) { "two" } | (_) { "many" }
}

main = () <- (io: Io) {
	io.print(ua(1)) <- (
		B((d: Int) {
			io.print("in B")
			r = ua(d - 1) + 1
			io.print(tell(r))
			r
		}),
		A((d: Int) {
			io.print("in A")
			d |>
				(0) { "str" }
				| (_) {
					ub(d)
					"str"
				}
		}),
	)
}
