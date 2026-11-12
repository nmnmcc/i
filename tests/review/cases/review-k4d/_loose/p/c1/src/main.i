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
	io.print(
		int_text(ua()),
	) <- (B(() { 1 }), A(() { ub() + 1 }))
}
int_text = (n: Int) { n |> (2) { "two" } | (_) { "other" } }
