S(N) = (() <- N -> ())
use = <N>() <- (s : S(N)) {
	S(f) = s
	f()
}
run = <N>(f : () <- N -> ()) { f() }
bad = <N>() {
	local = () {
		_syn3_0 : S(N) = S(() { use() })
		use() <- (_syn3_0)
	}
	local()
}
