S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) -> Nil {
	S(f, _) = s
	f()
}
run = <N>(body : () <- N -> Nil) { body() }
f = <N>() <- (s : S(N)) { run(() { use() }) }
