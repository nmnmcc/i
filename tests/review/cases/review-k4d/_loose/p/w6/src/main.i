S(N) = (f = () <- N -> Int)
use = <N>() <- (s : S(N)) {
	S(f = f, ..) = s
	f()
}
wrap = <N>(svc : S(N)) -> Int {
	_syn3_0 : S(N) = svc
	use() <- (_syn3_0)
}
