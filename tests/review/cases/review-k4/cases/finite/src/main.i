S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <N>(body : () <- N -> ()) { body() }
forward = <N>() <- (s : S(N)) { run(() { use() }) }
example = () {
	_syn3_0 : S(Never) = S(() { () }, (_ : Never) { () })
	forward() <- (_syn3_0)
}

main = () { example() }
