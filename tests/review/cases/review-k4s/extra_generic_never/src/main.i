S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(f : () <- N -> A) { f() }
forward = <N>() <- (s : S(N)) { run(() { use() }) }
f = () {
	_syn3_0 : S(Never) = S(() { () }, (_ : Never) { () })
	forward() <- (_syn3_0)
}
