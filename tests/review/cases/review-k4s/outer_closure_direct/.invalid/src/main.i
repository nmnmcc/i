// old-syntax-check: E0105
S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }
D = ()
test = <N>() <- (s: S(N)) { provided: D = D(() { use() })() }
