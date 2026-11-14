S(N) = (() <- N -> Nil)
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(f : () <- N -> Nil) { f() }
bad = <N>() {
	local = () {
		_syn3_0 : S(N) = S(() { use() })
		use() <- (_syn3_0)
	}
	local()
}
