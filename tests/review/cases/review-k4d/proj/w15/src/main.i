S(N) = (f = () <- N -> Int, m = (N) -> Nil)
use = <N>() <- (s: S(N)) {
	S(f = f, ..) = s
	f()
}
wrap = <N>(svc: S(N)) {
	_syn3_0: S(N) = svc
	use() <- (_syn3_0)
}
