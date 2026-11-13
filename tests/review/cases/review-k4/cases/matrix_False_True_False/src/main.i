S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = <N>(body: () <- N -> Nil) { body() }
f = <N>() <- (s: S(N)) { run(() { use() }) }

main = () {
	_syn3_0: S(Never) = S(() { Nil })
	f() <- (_syn3_0)
}
