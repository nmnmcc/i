S(N) = (f = () <- N -> Int)
use = <N>() <- (s : S(N)) {
	S(f = f, ..) = s
	f()
}
wrap = <N>(body : () <- (S(N), ..N) -> Int, svc : S(N)) {
	_syn3_0 : S(N) = svc
	body() <- (_syn3_0)
}
