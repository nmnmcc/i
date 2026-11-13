A(N) = (() <- N -> Int)
B(N) = (() <- N -> Int)
ua = <N>() <- (s: A(N)) {
	A(f) = s
	f()
}
ub = <N>() <- (s: B(N)) {
	B(f) = s
	f()
}
(Io) = i / io

main = () <- (io: Io) {
	_syn3_0: B(Never) = B(() { 1 })
	_syn3_1: A(B(Never)) = A(() { ub() + 1 })
	io.print(
		int_text(ua()),
	) <- (_syn3_0, _syn3_1)
}
int_text = (n: Int) { n |> (2) { "two" } | (_) { "other" } }
