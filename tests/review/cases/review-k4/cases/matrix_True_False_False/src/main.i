S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = (body) { body() }
f = <N>() <- (s : S(N)) { run(() { use() }) }

main = () {
	_syn3_0 : S(Never) = S(() { Nil }, (_ : Never) { Nil })
	f() <- (_syn3_0)
}
