S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <N>(f : () <- N -> ()) { f() }
bad = <N>() {
	local = () {
		_syn3_0 : S(N) = S(() { use() }, (_ : N) { () })
		use() <- (_syn3_0)
	}
	local()
}
