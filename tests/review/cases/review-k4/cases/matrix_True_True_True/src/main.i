S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) -> Nil {
	S(f, _) = s
	f()
}
run = <N>(body : () <- N -> Nil) { body() }
f = <N>() <- (s : S(N)) -> Nil { run(() { use() }) }

main = () {
	_syn3_0 : S(Never) = S(() { Nil }, (_ : Never) { Nil })
	f() <- (_syn3_0)
}
