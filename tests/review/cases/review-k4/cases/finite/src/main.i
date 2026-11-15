S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <N>(body : () <- N -> Nil) { body() }
forward = <N>() <- (s : S(N)) { run(() { use() }) }
example = () {
	_syn3_0 : S(Never) = S(() { Nil }, (_ : Never) { Nil })
	forward() <- (_syn3_0)
}

main = () { example() }
