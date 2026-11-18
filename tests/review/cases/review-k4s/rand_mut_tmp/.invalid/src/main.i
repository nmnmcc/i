// old-syntax-check: E0101,E0101,E0101,E0101
S(N) = (() <- N -> (), (N) -> ())
T(N) = (() <- N -> (), (N) -> ())
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
f2 = <N>(b: Bool) <- (s: S(N)) -> () {
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
f1 = <N>(b: Bool) <- (t: T(N)) -> () {
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
