S(N) = (f = () <- N -> Int)
use = <N>() <- (s : S(N)) {
	S(f = f, ..) = s
	f()
}
wrap : <N>(() <- (S(N), ..N) -> Int, S(N)) <- N -> Int = (body, svc) {
	_syn3_0 : S(N) = svc
	body() <- (_syn3_0)
}
