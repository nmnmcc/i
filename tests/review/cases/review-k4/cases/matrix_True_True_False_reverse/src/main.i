S(N) = (() <- N -> (), (N) -> ())
f = <N>() <- (s : S(N)) { run(() { use() }) }
run = <N>(body : () <- N -> ()) { body() }
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}

main = () {
	_syn3_0 : S(Never) = S(() { () }, (_ : Never) { () })
	f() <- (_syn3_0)
}
