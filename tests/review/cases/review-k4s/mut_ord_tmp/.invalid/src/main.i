// old-syntax-check: E0101
S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(f: () <- N -> A) { f() }
D = ()
second = <N>(b: Bool) <- (s: S(N)) -> Nil {
	b |>
		(True) {
			run(h1: _ = () {
				h0 = () { first(False) }
				h0()
			} )
			h1()
		}
		| (False) { use() }
}
first = <N>(b: Bool) <- (s: S(N)) -> Nil {
	b |>
		(True) {
			h1 = () {
				h0 = () { second(False) }
				h0()
			}
			h1()
		}
		| (False) { use() }
}
