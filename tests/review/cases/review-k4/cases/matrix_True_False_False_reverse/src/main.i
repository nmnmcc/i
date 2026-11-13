S(N) = (() <- N -> Nil, (N) -> Nil)
f = <N>() <- (s: S(N)) { run(() { use() }) }
run = (body) { body() }
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}

main = () {
	_syn3_0: S(Never) = S(() { Nil }, (_: Never) { Nil })
	f() <- (_syn3_0)
}
