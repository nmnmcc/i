S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }
D = ()
test = <N>() <- (s: S(N)) -> Nil { run(() { use() }) <- (D) }
