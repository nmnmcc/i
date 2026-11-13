S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
make = <N>(f: () <- N -> Nil) {
	result: () <- N -> Nil = () -> Nil {
		_syn3_0: S(N) = S(f, (_: N) { Nil })
		use() <- (_syn3_0)
	}
	result
}

main = () {
	f = make(() { Nil })
	f()
}
