S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
run = <A, N>(f: () <- N -> A) { f() }
D = ()
forward = <N>() <- (s: S(N)) { run(() { use() }) }
f = () {
	_syn3_0: S(D) = S(() <- (d: D) { Nil }, (_: D) { Nil })
	forward() <- (D, _syn3_0)
}
