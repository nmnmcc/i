S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }
forward = <N>() <- (s: S(N)) { run(() { use() }) }
