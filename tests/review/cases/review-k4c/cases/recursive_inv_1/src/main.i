S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <N>(f : () <- N -> Nil) { f() }
bad = <N>() {
	_syn3_0 : S(N) = S(() { use() }, (_ : N) { Nil })
	use() <- (_syn3_0)
}
