S(N) = (() <- N -> Nil)
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(f : () <- N -> Nil) { f() }
bad = <N>() {
	_syn3_0 : S(N) = S(() { use() })
	run(() { use() }) <- (_syn3_0)
}
