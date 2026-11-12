S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(f: () <- N -> A) { f() }
forward: <N>() <- (S(N), ..N) -> Nil = () <- (s: S(N)) { run(() { use() }) }
