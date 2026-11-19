S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(body : () <- N -> ()) { body() }
f = <N>() <- (s : S(N)) { run(() { use() }) }
