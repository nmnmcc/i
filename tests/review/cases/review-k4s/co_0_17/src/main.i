S(N) = (() <- N -> Nil)
use = <N>() <- (s : S(N)) -> Nil {
	S(f) = s
	f()
}
run = <N>(body : () <- N -> Nil) { body() }
f = <N>() <- (s : S(N)) { run(() { use() }) }
