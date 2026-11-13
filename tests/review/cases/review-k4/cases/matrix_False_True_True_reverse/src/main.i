S(N) = (() <- N -> Nil)
f = <N>() <- (s: S(N)) -> Nil { run(() { use() }) }
run = <N>(body: () <- N -> Nil) { body() }
use = <N>() <- (s: S(N)) -> Nil {
	S(f) = s
	f()
}

main = () {
	_syn3_0: S(Never) = S(() { Nil })
	f() <- (_syn3_0)
}
