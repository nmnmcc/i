S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <N>(body: () <- N -> Nil) { body() }
helper = <N>() { run(() { use() }) }
