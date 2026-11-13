S(N) = (() <- N -> Nil, (N) -> Nil)
a = <N>() {
	_syn3_0: S(N) = S(() { b() }, (_: N) { Nil })
	use() <- (_syn3_0)
}
b = () { a() }
use = <N>() <- (s: S(N)) {
	S(f, _) = s
	f()
}
