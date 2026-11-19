S(N) = (() <- N -> (), (N) -> ())
use = <N>() <- (s : S(N)) {
	S(f, _) = s
	f()
}
run = <N>(body : () <- N -> ()) { body() }
example = () {
	_syn3_0 : S(Never) = S(() { () }, (_ : Never) { () })
	run(() { use() }) <- (_syn3_0)
}

main = () { example() }
