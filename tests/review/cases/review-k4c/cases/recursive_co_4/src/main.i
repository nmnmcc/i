S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <N>(f: () <- N -> Nil) { f() }
bad = <N>() { run(() { use() }) <- (S(() { use() })) }
