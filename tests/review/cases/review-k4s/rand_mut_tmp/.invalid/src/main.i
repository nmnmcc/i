// old-syntax-check: E0101
S(N) = (() <- N -> Nil, (N) -> Nil)
T(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
tuse = <N>() <- (s: T(N)) {
	T(f, _) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }
D = ()
f2 = <N>(b: Bool) <- (s: S(N)) -> Nil {
	b |>
		(True) {
            <- (D)
			run(() {
				h0 = () { f0(False) }
				h0()
			}) <- (D)
		}
		| (False) { tuse() }
}
f1 = <N>(b: Bool) <- (t: T(N)) -> Nil {
	b |>
		(True) {
			run(() {
				h0 = () { f2(False) }
				h0()
			})
		}
		| (False) { use() }
}
f0 = <N>(b: Bool) <- (s: S(N), t: T(N)) {
	b |>
		(True) { run(() { f1(False) }) }
		| (False) {
			use()
			tuse()
		}
}
