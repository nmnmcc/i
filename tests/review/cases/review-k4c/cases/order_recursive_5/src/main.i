b = () { a() }
S(N) = (() <- N -> Nil, (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
a = <N>() {
	_syn3_0: S(N) = S(() { b() }, (_: N) { Nil })
	use() <- (_syn3_0)
}
