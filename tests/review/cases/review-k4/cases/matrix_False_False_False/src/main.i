S(N) = (() <- N -> Nil)
use = <N>() <- (s: S(N)) {
	S(f) = s
	f()
}
run = (body) { body() }
f = <N>() <- (s: S(N)) { run(() { use() }) }

main = () {
	_syn3_0: S(Never) = S(() { Nil })
	f() <- (_syn3_0)
}
